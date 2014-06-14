
use eventprocessor::EventProcessor;
use paddedatomics::Padded64;
use std::cmp::min;


pub trait WaitStrategy {
	fn new() -> Self;
	fn wait_for(&self, sequence: int, ep: &Vec<&Padded64>) -> int;

	fn lowest_dep(&self, deps: &Vec<&Padded64>) -> int {
		let mut lowest = -1;
		for cursor in deps.iter() {
			lowest = min(lowest, cursor.load());
		}
		lowest
	}
}

pub struct BusyWait;

impl WaitStrategy for BusyWait {
	fn new() -> BusyWait {
		BusyWait
	}

	fn wait_for(&self, sequence: int, deps: &Vec<&Padded64>) -> int {
		let mut avail: int  = self.lowest_dep(deps);
		while avail < sequence {
			avail = self.lowest_dep(deps);
		}
		avail
	}
}
