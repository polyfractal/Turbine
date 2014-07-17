
use std::ty::Unsafe;

macro_rules! is_pow2(
    ($x:ident) => (
      (($x != 0) && ($x & ($x - 1)) == 0)
    );
)

/// A container for data inside the RingBuffer
///
/// Slot must be implemented by the user.  A Slot implementation will provide
/// a generic container which holds data to be placed inside of Turbine.  The
/// contents of this container are irrelevant to Turbine, it simply needs to
/// implement the Slot trait.
///
/// Slot's must be Sendable since they are passed between tasks.
///
/// *Note:* The size of the buffer in memory, allocated immediately upon instantiation
/// of Turbine, will be `buffer_size * sizeof(YourSlot)`.
pub trait Slot: Send {
    /// Create a new Slot
    ///
    /// All Slot implementations must provide a new constructor method so that
    /// Turbine can instantiate the object.  If your Slots do not need any instantiation,
    /// simply return an empty struct in this method.
    ///
    ///##Example
    ///
    ///```
    ///struct TestSlot {
    /// pub value: int
    ///}
    ///
    ///impl Slot for TestSlot {
    ///  fn new() -> TestSlot {
    ///    TestSlot {
    ///      value: -1
    ///    }
    ///  }
    ///}
    ///```
    fn new() -> Self;
}

pub struct RingBuffer<T> {
    entries: Unsafe<Vec<T>>
}

impl<T: Slot + Send> RingBuffer<T> {

    pub fn new(size: uint) -> RingBuffer<T> {
        let entries: Unsafe<Vec<T>> = match size {
            0 => fail!("Buffer Size must be greater than zero."),
            s if !(is_pow2!(s)) => fail!("Buffer Size must be a power of two"),
            _ => Unsafe::new(Vec::from_fn(size, |_| Slot::new()))
        };

        RingBuffer::<T> {
            entries: entries
        }
    }

    pub fn get_capacity(&self) -> uint {
        let v: *mut Vec<T> = unsafe { self.entries.get() };
        unsafe { (*v).len() }
    }

    // Unsafe because we have no guarantees the caller won't invalidate this slot
    pub unsafe fn get(&self, from: uint, size: uint) -> &[T] {
        debug!("              RingBuffer get({}, {})", from, size);
        let v: *mut Vec<T> = self.entries.get();
        (*v).slice(from, size)
    }

    // Unsafe because we have no guarantees the caller won't invalidate this slot
    pub unsafe fn write(&self, position: uint, data: T) {
        let v: *mut Vec<T> = self.entries.get();
        let slot = (*v).get_mut(position);
        *slot = data;
    }
}


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
        let _: RingBuffer<TestSlot> = RingBuffer::new(2);
    }

    #[test]
    #[should_fail]
    fn new_ringbuff_non_power_of_two() {
        let _: RingBuffer<TestSlot> = RingBuffer::new(5);
    }

    #[test]
    #[should_fail]
    fn new_ringbuff_zero() {
        let _: RingBuffer<TestSlot> = RingBuffer::new(0);
    }
}
