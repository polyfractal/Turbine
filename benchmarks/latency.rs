#![feature(phase)]
#![feature(macro_rules)]
#[phase(syntax, link)]

extern crate log;
extern crate sync;
extern crate libc;
extern crate time;
extern crate turbine;

use turbine::{Turbine, Slot, WaitStrategy, BusyWait};

use std::io::timer;
use std::fmt;
use std::task::{TaskBuilder};
use std::sync::Future;
use time::precise_time_ns;
use std::num::{abs,pow};
use libc::funcs::posix88::unistd::usleep;
use std::io::File;

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
        let mut latencies: Vec<u64> = Vec::from_fn(100, |i| 0);

        event_processor.start::<BusyWait>(|data: &[TestSlotU64]| -> Result<(),()> {
            for d in data.iter() {
                let end = precise_time_ns();
                let total = abs((end - d.value) as i64) as u64;

                for i in range(1u,100) {
                    if pow(2, i) > total {
                        *latencies.get_mut(i) += 1;
                        break;
                    }
                }

                //error!("{}, {}, {}", d.value, end, total);
                counter += 1;
            }

            if counter == 50000000 {
                return Err(());
            } else {
                return Ok(());
            }

        });
        tx.send(latencies);
    });

    for i in range(0i, 50000000) {
        let mut s: TestSlotU64 = Slot::new();
        s.value = precise_time_ns();
        t.write(s);

        unsafe { usleep(10); }	//sleep for 1 microseconds
    }

    let latencies = match rx.recv_opt() {
        Ok(l) => l,
        Err(_) => fail!("No latencies were returned!")
    };


    for l in latencies.iter() {
        match file.write_line(l.to_string().as_slice()) {
            Err(why) => {
                fail!("couldn't write to file: {}", why.desc)
            },
            Ok(_) => {}
        }
    }

}


fn bench_chan_latency() {
    let path = Path::new("chan_latency.csv");
    let mut file = match File::create(&path) {
            Err(why) => fail!("couldn't create file: {}", why.desc),
            Ok(file) => file
    };

    let (tx_bench, rx_bench): (Sender<u64>, Receiver<u64>) = channel();


    let mut future = Future::spawn(proc() {
        for _ in range(0i, 50000000)  {
            let x = precise_time_ns();
            tx_bench.send(x);
            unsafe { usleep(10); }	//sleep for 1 microseconds
        }

    });

    let mut counter: int = 0;
    let mut latencies: Vec<u64> = Vec::from_fn(100, |i| 0);

    for i in range(0i, 50000000) {
        counter += 1;
        let end = precise_time_ns();
        let start = rx_bench.recv();
        let total = abs((end - start) as i64) as u64;	// because ticks can go backwards between different cores
        for i in range(1u,100) {
            if pow(2, i) > total {
                *latencies.get_mut(i) += 1;
                break;
            }
        }
    }

    for l in latencies.iter() {
        match file.write_line(l.to_string().as_slice()) {
            Err(why) => {
                fail!("couldn't write to file: {}", why.desc)
            },
            Ok(_) => {}
        }
    }

    future.get();
}

fn main() {
    bench_turbine_latency();
    error!("Starting channels");
    bench_chan_latency();
}

