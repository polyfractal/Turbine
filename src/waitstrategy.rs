
use eventprocessor::EventProcessor;


pub trait WaitStrategy {
	fn new() -> Self;
	fn wait_for(&self, sequence: int, ep: &EventProcessor) -> int;
}

pub struct BusyWait;

impl WaitStrategy for BusyWait {
	fn new() -> BusyWait {
		BusyWait
	}

	fn wait_for(&self, sequence: int, ep: &EventProcessor) -> int {
		//let mut avail = ep.lowest_dep();
		//while avail < sequence {
			//avail = ep.lowest_dep();
		//}
		//avail
		0
	}
}
