
macro_rules! is_pow2(
    ($x:ident) => (
      (($x != 0) && ($x & ($x - 1)) == 0)
    );
)

pub trait Slot {
	fn new() -> Self;
}

pub struct RingBuffer<T> {
	entries: Vec<T>,
	mask: uint
}

impl<T: Slot> RingBuffer<T> {

	pub fn new(size: uint) -> RingBuffer<T> {

    let entries: Vec<T> = match size {
			0 => fail!("Buffer Size must be greater than zero."),
			s if !(is_pow2!(s)) => fail!("Buffer Size must be a power of two"),
			_ => Vec::from_fn(size, |_| Slot::new())
		};

		RingBuffer::<T> {
			entries: entries,
			mask: size - 1
		}

	}
}



#[cfg(test)]
mod tests {

	use super::{RingBuffer, Slot};

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
