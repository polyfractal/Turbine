

use sync::Arc;
use std::sync::atomics::{AtomicInt, SeqCst, Release, Acquire};
use waitstrategy::{WaitStrategy};
use paddedatomics::Padded64;
use ringbuffer::{RingBuffer, Slot};
use std::fmt::{Show};
use std::io::timer;


pub struct EventProcessor<T> {
	graph: Arc<Vec<Vec<uint>>>,
	cursors: Arc<Vec<Padded64>>,
	token: uint,
	ring: Arc<RingBuffer<T>>
}


impl<T: Slot + Show> EventProcessor<T> {
	pub fn new(ring: Arc<RingBuffer<T>>, graph: Arc<Vec<Vec<uint>>>, cursors: Arc<Vec<Padded64>>, token: uint) -> EventProcessor<T> {
		EventProcessor::<T> {
			graph: graph,
			cursors: cursors,
			token: token,
			ring: ring
		}
	}


	pub fn start<W: WaitStrategy>(&self, f: |data: &[T]| -> Result<(),()>) {
		let capacity = self.ring.get_capacity();

		let wait_strategy: W = WaitStrategy::new(capacity);

		let dep_eps = self.graph.get(self.token);
		let mut deps: Vec<&Padded64> = Vec::with_capacity(dep_eps.len());
		for ep in dep_eps.iter() {
			deps.push((*self.cursors).get(*ep));
		}
		drop(dep_eps);

		let cursor = (*self.cursors).get(self.token);

		let mask: uint  = capacity - 1;

		let mut rollover = (false, 0);

		loop {
			let c = cursor.load();
			error!("BusyWait:: Current: {}", c);
			let current = match c {
				-1 => 0,
				v @ _ => (v as uint & mask) as int
			};

			let mut available: uint = wait_strategy.wait_for(current, &deps);

			if available >= capacity {
				rollover = (true, available - capacity);
				available = capacity;
			}
			//let to = (available - next);

			error!("BusyWait::  current: {}, available: {}", current, available);

			// This is safe because the Producer task cannot invalidate these slots
			// before we increment our cursor.  Since the slice is borrowed out, we
			// know it will be returned after the function call ends.  The slice will
			// be dropped after the unsafe block, and *then* we increment our cursor
			let status = unsafe {
				let data: &[T] = self.ring.get(current as uint, available);
				f(data)
			};

			if rollover.val0() == true {
				error!("!!!!  ROLLOVER");
				let status = unsafe {
					let data: &[T] = self.ring.get(0, rollover.val1());
					f(data)
				};

				rollover = (false, 0);
			}

			// Advance our location
			//available += 1;
			error!("BusyWait:: advancing to: {}", available);
			cursor.store(available as int);

			//timer::sleep(1000);
			match status {
				Err(_) => break,
				Ok(_) => {}
			};

		}
		error!("BusyWait::end");
	}

}
