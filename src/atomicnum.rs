#![allow(dead_code)]

use std::intrinsics;
use std::cell::UnsafeCell;
use std::sync::atomic::Ordering::{self, Release, Acquire, AcqRel, Relaxed};

/// An unsigned atomic integer type, supporting basic atomic arithmetic operations
pub struct AtomicNum<T> {
    v: UnsafeCell<T>,
}

impl<T> AtomicNum<T> {
    /// Create a new `AtomicUint`
    pub fn new(v: T) -> AtomicNum<T> {
        AtomicNum { v: UnsafeCell::new(v) }
    }

    /// Load the value
    #[inline]
    pub fn load(&self, order: Ordering) -> T {
        unsafe { atomic_load(self.v.get() as *const T, order) }
    }

    /// Store the value
    #[inline]
    pub fn store(&self, val: T, order: Ordering) {
        unsafe { atomic_store(self.v.get(), val, order); }
    }

    /// Store a value, returning the old value
    #[inline]
    pub fn swap(&self, val: T, order: Ordering) -> T {
        unsafe { atomic_swap(self.v.get(), val, order) }
    }

    /// If the current value is the same as expected, store a new value
    ///
    /// Compare the current value with `old`; if they are the same then
    /// replace the current value with `new`. Return the previous value.
    /// If the return value is equal to `old` then the value was updated.
    #[inline]
    pub fn compare_and_swap(&self, old: T, new: T, order: Ordering) -> T {
        unsafe { atomic_compare_and_swap(self.v.get(), old, new, order) }
    }

    /// Add to the current value, returning the previous
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::atomics::{AtomicUint, SeqCst};
    ///
    /// let foo = AtomicUint::new(0);
    /// assert_eq!(0, foo.fetch_add(10, SeqCst));
    /// assert_eq!(10, foo.load(SeqCst));
    /// ```
    #[inline]
    pub fn fetch_add(&self, val: T, order: Ordering) -> T {
        unsafe { atomic_add(self.v.get(), val, order) }
    }

    /// Subtract from the current value, returning the previous
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::atomics::{AtomicUint, SeqCst};
    ///
    /// let foo = AtomicUint::new(10);
    /// assert_eq!(10, foo.fetch_sub(10, SeqCst));
    /// assert_eq!(0, foo.load(SeqCst));
    /// ```
    #[inline]
    pub fn fetch_sub(&self, val: T, order: Ordering) -> T {
        unsafe { atomic_sub(self.v.get(), val, order) }
    }

    /// Bitwise and with the current value, returning the previous
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::atomics::{AtomicUint, SeqCst};
    ///
    /// let foo = AtomicUint::new(0b101101);
    /// assert_eq!(0b101101, foo.fetch_and(0b110011, SeqCst));
    /// assert_eq!(0b100001, foo.load(SeqCst));
    #[inline]
    pub fn fetch_and(&self, val: T, order: Ordering) -> T {
        unsafe { atomic_and(self.v.get(), val, order) }
    }

    /// Bitwise or with the current value, returning the previous
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::atomics::{AtomicUint, SeqCst};
    ///
    /// let foo = AtomicUint::new(0b101101);
    /// assert_eq!(0b101101, foo.fetch_or(0b110011, SeqCst));
    /// assert_eq!(0b111111, foo.load(SeqCst));
    #[inline]
    pub fn fetch_or(&self, val: T, order: Ordering) -> T {
        unsafe { atomic_or(self.v.get(), val, order) }
    }

    /// Bitwise xor with the current value, returning the previous
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::atomics::{AtomicUint, SeqCst};
    ///
    /// let foo = AtomicUint::new(0b101101);
    /// assert_eq!(0b101101, foo.fetch_xor(0b110011, SeqCst));
    /// assert_eq!(0b011110, foo.load(SeqCst));
    #[inline]
    pub fn fetch_xor(&self, val: T, order: Ordering) -> T {
        unsafe { atomic_xor(self.v.get(), val, order) }
    }
}


#[inline]
unsafe fn atomic_store<T>(dst: *mut T, val: T, order:Ordering) {
    match order {
        Release => intrinsics::atomic_store_rel(dst, val),
        Relaxed => intrinsics::atomic_store_relaxed(dst, val),
        _       => intrinsics::atomic_store(dst, val)
    }
}

#[inline]
unsafe fn atomic_load<T>(dst: *const T, order:Ordering) -> T {
    match order {
        Acquire => intrinsics::atomic_load_acq(dst),
        Relaxed => intrinsics::atomic_load_relaxed(dst),
        _       => intrinsics::atomic_load(dst)
    }
}

#[inline]
unsafe fn atomic_swap<T>(dst: *mut T, val: T, order: Ordering) -> T {
    match order {
        Acquire => intrinsics::atomic_xchg_acq(dst, val),
        Release => intrinsics::atomic_xchg_rel(dst, val),
        AcqRel  => intrinsics::atomic_xchg_acqrel(dst, val),
        Relaxed => intrinsics::atomic_xchg_relaxed(dst, val),
        _       => intrinsics::atomic_xchg(dst, val)
    }
}

/// Returns the old value (like __sync_fetch_and_add).
#[inline]
unsafe fn atomic_add<T>(dst: *mut T, val: T, order: Ordering) -> T {
    match order {
        Acquire => intrinsics::atomic_xadd_acq(dst, val),
        Release => intrinsics::atomic_xadd_rel(dst, val),
        AcqRel  => intrinsics::atomic_xadd_acqrel(dst, val),
        Relaxed => intrinsics::atomic_xadd_relaxed(dst, val),
        _       => intrinsics::atomic_xadd(dst, val)
    }
}

/// Returns the old value (like __sync_fetch_and_sub).
#[inline]
unsafe fn atomic_sub<T>(dst: *mut T, val: T, order: Ordering) -> T {
    match order {
            Acquire => intrinsics::atomic_xsub_acq(dst, val),
            Release => intrinsics::atomic_xsub_rel(dst, val),
            AcqRel  => intrinsics::atomic_xsub_acqrel(dst, val),
            Relaxed => intrinsics::atomic_xsub_relaxed(dst, val),
            _       => intrinsics::atomic_xsub(dst, val)
    }
}

#[inline]
unsafe fn atomic_compare_and_swap<T>(dst: *mut T, old:T, new:T, order: Ordering) -> T {
    match order {
            Acquire => intrinsics::atomic_cxchg_acq(dst, old, new),
            Release => intrinsics::atomic_cxchg_rel(dst, old, new),
            AcqRel  => intrinsics::atomic_cxchg_acqrel(dst, old, new),
            Relaxed => intrinsics::atomic_cxchg_relaxed(dst, old, new),
            _       => intrinsics::atomic_cxchg(dst, old, new),
    }
}

#[inline]
unsafe fn atomic_and<T>(dst: *mut T, val: T, order: Ordering) -> T {
    match order {
            Acquire => intrinsics::atomic_and_acq(dst, val),
            Release => intrinsics::atomic_and_rel(dst, val),
            AcqRel  => intrinsics::atomic_and_acqrel(dst, val),
            Relaxed => intrinsics::atomic_and_relaxed(dst, val),
            _       => intrinsics::atomic_and(dst, val)
    }
}

#[inline]
unsafe fn atomic_nand<T>(dst: *mut T, val: T, order: Ordering) -> T {
    match order {
            Acquire => intrinsics::atomic_nand_acq(dst, val),
            Release => intrinsics::atomic_nand_rel(dst, val),
            AcqRel  => intrinsics::atomic_nand_acqrel(dst, val),
            Relaxed => intrinsics::atomic_nand_relaxed(dst, val),
            _       => intrinsics::atomic_nand(dst, val)
    }
}


#[inline]
unsafe fn atomic_or<T>(dst: *mut T, val: T, order: Ordering) -> T {
    match order {
            Acquire => intrinsics::atomic_or_acq(dst, val),
            Release => intrinsics::atomic_or_rel(dst, val),
            AcqRel  => intrinsics::atomic_or_acqrel(dst, val),
            Relaxed => intrinsics::atomic_or_relaxed(dst, val),
            _       => intrinsics::atomic_or(dst, val)
    }
}


#[inline]
unsafe fn atomic_xor<T>(dst: *mut T, val: T, order: Ordering) -> T {
    match order {
            Acquire => intrinsics::atomic_xor_acq(dst, val),
            Release => intrinsics::atomic_xor_rel(dst, val),
            AcqRel  => intrinsics::atomic_xor_acqrel(dst, val),
            Relaxed => intrinsics::atomic_xor_relaxed(dst, val),
            _       => intrinsics::atomic_xor(dst, val)
    }
}


#[cfg(test)]
mod tests {
    use super::AtomicNum;
    use std::sync::atomic::Ordering::{SeqCst, Release, Acquire, AcqRel, Relaxed};

    #[test]
    fn test_max_store() {
        let c: AtomicNum<u64> = AtomicNum::new(0);
        c.store(18446744073709551615, SeqCst);
        let v = c.load(SeqCst);

        assert!(v == 18446744073709551615);
    }

    #[test]
    fn test_max_store_overflow() {
        let c: AtomicNum<u64> = AtomicNum::new(0);
        c.store(18446744073709551615, SeqCst);
        c.fetch_add(1, SeqCst);
        let v = c.load(SeqCst);

        assert!(v == 0);
    }
}
