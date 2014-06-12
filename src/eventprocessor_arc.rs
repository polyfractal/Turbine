

use sync::Arc;
use std::sync::atomics::{AtomicInt, SeqCst, Release, Acquire};
use waitstrategy::{WaitStrategy};
use std::cmp::min;

pub struct EventProcessor {
	deps: Vec<Arc<EventProcessor>>,
	sequence: AtomicInt
}

impl EventProcessor {
	pub fn new() -> EventProcessor {
		EventProcessor {
			deps: Vec::with_capacity(2),
			sequence: AtomicInt::new(-1)
		}
	}

	pub fn depends(self, r: &Arc<EventProcessor>) -> EventProcessor {
		let mut new = self.deps.clone();
		new.push(r.clone());
		EventProcessor { deps: new, ..self }
	}

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
}
