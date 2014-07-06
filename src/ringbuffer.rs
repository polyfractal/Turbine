
use std::ty::Unsafe;
use std::fmt;

macro_rules! is_pow2(
    ($x:ident) => (
      (($x != 0) && ($x & ($x - 1)) == 0)
    );
)

pub trait Slot: Send {
	fn new() -> Self;
}

pub struct RingBuffer<T> {
	entries: Unsafe<Vec<T>>,
	mask: uint
}

impl<T: Slot + Send> RingBuffer<T> {

	pub fn new(size: uint) -> RingBuffer<T> {
    let entries: Unsafe<Vec<T>> = match size {
			0 => fail!("Buffer Size must be greater than zero."),
			s if !(is_pow2!(s)) => fail!("Buffer Size must be a power of two"),
			_ => unsafe { Unsafe::new(Vec::from_fn(size, |_| Slot::new())) }
		};

		RingBuffer::<T> {
			entries: entries,
			mask: size - 1
		}
	}

  pub fn get_capacity(&self) -> uint {
    let v: *mut Vec<T> = unsafe { self.entries.get() };
    unsafe { (*v).len() }
  }

  // Unsafe because we have no guarantees the caller won't invalidate this slot
  pub unsafe fn get(&self, from: uint, size: uint) -> &[T] {
    //error!("              RingBuffer get({}, {})", from, size);
    let v: *mut Vec<T> = unsafe { self.entries.get() };
    //unsafe { println!("Ring: {}", (*v)); }
    unsafe { (*v).slice(from, size) }
  }

  // Unsafe because we have no guarantees the caller won't invalidate this slot
  pub unsafe fn write(&self, position: uint, data: T) {
    let v: *mut Vec<T> = unsafe { self.entries.get() };
    let slot = unsafe { (*v).get_mut(position) };
    *slot = data;
  }
}

/*

#[cfg(test)]
mod tests {

	use super::{RingBuffer, Slot};

  #[deriving(Show)]
  struct TestSlot;

  impl Slot for TestSlot {
    fn new() -> TestSlot {
      TestSlot
    }
  }


	#[test]
	fn new_ringbuf() {
		let r: RingBuffer<TestSlot> = RingBuffer::new(2);
	}

  #[test]
  #[should_fail]
  fn new_ringbuff_non_power_of_two() {
    let r: RingBuffer<TestSlot> = RingBuffer::new(5);
  }

  #[test]
  #[should_fail]
  fn new_ringbuff_zero() {
    let r: RingBuffer<TestSlot> = RingBuffer::new(0);
  }
}

*/
