
use eventprocessor::EventProcessor;
use paddedatomics::Padded64;
use std::cmp::{min, max};

/// A trait which provides a unified interface to various waiting strategies
pub trait WaitStrategy {

	/// Instantiate a new WaitStrategy. Must provide the size of the underlying buffer
	fn new(ring_size: uint) -> Self;

	/// Get the underlying max buffer capacity
	fn get_ring_size(&self) -> uint;

	/// Wait for the requested sequence, but return the largest available
	///
	/// Provided a target cursor position and a slice of dependency cursors,
	/// this method will block until a slot is available to read from.  The method
	/// of blocking varies depending on the implementation (e.g. busy-wait, sleep, etc).
	///
	/// This method should return the highest available position in the buffer to
	/// allow EventProcessors to batch reads
	fn wait_for(&self, sequence: u64, ep: &Vec<&Padded64>) -> u64;
}

/// An implementation of WaitStrategy that busy-spins while waiting
///
/// This strategy should have the best perforamnce and keep caches hot, but will chew
/// CPU while there is no work to be done.
pub struct BusyWait {
	ring_size: uint,
  ring_mask: uint
}

impl BusyWait {
	fn can_read(&self, sequence: u64, deps: &Vec<&Padded64>) -> Option<u64> {
		let mut min_cursor = 18446744073709551615;

		for v in deps.iter() {
			let cursor = v.load();
			debug!("					cursor: {}", cursor);

			if sequence == cursor {
				debug!("					Same as dep cursor, abort!");
				return None;	// at same position as a dependency. we can't move
			}
			min_cursor = min(min_cursor, cursor);
			debug!("					dep cursor: {}, ring_size: {}, sequence: {}", cursor, self.ring_size as int, sequence);
			debug!("					min_cursor: {}", min_cursor);

		}
		Some(min_cursor)
	}
}

impl WaitStrategy for BusyWait {
	fn new(ring_size: uint) -> BusyWait {
		BusyWait {
			ring_size: ring_size,
			ring_mask: ring_size - 1
		}
	}

	fn get_ring_size(&self) -> uint {
		self.ring_size
	}

	fn wait_for(&self, sequence: u64, deps: &Vec<&Padded64>) -> u64 {
		let mut available: u64 = 0;
		debug!("					Waiting for: {}", sequence);
		loop {
			match self.can_read(sequence, deps) {
				Some(v) => {
					available = v;
					break
				},
				None => {}
			}
		}
		debug!("					Wait done, returning {}", available);
		available
	}
}
