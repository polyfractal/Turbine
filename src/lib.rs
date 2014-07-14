#![crate_id = "turbine"]
//#![deny(missing_doc)]
#![feature(phase)]
#![feature(macro_rules)]


//! Turbine is a high-performance, non-locking, inter-task communication library.
//!
//! Turbine is a spiritual port of the LMAX-Disruptor pattern.  Although the
//! abstractions used in this library are different from those in the original
//! Disruptor, they share similar concepts and operate on the same principle
//!
//! Turbine is essentially a channel on steroids, permitting data passing and
//! communication between tasks in a very efficient manner.  Turbine uses a variety
//! of techniques -- such as non-locking ring buffer, single producer, consumer
//! dependency management and batching -- to produce very low latencies and high
//! throughput.
//!
//! So why would you choose Turbine?  Turbine is excellent if it forms the core of
//! your application.  Turbine, like Disruptor, is used if several consumers need
//! act on the data in parallel, and then allow the "business" logic to execute.
//! Further, Turbine is used when you need to process millions of events per second.
//!
//! On simple, synthetic tests, Turbine exceeds 30 million messages per second between
//! tasks, while channels cap out around 4m (on the test hardware).
//!
//! That said, Turbine does not replace channels for a variety of reasons.
//!
//! - Channels are much simpler to use
//! - Channels are more efficient if you have low or inconsistent communication requirements
//! - Channels can be MPSC (multi-producer, single-consumer) while Turbine is SPMC
//! - Turbine requires significant memory overhead to initialize (the ring buffer)
//!

#[phase(syntax, link)]







extern crate log;
extern crate sync;

extern crate time;
extern crate libc;

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
mod atomicu64;
mod ringbuffer;


pub struct Turbine<T> {
	finalized: bool,
	epb: Vec<Option<Vec<uint>>>,
	graph: Arc<Vec<Vec<uint>>>,
	cursors: Arc<Vec<Padded64>>,
	ring: Arc<RingBuffer<T>>,
	current_pos: u64,
	size: uint,
	mask: u64,
	until: u64
}

impl<T: Slot> Turbine<T> {
	pub fn new(ring_size: uint) -> Turbine<T> {
		let mut epb = Vec::with_capacity(8);

		Turbine::<T> {
			finalized: false,
			epb: epb,
			graph: Arc::new(vec![]),
			cursors: Arc::new(vec![]),
			ring: Arc::new(RingBuffer::<T>::new(ring_size)),
			current_pos: 0,
			size: ring_size,
			mask: (ring_size - 1) as u64,
			until: (ring_size - 1) as u64
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
			//debug!("Spin...");
			match self.can_write() {
				true => break,
				false => {}
			}
		}

		let write_pos = self.current_pos & self.mask;
		debug!("current_pos is {}, writing to {}", self.current_pos, write_pos);
		unsafe {
			self.ring.write(write_pos as uint, data);
		}

		self.current_pos += 1;
		self.cursors.get(0).store(self.current_pos);
		debug!("Write complete.")

	}

	fn can_write(&mut self) -> bool {
		/*
		debug!("Until is: {} ({})  --  Current is: {} ({})  -- {}",
							self.until, self.until & self.mask,
							self.current_pos, self.current_pos & self.mask,
							self.until == (self.current_pos & self.mask));
		*/
		debug!("{} == {} ({} & {})  -- {}", self.until, self.current_pos & self.mask, self.current_pos, self.mask, self.until == (self.current_pos & self.mask));

		if self.until == (self.current_pos & self.mask) {
			debug!("*****");

			let mut min_cursor = 18446744073709551615;
			for v in self.cursors.iter().skip(1) {
				debug!("CURSOR: {}", v.load());
				//let diff = self.current_pos - v.load();
				min_cursor = min(min_cursor, v.load());

				if (self.current_pos - min_cursor >= self.size as u64) {
					debug!("Not writeable!  {} - {} == {}, which is >= {}", self.current_pos, min_cursor, (self.current_pos - min_cursor), self.size);
					return false;
				}
			}

			/*
			self.until = if (self.current_pos & self.mask) > (min_cursor & self.mask) {
				// 1025					1024
				// current_pos & mask: 1, min_cursor & mask: 0
				// avail = (1024 - 988) + 795
				debug!("UNTIL DECISION: Current > Min");
				debug!("{} + ({} - {}) + {}", self.current_pos, self.size, (self.current_pos & self.mask), (min_cursor & self.mask));
				self.current_pos + (self.size as u64 - (self.current_pos & self.mask)) + (min_cursor & self.mask)
			} else if (self.current_pos & self.mask) == (min_cursor & self.mask) {
				debug!("UNTIL DECISION: Current == Min");
				self.current_pos + (self.until + 1) & self.mask
			} else {
				// current already wrapped
				// current_pos & mask: 0, min_cursor & mask: 1
				// avail = (1024 - 1) + 0 == 234
				debug!("UNTIL DECISION: Current < Min");
				debug!("{} + ({} - {}) + {}", self.current_pos, self.size, (min_cursor & self.mask), self.current_pos & self.mask);

				self.current_pos + (self.size as u64 - (min_cursor & self.mask)) + (self.current_pos & self.mask)
				//self.current_pos + (min_cursor & self.mask)
			};
			*/

			self.until = min_cursor & self.mask;

			//self.until = self.current_pos + (min_cursor & self.mask);
			debug!("current_pos: {}, min_cursor: {}, new until: {}", self.current_pos, min_cursor, self.until);
			debug!("current_pos & mask: {}, min_cursor & mask: {}", (self.current_pos & self.mask), (min_cursor & self.mask));
		}

		true
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
	use std::rand::{task_rng, Rng};

	use libc::funcs::posix88::unistd::usleep;
	use std::io::File;
	use std::num::abs;

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

	struct TestSlotU64 {
		pub value: u64
	}

	impl Slot for TestSlotU64 {
		fn new() -> TestSlotU64 {
			TestSlotU64 {
				value: -1	// Negative value here helps catch bugs since counts will be wrong
			}
		}
	}

/*
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

		assert!(t.current_pos == 0);
		t.write(Slot::new());

		assert!(t.current_pos == 1);
	}


	#[test]
	fn test_write_1024() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);

		assert!(t.current_pos == 0);

		// fill the buffer but don't roll over
		for i in range(1u64, 1023) {
			t.write(Slot::new());

			assert!(t.current_pos == i);
		}

	}


	#[test]
	fn test_write_ring_rollover() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);

		assert!(t.current_pos == 0);

		//move our reader's cursor so we can rollover
		t.cursors.get(1).store(1);

		for i in range(1u64, 1025) {
			t.write(Slot::new());

			assert!(t.current_pos == i);
		}
		t.write(Slot::new());
		assert!(t.current_pos == 1025);
	}

	#[test]
	fn test_write_ring_double_rollover() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);

		assert!(t.current_pos == 0);

		//move our reader's cursor so we can rollover
		t.cursors.get(1).store(1);

		for i in range(1u64, 1025) {
			t.write(Slot::new());

			assert!(t.current_pos == i);
		}

		//move our reader's cursor so we can rollover again
		t.cursors.get(1).store(1025);
		for i in range(1i, 1025i) {
			t.write(Slot::new());
		}
		assert!(t.current_pos == 2048);
	}


	#[test]
	fn test_write_one_read_one() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);
		let (tx, rx): (Sender<int>, Receiver<int>) = channel();

		let mut future = Future::spawn(proc() {
			event_processor.start::<BusyWait>(|data: &[TestSlot]| -> Result<(),()> {
				//debug!("data[0].value: {}", data[0].value);
				assert!(data.len() == 1);
				assert!(data[0].value == 19);
				//debug!("EP:: Done");
				return Err(());
			});
			tx.send(1);
		});

		assert!(t.current_pos == 0);

		let mut x: TestSlot = Slot::new();
		x.value = 19;
		t.write(x);

		assert!(t.current_pos == 1);
		if rx.recv_opt().is_err() == true {fail!()}
		//debug!("Test::end");
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

				//debug!("EP::data.len: {}", data.len());

				for x in data.iter() {
					debug!("EP:: last: {}, value: {}", last, x.value);
					assert!(last + 1 == x.value);
					counter += 1;
					last = x.value;
					debug!("EP::counter: {}", counter);
				}

				if counter == 1000 {
						return Err(());
				} else {
					return Ok(());
				}

			});
			tx.send(1);
		});

		assert!(t.current_pos == 0);

		for i in range(0u64, 1000) {
			let mut x: TestSlot = Slot::new();
			x.value = i as int;
			debug!("Writing: {}", x.value);
			t.write(x);
		}

		if rx.recv_opt().is_err() == true {fail!()}

	}


	#[test]
	fn test_write_read_many_with_rollover() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);
		let (tx, rx): (Sender<int>, Receiver<int>) = channel();

		let mut future = Future::spawn(proc() {
			let mut counter = 0i;
			let mut last = -1i;
			event_processor.start::<BusyWait>(|data: &[TestSlot]| -> Result<(),()> {
				for x in data.iter() {
					debug!(">>>>>>>>>> last: {}, value: {}, -- {}", last, x.value, last + 1 == x.value);
					assert!(last + 1 == x.value);
					counter += 1;
					last = x.value;
					debug!("EP::counter: {}", counter);
				}

				if counter >= 1200 {
						return Err(());
				} else {
					return Ok(());
				}

			});
			tx.send(1);
		});

		for i in range(0u64, 1200) {
			let mut x: TestSlot = Slot::new();
			x.value = i as int;
			debug!("______Writing {}", i);
			t.write(x);

		}
		if rx.recv_opt().is_err() == true {fail!()}

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

				//debug!("EP::data.len: {}", data.len());

				for x in data.iter() {
					debug!(">>>>>>>>>>>>>>>>>>>> last: {}, value: {}, -- {}", last, x.value, last + 1 == x.value);
					assert!(last + 1 == x.value);
					counter += 1;
					last = x.value;
					//debug!("counter: {}", counter);
				}

				if counter >= 50000 {
						return Err(());
				} else {
					return Ok(());
				}

			});
			debug!("Event processor done");
			tx.send(1);
			return;
		});

		for i in range(0u64, 50001) {
			let mut x: TestSlot = Slot::new();
			x.value = i as int;
			debug!("Writing {}", i);
			t.write(x);
		}

		debug!("Exit write loop");
		if rx.recv_opt().is_err() == true {fail!()}
		debug!("Recv_opt done");
		return;
		//
	}


	#[test]
	fn test_random_ep_pause() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);
		let (tx, rx): (Sender<int>, Receiver<int>) = channel();


		let mut future = Future::spawn(proc() {
			let mut counter = 0i;
			let mut last = -1i;
			let mut rng = task_rng();
			event_processor.start::<BusyWait>(|data: &[TestSlot]| -> Result<(),()> {
				let sleep_time = rng.gen_range(0u, 100) as u64;
				debug!("												SLEEPING {}", sleep_time);
				timer::sleep(sleep_time);
				debug!("												DONE SLEEPING");

				for x in data.iter() {
					debug!("									>>>>>>>>>>>>>>>>>>>> last: {}, value: {}, -- {}", last, x.value, last + 1 == x.value);
					assert!(last + 1 == x.value);
					counter += 1;
					last = x.value;
					//debug!("counter: {}", counter);
				}

				if counter >= 50000 {
						return Err(());
				} else {
					return Ok(());
				}

			});
			debug!("Event processor done");
			tx.send(1);
			return;
		});

		for i in range(0u64, 50001) {
			let mut x: TestSlot = Slot::new();
			x.value = i as int;
			debug!("Writing {} -----------------------------------------------------", i);
			t.write(x);
		}

		debug!("Exit write loop");
		if rx.recv_opt().is_err() == true {fail!()}
		debug!("Recv_opt done");
		return;
		//
	}


	#[test]
	fn test_two_readers() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new().unwrap();
		let e2 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);
		let (tx, rx): (Sender<int>, Receiver<int>) = channel();

		let mut future = Future::spawn(proc() {
			let mut counter = 0i;
			let mut last = -1i;
			event_processor.start::<BusyWait>(|data: &[TestSlot]| -> Result<(),()> {
				for x in data.iter() {
					//debug!(">>>>>>>>>> last: {}, value: {}, -- {}", last, x.value, last + 1 == x.value);
					assert!(last + 1 == x.value);
					counter += 1;
					last = x.value;
					//debug!("EP::counter: {}", counter);
				}

				if counter >= 1200 {
						return Err(());
				} else {
					return Ok(());
				}

			});
			tx.send(1);
		});

		let event_processor2 = t.ep_finalize(e2);
		let (tx2, rx2): (Sender<int>, Receiver<int>) = channel();

		let mut future = Future::spawn(proc() {
			let mut counter = 0i;
			let mut last = -1i;
			event_processor2.start::<BusyWait>(|data: &[TestSlot]| -> Result<(),()> {
				for x in data.iter() {
					//debug!(">>>>>>>>>> last: {}, value: {}, -- {}", last, x.value, last + 1 == x.value);
					assert!(last + 1 == x.value);
					counter += 1;
					last = x.value;
					//debug!("EP::counter: {}", counter);
				}

				if counter >= 1200 {
						return Err(());
				} else {
					return Ok(());
				}

			});
			tx2.send(1);
		});

		for i in range(0u64, 1200) {
			let mut x: TestSlot = Slot::new();
			x.value = i as int;
			//debug!("______Writing {}", i);
			t.write(x);

		}
		if rx.recv_opt().is_err() == true {fail!()}
		if rx2.recv_opt().is_err() == true {fail!()}

	}

	#[test]
	fn test_two_readers_dependency() {
		let mut t: Turbine<TestSlot> = Turbine::new(1024);
		let e1 = t.ep_new().unwrap();
		let e2 = t.ep_new().unwrap();

		t.ep_depends(e2, e1);

		let event_processor = t.ep_finalize(e1);
		let (tx, rx): (Sender<int>, Receiver<int>) = channel();

		let mut future = Future::spawn(proc() {
			let mut counter = 0i;
			let mut last = -1i;
			event_processor.start::<BusyWait>(|data: &[TestSlot]| -> Result<(),()> {
				for x in data.iter() {
					//debug!(">>>>>>>>>> last: {}, value: {}, -- {}", last, x.value, last + 1 == x.value);
					assert!(last + 1 == x.value);
					counter += 1;
					last = x.value;
					//debug!("EP::counter: {}", counter);
				}

				if counter >= 1200 {
						return Err(());
				} else {
					return Ok(());
				}

			});
			tx.send(1);
		});

		let event_processor2 = t.ep_finalize(e2);
		let (tx2, rx2): (Sender<int>, Receiver<int>) = channel();

		let mut future = Future::spawn(proc() {
			let mut counter = 0i;
			let mut last = -1i;
			event_processor2.start::<BusyWait>(|data: &[TestSlot]| -> Result<(),()> {
				for x in data.iter() {
					//debug!(">>>>>>>>>> last: {}, value: {}, -- {}", last, x.value, last + 1 == x.value);
					assert!(last + 1 == x.value);
					counter += 1;
					last = x.value;
					//debug!("EP::counter: {}", counter);
				}

				if counter >= 1200 {
						return Err(());
				} else {
					return Ok(());
				}

			});
			tx2.send(1);
		});


		for i in range(0i, 1200i) {
			let mut x: TestSlot = Slot::new();
			x.value = i as int;
			//debug!("______Writing {}", i);
			t.write(x);

		}
		rx.recv_opt();
		rx2.recv_opt();

	}

	#[test]
	fn bench_chan_10m() {

		let (tx_bench, rx_bench): (Sender<int>, Receiver<int>) = channel();


		let mut future = Future::spawn(proc() {
			for _ in range(0i, 10000000)  {
				tx_bench.send(1);
			}

		});

		let start = precise_time_ns();
		let mut counter = 0;
		for i in range(0i, 10000000) {
			counter += rx_bench.recv();
		}
		let end = precise_time_ns();

		future.get();

		error!("Channel: Total time: {}", (end-start) as f32 / 1000000f32);
		error!("Channel: ops/s: {}", 10000000f32 / ((end-start) as f32 / 1000000f32 / 1000f32));
	}

	#[test]
	fn bench_turbine_10m() {
		let mut t: Turbine<TestSlot> = Turbine::new(1048576);
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);
		let (tx, rx): (Sender<int>, Receiver<int>) = channel();

		let mut future = Future::spawn(proc() {
			let mut counter = 0;
			event_processor.start::<BusyWait>(|data: &[TestSlot]| -> Result<(),()> {
				for _ in data.iter() {
					counter += data[0].value;
				}

				if counter == 10000000 {
						return Err(());
				} else {
					return Ok(());
				}

			});
			tx.send(1);
		});

		let start = precise_time_ns();
		for i in range(0i, 10000000) {
			let mut s: TestSlot = Slot::new();
			s.value = 1;
			t.write(s);
		}

		rx.recv_opt();
		let end = precise_time_ns();


		error!("Turbine: Total time: {}", (end-start) as f32 / 1000000f32);
		error!("Turbine: ops/s: {}", 10000000f32 / ((end-start) as f32 / 1000000f32 / 1000f32));
	}
*/


	#[test]
	fn bench_turbine_latency() {
		let path = Path::new("turbine_latency.csv");
		let mut file = match File::create(&path) {
				Err(why) => fail!("couldn't create file: {}", why.desc),
				Ok(file) => file
		};

		let mut t: Turbine<TestSlotU64> = Turbine::new(1048576);
		let e1 = t.ep_new().unwrap();

		let event_processor = t.ep_finalize(e1);
		let (tx, rx): (Sender<Vec<u64>>, Receiver<Vec<u64>>) = channel();

		let mut future = Future::spawn(proc() {
			let mut counter: int = 0;
			let mut latencies = Vec::with_capacity(100000);

			event_processor.start::<BusyWait>(|data: &[TestSlotU64]| -> Result<(),()> {
				for d in data.iter() {
					let end = precise_time_ns();
					let total = abs((end - d.value) as i64) as u64;
					latencies.push(total);

					//error!("{}, {}, {}", d.value, end, total);
					counter += 1;
				}

				if counter == 100000 {
						return Err(());
				} else {
					return Ok(());
				}

			});
			tx.send(latencies);
		});

		for i in range(0i, 100000) {
			let mut s: TestSlotU64 = Slot::new();
			s.value = precise_time_ns();
			t.write(s);

			unsafe { usleep(10); }	//sleep for 10 microseconds
		}

		let latencies = match rx.recv_opt() {
			Ok(l) => l,
			Err(_) => fail!("No latencies were returned!")
		};


		for l in latencies.iter() {
			match file.write_line(l.to_str().as_slice()) {
        Err(why) => {
            fail!("couldn't write to file: {}", why.desc)
        },
        Ok(_) => {}
    	}
		}

	}


	#[test]
	fn bench_chan_latency() {
		let path = Path::new("chan_latency.csv");
		let mut file = match File::create(&path) {
				Err(why) => fail!("couldn't create file: {}", why.desc),
				Ok(file) => file
		};

		let (tx_bench, rx_bench): (Sender<u64>, Receiver<u64>) = channel();


		let mut future = Future::spawn(proc() {
			for _ in range(0i, 100000)  {
				tx_bench.send(precise_time_ns());
				unsafe { usleep(10); }	//sleep for 10 microseconds
			}

		});

		let mut counter: int = 0;
		let mut latencies = Vec::with_capacity(100000);

		for i in range(0i, 100000) {
			counter += 1;
			let end = precise_time_ns();
			let start = rx_bench.recv();
			let total = abs((end - start) as i64) as u64;	// because ticks can go backwards between different cores
			latencies.push(total);
			//error!("{}, {}, {}", start, end, total);
		}

		for l in latencies.iter() {
			match file.write_line(l.to_str().as_slice()) {
				Err(why) => {
						fail!("couldn't write to file: {}", why.desc)
				},
				Ok(_) => {}
			}
		}

		future.get();

	}
}
