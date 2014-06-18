
#![allow(dead_code)]

use std::sync::atomics::{AtomicInt, SeqCst, Release, Acquire};
use std::fmt::{Formatter, Result, Show};


//------------------------- Padded 64 -------------------------//

/// AtomicInt padded with 64 bytes
pub struct Padded64 {
	p: [u64, ..7],
	counter: AtomicInt
}

impl Padded64 {
	pub fn new(x: int) -> Padded64 {
		Padded64 {
			p: [0u64,0u64,0u64,0u64,0u64,0u64,0u64],
			counter: AtomicInt::new(x)
		}
	}
	#[inline]
	pub fn add(&self, x: int) -> int {
		self.counter.fetch_add(x, SeqCst)
	}

	#[inline]
	pub fn load(&self) -> int {
		self.counter.load(SeqCst)
	}

	#[inline]
	pub fn store(&self, x: int) {
		self.counter.store(x, SeqCst);
	}

	#[inline]
	pub fn reset(&self) {
		self.store(0);
	}

	#[inline]
	pub fn or(&self, x: int) -> int {
		self.counter.fetch_or(x, SeqCst)
	}

	#[inline]
	pub fn and(&self, x: int) -> int {
		self.counter.fetch_and(x, SeqCst)
	}
}

impl Show for Padded64 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f.buf, "{}", self.counter.load(SeqCst))
				//write!(f.buf, "{}", 0)
    }
}
