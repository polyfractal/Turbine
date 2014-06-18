

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
		//error!("					Token: {}", self.token);
		let cursor = (*self.cursors).get(self.token + 1);

		let mask: uint  = capacity - 1;
		let mut rollover = (false, 0);

		let mut internal_cursor = cursor.load();

		loop {
			let c = internal_cursor;	//cursor.load();
			//error!("              Current: {}, waiting on: {}", c, c);

			let mut available: uint = wait_strategy.wait_for(c, &deps);

			let from = c as uint & mask;
			let mut to = available & mask;

			//error!("              from: {}, to: {}", from, to);
			if to < from {
				rollover = (true, to);
				to = capacity;
			}

			// This is safe because the Producer task cannot invalidate these slots
			// before we increment our cursor.  Since the slice is borrowed out, we
			// know it will be returned after the function call ends.  The slice will
			// be dropped after the unsafe block, and *then* we increment our cursor
			let status = unsafe {
				let data: &[T] = self.ring.get(from, to + 1);
				f(data)
			};

			if rollover.val0() == true {
				//error!("              ROLLOVER!");
				let status = unsafe {
					let data: &[T] = self.ring.get(0, rollover.val1());
					f(data)
				};
			}

			let adjusted_pos = self.increment(available as int, capacity as int);
			//error!("              available: {}, adjusted_pos: {}", available, adjusted_pos);
			internal_cursor = adjusted_pos;
			cursor.store(adjusted_pos);

			//timer::sleep(1000);
			match status {
				Err(_) => break,
				Ok(_) => {}
			};

		}
		//error!("BusyWait::end");
	}

	fn increment(&self, p: int, size: int) -> int {
		(p + 1) & ((2 * size) - 1)
	}

}
