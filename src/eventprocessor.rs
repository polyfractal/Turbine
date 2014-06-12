

use sync::Arc;
use std::sync::atomics::{AtomicInt, SeqCst, Release, Acquire};
use waitstrategy::{WaitStrategy};
use std::cmp::min;
use paddedatomics::Padded64;

pub struct EventProcessor {
	graph: Arc<Vec<Vec<uint>>>,
	cursors: Arc<Vec<Padded64>>,
	token: uint
}


impl EventProcessor {
	pub fn new(graph: Arc<Vec<Vec<uint>>>, cursors: Arc<Vec<Padded64>>, token: uint) -> EventProcessor {
		EventProcessor {
			graph: graph,
			cursors: cursors,
			token: token
		}
	}

/*
	pub fn start<T: WaitStrategy>(&self, f: ||) {
		let wait_strategy: T = WaitStrategy::new();

		loop {
			let next = self.get_sequence() + 1;
			let available = wait_strategy.wait_for(next, self);

			self.next(available);
		}
	}

	pub fn lowest_dep(&self) -> int {
		let mut lowest = -1;
		for ep in self.deps.iter() {
			lowest = min(lowest, ep.get_sequence());
		}
		lowest
	}

	pub fn get_sequence(&self) -> int {
		self.sequence.load(Acquire)
	}

	fn next(&self, next: int) {

	}
	*/
}
