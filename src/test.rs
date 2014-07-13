#![feature(phase)]
#![feature(macro_rules)]
#[phase(syntax, link)]

extern crate log;
extern crate sync;

extern crate time;
extern crate turbine;



#[cfg(test)]
mod test {

	use turbine::{Turbine, Slot, WaitStrategy, BusyWait};

	use std::io::timer;
	use std::fmt;
	use std::task::{TaskBuilder};
	use std::sync::Future;
	use time::precise_time_ns;

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

		debug!("Total time: {}", (end-start) as f32 / 1000000f32);
		debug!("ops/s: {}", 10000000f32 / ((end-start) as f32 / 1000000f32 / 1000f32));
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


		//debug!("Total time: {}", (end-start) as f32 / 1000000f32);
		//debug!("ops/s: {}", 10000000f32 / ((end-start) as f32 / 1000000f32 / 1000f32));
		//
	}
}
