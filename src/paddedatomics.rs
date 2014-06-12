
#![allow(dead_code)]

use std::sync::atomics::{AtomicInt, SeqCst, Release, Acquire};

/// Atomic counter which may have variable padding
///
/// Counters are backed by a single AtomicInt, but padded with a variable
/// amount of space to prevent false sharing between cores/CPUs
pub trait Counter {

	fn add(&self, x: int) -> int;
	fn load(&self) -> int;
	fn store(&self, x: int);
	fn reset(&self);
	fn or(&self, x: int) -> int;
	fn and(&self, x: int) -> int;
}

//-------------------------- Unpadded  --------------------------//

/// AtomicInt padded with 0 bytes
pub struct Unpadded {
	counter: AtomicInt
}

impl Unpadded {
	pub fn new(x: int) -> Unpadded {
		Unpadded {
			counter: AtomicInt::new(x)
		}
	}
}
impl Counter for Unpadded {

	#[inline]
	fn add(&self, x: int) -> int {
		self.counter.fetch_add(x, Release)
	}

	#[inline]
	fn load(&self) -> int {
		self.counter.load(Acquire)
	}

	#[inline]
	fn store(&self, x: int) {
		self.counter.store(x, SeqCst);
	}

	#[inline]
	fn reset(&self) {
		self.store(0);
	}

	#[inline]
	fn or(&self, x: int) -> int {
		self.counter.fetch_or(x, Release)
	}

	#[inline]
	fn and(&self, x: int) -> int {
		self.counter.fetch_and(x, Release)
	}
}

//------------------------- Padded 32 -------------------------//

/// AtomicInt padded with 32 bytes
pub struct Padded32 {
	p: [u64, ..3],
	counter: AtomicInt
}

impl Padded32 {
	pub fn new(x: int) -> Padded32 {
		Padded32 {
			p: [0u64,0u64,0u64],
			counter: AtomicInt::new(x)
		}
	}
}
impl Counter for Padded32 {

	#[inline]
	fn add(&self, x: int) -> int {
		self.counter.fetch_add(x, Release)
	}

	#[inline]
	fn load(&self) -> int {
		self.counter.load(Acquire)
	}

	#[inline]
	fn store(&self, x: int) {
		self.counter.store(x, SeqCst);
	}

	#[inline]
	fn reset(&self) {
		self.store(0);
	}

	#[inline]
	fn or(&self, x: int) -> int {
		self.counter.fetch_or(x, Release)
	}

	#[inline]
	fn and(&self, x: int) -> int {
		self.counter.fetch_and(x, Release)
	}
}

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
}
impl Counter for Padded64 {

	#[inline]
	fn add(&self, x: int) -> int {
		self.counter.fetch_add(x, Release)
	}

	#[inline]
	fn load(&self) -> int {
		self.counter.load(Acquire)
	}

	#[inline]
	fn store(&self, x: int) {
		self.counter.store(x, SeqCst);
	}

	#[inline]
	fn reset(&self) {
		self.store(0);
	}

	#[inline]
	fn or(&self, x: int) -> int {
		self.counter.fetch_or(x, Release)
	}

	#[inline]
	fn and(&self, x: int) -> int {
		self.counter.fetch_and(x, Release)
	}
}

//------------------------- Padded 128 -------------------------//

/// AtomicInt padded with 128 bytes
pub struct Padded128 {
	p: [u64, ..15],
	counter: AtomicInt
}

impl Padded128 {
	pub fn new(x: int) -> Padded128 {
		Padded128 {
			p: [0u64,0u64,0u64,0u64,0u64,0u64,0u64,0u64,0u64,0u64,
					0u64,0u64,0u64,0u64,0u64],
			counter: AtomicInt::new(x)
		}
	}
}
impl Counter for Padded128 {

	#[inline]
	fn add(&self, x: int) -> int {
		self.counter.fetch_add(x, Release)
	}

	#[inline]
	fn load(&self) -> int {
		self.counter.load(Acquire)
	}

	#[inline]
	fn store(&self, x: int) {
		self.counter.store(x, SeqCst);
	}

	#[inline]
	fn reset(&self) {
		self.store(0);
	}

	#[inline]
	fn or(&self, x: int) -> int {
		self.counter.fetch_or(x, Release)
	}

	#[inline]
	fn and(&self, x: int) -> int {
		self.counter.fetch_and(x, Release)
	}
}

//------------------------- Padded 256 -------------------------//

/// AtomicInt padded with 256 bytes
pub struct Padded256 {
	p: [u64, ..31],
	counter: AtomicInt
}

impl Padded256 {
	pub fn new(x: int) -> Padded256 {
		Padded256 {
			p: [0u64,0u64,0u64,0u64,0u64,0u64,0u64,0u64,0u64,0u64,
					0u64,0u64,0u64,0u64,0u64,0u64,0u64,0u64,0u64,0u64,
					0u64,0u64,0u64,0u64,0u64,0u64,0u64,0u64,0u64,0u64,0u64],
			counter: AtomicInt::new(x)
		}
	}
}
impl Counter for Padded256 {

	#[inline]
	fn add(&self, x: int) -> int {
		self.counter.fetch_add(x, Release)
	}

	#[inline]
	fn load(&self) -> int {
		self.counter.load(Acquire)
	}

	#[inline]
	fn store(&self, x: int) {
		self.counter.store(x, SeqCst);
	}

	#[inline]
	fn reset(&self) {
		self.store(0);
	}

	#[inline]
	fn or(&self, x: int) -> int {
		self.counter.fetch_or(x, Release)
	}

	#[inline]
	fn and(&self, x: int) -> int {
		self.counter.fetch_and(x, Release)
	}
}
