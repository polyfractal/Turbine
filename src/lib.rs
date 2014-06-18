#![crate_id = "turbine"]

#![feature(phase)]
#![feature(macro_rules)]
#[phase(syntax, link)]

//#![deny(missing_doc)]





extern crate log;
extern crate sync;

use sync::Arc;
use eventprocessor::EventProcessor;
use paddedatomics::Padded64;
use ringbuffer::{RingBuffer};
use std::cmp::{min, max};
use std::fmt;

pub use ringbuffer::Slot;

mod eventprocessor;
mod waitstrategy;
mod paddedatomics;
mod ringbuffer;





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
impl fmt::Show for TestSlot {
		fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				write!(f.buf, "{}", self.value)
		}
}


struct Turbine<T> {
	finalized: bool,
	epb: Vec<Option<Vec<uint>>>,
	graph: Arc<Vec<Vec<uint>>>,
	cursors: Arc<Vec<Padded64>>,
	ring: Arc<RingBuffer<T>>,
	until: int,
	current: int,
	mask: int
}

impl<T: Slot + Send + fmt::Show> Turbine<T> {
	pub fn new() -> Turbine<T> {
		let mut epb = Vec::with_capacity(8);

		Turbine::<T> {
			finalized: false,
			epb: epb,
			graph: Arc::new(vec![]),
			cursors: Arc::new(vec![]),
			ring: Arc::new(RingBuffer::<T>::new(1024)),
			until: 1023,
			current: 0,
			mask: 1023
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
			cursors.push(Padded64::new(-1));
		}

		self.graph = Arc::new(eps);
		self.cursors = Arc::new(cursors);
		drop(&self.epb);
		self.finalized = true;
	}

	pub fn write(&mut self, data: T) {

		loop {
			let next = self.current & self.mask;
			error!("Write next is: {}, until is: {}", next, self.until)
			if next != self.until {
				error!("Turbine::write to {}", next);
				unsafe {
					self.ring.write(next as uint, data);
				}
				self.current = next + 1;
				self.cursors.get(0).store(next + 1);
				error!("CURSOR becomes {}", self.current);
				return;

			} else {
					error!("Write Spin...");
					self.until = self.closest_counter_clockwise(self.current);

					error!("Self.until now: {}", self.until);
			}

		}

	}
	fn closest_counter_clockwise(&self, pos: int) -> int {
		let first_pos = self.cursors.get(1).load();
		error!("First_pos: {}", first_pos);
		let mut max_pos = first_pos;
		let mut min_pos = first_pos;

		error!("Self.cursors.len(): {}", self.cursors.len());
		for v in self.cursors.iter().skip(1) {
			let pos = v.load();
			error!("next pos: {}", pos);
			min_pos = min(min_pos, pos);
			max_pos = max(max_pos, pos);
		}

		error!("pos: {}, max_pos: {}, min_pos: {}", pos, max_pos, min_pos);
		if (max_pos > pos) {
			max_pos
		} else {
			min_pos
		}
	}

/*
	fn survey_cursors(&self) -> (int, int) {
		let first_pos = self.cursors.get(1).load();
		let mut max_pos = first_pos;
		let mut min_pos = first_pos;
		for v in self.cursors.iter().skip(2) {
			let pos = v.load();
			min_pos = min(min_pos, pos);
			max_pos = max(max_pos, pos);
		}
		error!("Write Survey: {}, {}", min_pos, max_pos);
		(min_pos, max_pos)
	}
*/





}


#[cfg(test)]
mod test {

	use Turbine;
	use Slot;
	use waitstrategy::BusyWait;
	use std::io::timer;
	use std::fmt;
	use std::task::{TaskBuilder};
	use sync::Future;

	use TestSlot;

	#[test]
	fn test_init() {
		let t: Turbine<TestSlot> = Turbine::new();
	}

	#[test]
	fn test_create_epb() {
		let mut t: Turbine<TestSlot> = Turbine::new();
		let e1 = t.ep_new();
	}

	#[test]
	fn test_depends() {
		let mut t: Turbine<TestSlot> = Turbine::new();
		let e1 = t.ep_new().unwrap();
		let e2 = t.ep_new().unwrap();

		t.ep_depends(e2, e1);
	}

	#[test]
	fn test_many_depends() {
		let mut t: Turbine<TestSlot> = Turbine::new();
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
		let mut t: Turbine<TestSlot> = Turbine::new();
		let e1 = t.ep_new();
		assert!(e1.is_ok() == true);

		let event_processor = t.ep_finalize(e1.unwrap());

		let e2 = t.ep_new();
		assert!(e2.is_err() == true);
	}

	#[test]
	fn test_double_finalize() {
		let mut t: Turbine<TestSlot> = Turbine::new();
		let e1 = t.ep_new();
		assert!(e1.is_ok() == true);

		let event_processor = t.ep_finalize(e1.unwrap());
		let event_processor2 = t.ep_finalize(e1.unwrap());

		let e2 = t.ep_new();
		assert!(e2.is_err() == true);
	}

	#[test]
	fn test_send_task() {
		let mut t: Turbine<TestSlot> = Turbine::new();
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
		let mut t: Turbine<TestSlot> = Turbine::new();
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);

		assert!(t.current == 0);

		t.write(Slot::new());

		assert!(t.current == 1);
	}


	#[test]
	fn test_write_1024() {
		let mut t: Turbine<TestSlot> = Turbine::new();
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);

		assert!(t.current == 0);

		// fill the buffer but don't roll over
		for i in range(1, 1023) {
			t.write(Slot::new());

			assert!(t.current == i);
		}

	}


	#[test]
	fn test_write_ring_rollover() {
		let mut t: Turbine<TestSlot> = Turbine::new();
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);

		assert!(t.current == 0);

		assert!(t.cursors.len() == 2);
		//move our reader's cursor so we can rollover
		t.cursors.get(1).store(1);

		for i in range(1, 1025) {
			t.write(Slot::new());

			assert!(t.current == i);
		}

		t.write(Slot::new());

		assert!(t.current == 1);
	}


	#[test]
	fn test_write_one_read_one() {
		let mut t: Turbine<TestSlot> = Turbine::new();
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);
		let (tx, rx) = channel();

		let mut future = Future::spawn(proc() {
			event_processor.start::<BusyWait>(|data: &[TestSlot]| -> Result<(),()> {
				assert!(data.len() == 1);
				assert!(data[0].value == 19);
				error!("EP:: Done");
				return Err(());
			});
			tx.send(1);
		});

		assert!(t.current == 0);

		let mut x: TestSlot = Slot::new();
		x.value = 19;
		t.write(x);

		assert!(t.current == 1);
		rx.recv_opt();
		future.get();
		error!("Test::end");
	}


	#[test]
	fn test_write_read_many() {
		let mut t: Turbine<TestSlot> = Turbine::new();
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);
		let (tx, rx) = channel();

		let mut future = Future::spawn(proc() {
			let mut counter = 0;
			let mut last = -1;
			event_processor.start::<BusyWait>(|data: &[TestSlot]| -> Result<(),()> {

				//error!("EP::data.len: {}", data.len());

				for x in data.iter() {
					error!("EP:: last: {}, value: {}", last, x.value);
					assert!(last + 1 == x.value);
					counter += 1;
					last = x.value;
					error!("EP::counter: {}", counter);
				}

				if counter == 1000 {
						return Err(());
				} else {
					return Ok(());
				}

			});
			tx.send(1);
		});

		assert!(t.current == 0);

		for i in range(0, 1000) {
			let mut x: TestSlot = Slot::new();
			x.value = i;
			t.write(x);
		}

		//timer::sleep(10000);
		rx.recv_opt();
		future.get();
		error!("Test::end");

		//
	}
/*
	#[test]
	fn test_write_read_many_rollover() {
		let mut t: Turbine<TestSlot> = Turbine::new();
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);
		let (tx, rx) = channel();

		let mut future = Future::spawn(proc() {
			let mut counter = 0;
			let mut last = -1;
			event_processor.start::<BusyWait>(|data: &[TestSlot]| -> Result<(),()> {

				error!("EP::data.len: {}", data.len());

				for x in data.iter() {
					error!(">>>>>>>>>> last: {}, value: {}, -- {}", last, x.value, last + 1 == x.value);
					assert!(last + 1 == x.value);
					counter += 1;
					last = x.value;
					error!("EP::counter: {}", counter);
				}

				if counter == 1024 {
						return Err(());
				} else {
					return Ok(());
				}

			});
			tx.send(1);
		});

		assert!(t.get_pos() == 0);
		assert!(t.current == 0);

		for i in range(0, 1025) {
			let mut x: TestSlot = Slot::new();
			x.value = i;
			error!("Writing {}", i);
			t.write(x);

		}
		error!("Test::end");
		rx.recv_opt();
		future.get();

		//timer::sleep(10000);
	}

	#[test]
	fn test_write_read_large() {
		let mut t: Turbine<TestSlot> = Turbine::new();
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);
		let (tx, rx) = channel();

		assert!(t.cursors.len() == 2);

		let mut future = Future::spawn(proc() {
			let mut counter = 0;
			let mut last = -1;
			event_processor.start::<BusyWait>(|data: &[TestSlot]| -> Result<(),()> {

				error!("EP::data.len: {}", data.len());

				for x in data.iter() {
					error!(">>>>>>>>>> last: {}, value: {}, -- {}", last, x.value, last + 1 == x.value);
					assert!(last + 1 == x.value);
					counter += 1;
					last = x.value;
					error!("EP::counter: {}", counter);
					//timer::sleep(500);
				}

				if counter == 50000 {
						return Err(());
				} else {
					return Ok(());
				}

			});
			tx.send(1);
		});

		assert!(t.get_pos() == 0);
		assert!(t.current == 0);

		for i in range(0, 50002) {
			let mut x: TestSlot = Slot::new();
			x.value = i;
			error!("Writing {}", i);
			t.write(x);
			timer::sleep(10);

		}
		timer::sleep(50000);
		error!("Test::end");
		rx.recv_opt();
		future.get();

		//
	}
*/
}
