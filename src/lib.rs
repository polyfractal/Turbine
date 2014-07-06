#![crate_id = "turbine"]

#![feature(phase)]
#![feature(macro_rules)]
#[phase(syntax, link)]

//#![deny(missing_doc)]





extern crate log;
extern crate sync;

extern crate time;

use sync::Arc;
use eventprocessor::EventProcessor;
use paddedatomics::Padded64;
use ringbuffer::{RingBuffer};
use std::cmp::{min, max};
use std::fmt;

pub use ringbuffer::Slot;
pub use waitstrategy::{WaitStrategy, BusyWait};

mod eventprocessor;
mod waitstrategy;
mod paddedatomics;
mod ringbuffer;


pub struct Turbine<T> {
	finalized: bool,
	epb: Vec<Option<Vec<uint>>>,
	graph: Arc<Vec<Vec<uint>>>,
	cursors: Arc<Vec<Padded64>>,
	ring: Arc<RingBuffer<T>>,
	start: int,
	end: int,
	size: int,
	mask: int,
	until: int
}

impl<T: Slot + Send> Turbine<T> {
	pub fn new(ring_size: uint) -> Turbine<T> {
		let mut epb = Vec::with_capacity(8);

		Turbine::<T> {
			finalized: false,
			epb: epb,
			graph: Arc::new(vec![]),
			cursors: Arc::new(vec![]),
			ring: Arc::new(RingBuffer::<T>::new(ring_size)),
			start: 0,
			end: 0,
			size: ring_size as int,
			mask: (ring_size - 1) as int,
			until: ring_size as int
		}
	}

	pub fn ep_new(&mut self) -> Result<uint, ()> {
		match self.finalized {
			true => Err(()),
			false => {
					self.epb.push(None);
					Ok(self.epb.len() - 1)
			}
		}
	}

	pub fn ep_depends(&mut self, epb_index: uint, dep: uint) -> Result<(),()> {
		if self.finalized == true {
			return Err(());
		}

		let epb = self.epb.get_mut(epb_index);
		match *epb {
			Some(ref mut v) => v.push(dep),
			None => {
				*epb = Some(vec![dep])
			}
		};
		Ok(())
	}

	pub fn ep_finalize(&mut self, token: uint) -> EventProcessor<T> {
		if self.finalized == false {
			self.finalize_graph();
		}

		EventProcessor::<T>::new(self.ring.clone(), self.graph.clone(), self.cursors.clone(), token)
	}

	fn finalize_graph(&mut self) {
		let mut eps: Vec<Vec<uint>> = Vec::with_capacity(self.epb.len());
		let mut cursors: Vec<Padded64> = Vec::with_capacity(self.epb.len() + 1);

		// Add the root cursor
		cursors.push(Padded64::new(0));

		for node in self.epb.iter() {
			let deps: Vec<uint> = match *node {
				Some(ref v) => v.clone(),
				None => vec![0]
			};
			eps.push(deps);
			cursors.push(Padded64::new(0));
		}

		self.graph = Arc::new(eps);
		self.cursors = Arc::new(cursors);
		drop(&self.epb);
		self.finalized = true;
	}

	pub fn write(&mut self, data: T) {

		// Busy spin
		loop {
			error!("Spin...");
			match self.can_write() {
				true => break,
				false => {}
			}
		}

		let write_pos = self.end & (self.mask);
		error!("end is {}, writing to {}", self.end, write_pos);
		unsafe {
			self.ring.write(write_pos as uint, data);
		}



		let adjusted_pos = self.increment(self.end);
		error!("adjusted_pos is {}", adjusted_pos);
		self.end = adjusted_pos;
		self.cursors.get(0).store(adjusted_pos);
		error!("Write complete.")

	}

	fn can_write(&mut self) -> bool {
		//return cb->end == (cb->start ^ cb->size);
		let mut writeable = false;

		error!("Until is: {}", self.until);
		if self.until == self.end {
			error!("*");
			let mut closest = (self.size * 2) + 1;

			for v in self.cursors.iter().skip(1) {
				let cursor_pos = v.load();
				if (self.end == (cursor_pos ^ self.size)) {
					//error!("Buffer full!");
					writeable = false;		// full ring buffer, same position but flipped parity bits
					closest = min(closest, cursor_pos);
				}
			}
			writeable = true;
			self.until = closest;
		} else {
			error!(".");
			writeable = true;
		}

		writeable
	}


	fn increment(&self, p: int) -> int {
		(p + 1) & ((2 * self.size) - 1)
	}




}


#[cfg(test)]
mod test {

	use Turbine;
	use Slot;
	use waitstrategy::BusyWait;
	use std::io::timer;
	use std::fmt;
	use std::task::{TaskBuilder};
	use std::sync::Future;
	use time::precise_time_ns;
	use std::comm::TryRecvError;

	//use TestSlot;

	struct TestSlot {
		pub value: int
	}

	impl Slot for TestSlot {
		fn new() -> TestSlot {
			TestSlot {
				value: -1	// Negative value here helps catch bugs since counts will be wrong
			}
		}
	}


	#[test]
	fn test_init() {
		let t: Turbine<TestSlot> = Turbine::new(1024);
	}

	#[test]
	fn test_create_epb() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new();
	}

	#[test]
	fn test_depends() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new().unwrap();
		let e2 = t.ep_new().unwrap();

		t.ep_depends(e2, e1);
	}

	#[test]
	fn test_many_depends() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new().unwrap();
		let e2 = t.ep_new().unwrap();
		let e3 = t.ep_new().unwrap();
		let e4 = t.ep_new().unwrap();
		let e5 = t.ep_new().unwrap();
		let e6 = t.ep_new().unwrap();

		/*
			Graph layout:

			e6 --> e1 <-- e2
						^      ^
						|      |
						+---- e3 <-- e4 <-- e5

		*/
		t.ep_depends(e2, e1);
		t.ep_depends(e5, e4);
		t.ep_depends(e3, e1);
		t.ep_depends(e4, e3);
		t.ep_depends(e3, e2);

		t.ep_finalize(e1);
		t.ep_finalize(e2);
		t.ep_finalize(e3);
		t.ep_finalize(e4);
		t.ep_finalize(e5);
		t.ep_finalize(e6);
	}

	#[test]
	fn test_finalize() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new();
		assert!(e1.is_ok() == true);

		let event_processor = t.ep_finalize(e1.unwrap());

		let e2 = t.ep_new();
		assert!(e2.is_err() == true);
	}

	#[test]
	fn test_double_finalize() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new();
		assert!(e1.is_ok() == true);

		let event_processor = t.ep_finalize(e1.unwrap());
		let event_processor2 = t.ep_finalize(e1.unwrap());

		let e2 = t.ep_new();
		assert!(e2.is_err() == true);
	}

	#[test]
	fn test_send_task() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new();
		assert!(e1.is_ok() == true);

		let e2 = t.ep_new();
		assert!(e2.is_ok() == true);

		t.ep_depends(e2.unwrap(), e1.unwrap());

		let ep1 = t.ep_finalize(e1.unwrap());
		let ep2 = t.ep_finalize(e2.unwrap());

		spawn(proc() {
			let a = ep1;
		});

		spawn(proc() {
			let b = ep2;
		});
	}

	#[test]
	fn test_write_one() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);

		assert!(t.end == 0);
		t.write(Slot::new());

		assert!(t.end == 1);
	}


	#[test]
	fn test_write_1024() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);

		assert!(t.end == 0);

		// fill the buffer but don't roll over
		for i in range(1, 1023) {
			t.write(Slot::new());

			assert!(t.end == i);
		}

	}


	#[test]
	fn test_write_ring_rollover() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);

		assert!(t.end == 0);

		//move our reader's cursor so we can rollover
		t.cursors.get(1).store(1);

		for i in range(1, 1025) {
			t.write(Slot::new());

			assert!(t.end == i);
		}
		t.write(Slot::new());
		assert!(t.end == 1025);
	}

	#[test]
	fn test_write_ring_double_rollover() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);

		assert!(t.end == 0);

		//move our reader's cursor so we can rollover
		t.cursors.get(1).store(1);

		for i in range(1i, 1025i) {
			t.write(Slot::new());

			assert!(t.end == i);
		}

		//move our reader's cursor so we can rollover again
		t.cursors.get(1).store(1025);
		for i in range(1i, 1025i) {
			t.write(Slot::new());
		}
		assert!(t.end == 0);
	}


	#[test]
	fn test_write_one_read_one() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);
		let (tx, rx): (Sender<int>, Receiver<int>) = channel();

		let mut future = Future::spawn(proc() {
			event_processor.start::<BusyWait>(|data: &[TestSlot]| -> Result<(),()> {
				//error!("data[0].value: {}", data[0].value);
				assert!(data.len() == 1);
				assert!(data[0].value == 19);
				//error!("EP:: Done");
				return Err(());
			});
			tx.send(1);
		});

		assert!(t.end == 0);

		let mut x: TestSlot = Slot::new();
		x.value = 19;
		t.write(x);

		assert!(t.end == 1);
		rx.recv_opt();
		future.get();
		//error!("Test::end");
	}


	#[test]
	fn test_write_read_many() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);
		let (tx, rx): (Sender<int>, Receiver<int>) = channel();

		let mut future = Future::spawn(proc() {
			let mut counter = 0i;
			let mut last = -1i;
			event_processor.start::<BusyWait>(|data: &[TestSlot]| -> Result<(),()> {

				//error!("EP::data.len: {}", data.len());

				for x in data.iter() {
				//	error!("EP:: last: {}, value: {}", last, x.value);
					assert!(last + 1 == x.value);
					counter += 1;
					last = x.value;
					//error!("EP::counter: {}", counter);
				}

				if counter == 1000 {
						return Err(());
				} else {
					return Ok(());
				}

			});
			tx.send(1);
		});

		assert!(t.end == 0);

		for i in range(0i, 1000i) {
			let mut x: TestSlot = Slot::new();
			x.value = i;
			t.write(x);
		}

		//timer::sleep(10000);
		rx.recv_opt();
		future.get();
		//error!("Test::end");

		//
	}


	#[test]
	fn test_write_read_many_rollover() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);
		let (tx, rx): (Sender<int>, Receiver<int>) = channel();

		let mut future = Future::spawn(proc() {
			let mut counter = 0i;
			let mut last = -1i;
			event_processor.start::<BusyWait>(|data: &[TestSlot]| -> Result<(),()> {
				for x in data.iter() {
					//error!(">>>>>>>>>> last: {}, value: {}, -- {}", last, x.value, last + 1 == x.value);
					assert!(last + 1 == x.value);
					counter += 1;
					last = x.value;
					//error!("EP::counter: {}", counter);
				}

				if counter >= 1200 {
						return Err(());
				} else {
					return Ok(());
				}

			});
			tx.send(1);
		});

		assert!(t.end == 0);

		for i in range(0i, 1200i) {
			let mut x: TestSlot = Slot::new();
			x.value = i;
			//error!("______Writing {}", i);
			t.write(x);

		}
		rx.recv_opt();

	}


	#[test]
	fn test_write_read_large() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);
		let (tx, rx): (Sender<int>, Receiver<int>) = channel();


		let mut future = Future::spawn(proc() {
			let mut counter = 0i;
			let mut last = -1i;
			event_processor.start::<BusyWait>(|data: &[TestSlot]| -> Result<(),()> {

				//error!("EP::data.len: {}", data.len());

				for x in data.iter() {
					error!(">>>>>>>>>> last: {}, value: {}, -- {}", last, x.value, last + 1 == x.value);
					assert!(last + 1 == x.value);
					counter += 1;
					last = x.value;
					//error!("counter: {}", counter);
				}

				if counter >= 50000 {
						return Err(());
				} else {
					return Ok(());
				}

			});
			error!("Event processor done");
			tx.send(1);
			return;
		});

		assert!(t.end == 0);

		for i in range(0i, 50001i) {
			let mut x: TestSlot = Slot::new();
			x.value = i;
			//error!("Writing {}", i);
			t.write(x);



		}
		error!("Exit write loop");
		rx.recv_opt();
		error!("Recv_opt done");
		return;
		//
	}




}
