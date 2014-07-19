#![feature(phase)]
#![feature(macro_rules)]
#[phase(syntax, link)]

extern crate log;
extern crate sync;

extern crate time;
extern crate turbine;

use turbine::{Turbine, Slot, WaitStrategy, BusyWait};

use std::io::timer;
use std::fmt;
use std::task::{TaskBuilder};
use std::sync::Future;
use time::precise_time_ns;
use std::io::File;

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

fn bench_chan_100m() -> f32 {

    let (tx_bench, rx_bench): (Sender<int>, Receiver<int>) = channel();

    let mut future = Future::spawn(proc() {
        for _ in range(0i, 100000000)  {
            tx_bench.send(1);
        }

    });

    let start = precise_time_ns();
    let mut counter = 0;
    for i in range(0i, 100000000) {
        counter += rx_bench.recv();
    }
    let end = precise_time_ns();

    future.get();

    error!("Channel: Total time: {}s", (end-start) as f32 / 1000000000f32);
    error!("Channel: ops/s: {}", 100000000f32 / ((end-start) as f32 / 1000000000f32));
    100000000f32 / ((end-start) as f32 / 1000000000f32)
}


fn bench_turbine_100m() -> f32 {
    let mut t: Turbine<TestSlot> = Turbine::new(16384);
    let e1 = t.ep_new().unwrap();

    let event_processor = t.ep_finalize(e1);
    let (tx, rx): (Sender<int>, Receiver<int>) = channel();

    let mut future = Future::spawn(proc() {
        let mut counter = 0;
        event_processor.start::<BusyWait>(|data: &[TestSlot]| -> Result<(),()> {
            for d in data.iter() {
                counter += d.value;
            }

            if counter == 100000000 {
                return Err(());
            } else {
                return Ok(());
            }

        });
        tx.send(1);
    });

    let start = precise_time_ns();
    for i in range(0i, 100000000) {
        let mut s: TestSlot = Slot::new();
        s.value = 1;
        t.write(s);
    }

    rx.recv_opt();
    let end = precise_time_ns();


     // 1000000000 ns == 1s
    error!("Turbine: Total time: {}s", (end-start) as f32 / 1000000000f32);
    error!("Turbine: ops/s: {}", 100000000f32 / ((end-start) as f32 / 1000000000f32));
    100000000f32 / ((end-start) as f32 / 1000000000f32)
}

fn main() {

    let path = Path::new("turbine_throughput.csv");
    let mut file = match File::create(&path) {
            Err(why) => fail!("couldn't create file: {}", why.desc),
            Ok(file) => file
    };

    for i in range(0i,20) {
        let value = bench_turbine_100m();
        match file.write_line(value.to_string().as_slice()) {
            Err(why) => {
                fail!("couldn't write to file: {}", why.desc)
            },
            Ok(_) => {}
        }
    }

    let path = Path::new("chan_throughput.csv");
    let mut file = match File::create(&path) {
            Err(why) => fail!("couldn't create file: {}", why.desc),
            Ok(file) => file
    };
    for i in range(0i,20) {
        let value =  bench_chan_100m();
        match file.write_line(value.to_string().as_slice()) {
            Err(why) => {
                fail!("couldn't write to file: {}", why.desc)
            },
            Ok(_) => {}
        }
    }
}

