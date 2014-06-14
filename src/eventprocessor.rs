

use sync::Arc;
use std::sync::atomics::{AtomicInt, SeqCst, Release, Acquire};
use waitstrategy::{WaitStrategy};
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


	pub fn start<T: WaitStrategy>(&self, f: ||) {
		let wait_strategy: T = WaitStrategy::new();

		let dep_eps = self.graph.get(self.token);
		let mut deps: Vec<&Padded64> = Vec::with_capacity(dep_eps.len());
		for ep in dep_eps.iter() {
			deps.push((*self.cursors).get(*ep));
		}
		drop(dep_eps);

		let cursor = (*self.cursors).get(self.token);
		loop {
			let next = cursor.load() + 1;
			let available = wait_strategy.wait_for(next, &deps);

			// Get data from ring buff

			//f(data);
		}
	}
]

}
