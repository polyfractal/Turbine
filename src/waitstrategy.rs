
use eventprocessor::EventProcessor;
use paddedatomics::Padded64;
use std::cmp::{min, max};


pub trait WaitStrategy {
	fn new(ring_size: uint) -> Self;
	fn get_ring_size(&self) -> uint;
	fn wait_for(&self, sequence: int, ep: &Vec<&Padded64>) -> uint;
	fn can_read(&self, sequence: int, deps: &Vec<&Padded64>) -> Option<int>;

	fn until(&self, sequence: int, deps: &Vec<&Padded64>) -> int {
		let mut next: Option<int> = None;

		//error!("deps: {}", deps);
		for v in deps.iter() {
			let pos: int = v.load();

			//error!("Dep: {}, Sequence: {}", pos, sequence);
			if pos != -1 {
				let adjusted = match pos < sequence {
					true => pos + self.get_ring_size() as int,
					false => pos
				};

				next = match next {
					None => Some(adjusted),
					Some(p) => {
						Some(min(p, adjusted))
					}
				}
			}
		}

		let final = match next {
			None => -1,
			Some(p) => p
		};

		//error!("WaitStrategy::until:  {}", final);
		final
	}

}

pub struct BusyWait {
	ring_size: uint,
  ring_mask: uint
}

impl WaitStrategy for BusyWait {
	fn new(ring_size: uint) -> BusyWait {
		BusyWait {
			ring_size: ring_size,
			ring_mask: ring_size -1
		}
	}

	fn get_ring_size(&self) -> uint {
		self.ring_size
	}

	fn wait_for(&self, sequence: int, deps: &Vec<&Padded64>) -> uint {
		let mut available = 0;
		loop {
			match self.can_read(sequence, deps) {
				Some(v) => {
					available = v;
					break
				},
				None => {}
			}
		}
		//error!("					Wait done, returning {}", available);
		available as uint
	}

	fn can_read(&self, sequence: int, deps: &Vec<&Padded64>) -> Option<int> {
		//return cb->end == (cb->start ^ cb->size);
		let mut min_cursor = (self.ring_size * 2) as int + 1;

		for v in deps.iter() {
			let cursor = v.load();
			//error!("					dep cursor: {}, ring_size: {}, sequence: {}, calculation: {}", cursor, self.ring_size as int, sequence, sequence == (cursor ^ self.ring_size as int));

			if sequence == cursor {
				return None;	// at same position as a dependency. we can't move
			}
			min_cursor = min(min_cursor, cursor);
			//error!("					dep cursor: {}, ring_size: {}, sequence: {}, calculation: {}", cursor, self.ring_size as int, sequence, sequence == (cursor ^ self.ring_size as int));
			//error!("					min_cursor: {}", min_cursor);

			//if sequence == (cursor ^ self.ring_size as int) {
			//	return false;	// full ring buffer, same position but flipped parity bits
			//}
		}
		Some(min_cursor)
	}
}
