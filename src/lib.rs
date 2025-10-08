use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, Ordering};

pub struct SpinCell<T: Sized> {
    lock: AtomicBool,
    is_initialized: AtomicBool,
    cell: MaybeUninit<UnsafeCell<T>>,
}

unsafe impl<T: Sized + Sync> Sync for SpinCell<T> {}
unsafe impl<T: Sized + Send> Send for SpinCell<T> {}

impl<T: Sized> SpinCell<T> {
    #[inline(always)]
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            is_initialized: AtomicBool::new(true),
            cell: MaybeUninit::new(UnsafeCell::new(data)),
        }
    }

    #[inline(always)]
    pub const fn uninit() -> Self {
        Self {
            lock: AtomicBool::new(false),
            is_initialized: AtomicBool::new(false),
            cell: MaybeUninit::uninit(),
        }
    }

    pub unsafe fn force_initialize<F>(&self, init_func: F)
    where
        F: FnOnce() -> T,
    {
        // Acquire the lock. Use compare_exchange to only write when the
        // lock was previously false; on contention use a relaxed failure
        // ordering to avoid unnecessary costs.
        while self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }

        // If initialized, drop the value in-place without moving the MaybeUninit
        // Swap to false and observe the previous value. Use AcqRel so the
        // operation acts as an acquire for observing the initialized data
        // (if it was true) and as a release for our write of `false`.
        if self.is_initialized.swap(false, Ordering::AcqRel) {
            let cell_ptr = self.cell.as_ptr() as *mut UnsafeCell<T>;
            unsafe {
                // Safety: we have exclusive access because we hold the lock,
                // and we only drop if it was initialized. We cannot call the
                // consuming `assume_init()` because `self` is borrowed; instead
                // drop the inner `T` in-place via raw pointers.
                core::ptr::drop_in_place((*cell_ptr).get());
            }
        }

        // Reinitialize with new value from closure
        let newcell = UnsafeCell::new(init_func());
        let ptr = self.cell.as_ptr() as *mut UnsafeCell<T>;
        unsafe {
            // Write the new UnsafeCell<T> into the MaybeUninit slot.
            core::ptr::write(ptr, newcell);
        }
        self.is_initialized.store(true, Ordering::Release);

        // Release the lock
        self.lock.store(false, Ordering::Release);
    }

    pub fn try_initialize<F>(&self, init_func: F) -> Result<(), ()>
    where
        F: FnOnce() -> T,
    {
        // If already initialized, return Err. This check is performed with
        // Acquire so that a true value implies the data is visible.
        if self.is_initialized.load(Ordering::Acquire) {
            return Err(());
        }

        // Not initialized (as far as we observed) — perform initialization.
        // `force_initialize` is unsafe, so call it inside an unsafe block.
        unsafe { self.force_initialize(init_func) };
        Ok(())
    }
}

impl<T: Sized> core::ops::Deref for SpinCell<T> {
    type Target = T;
    fn deref(&self) -> &T {
        assert!(
            self.is_initialized.load(Ordering::Acquire),
            "SpinCell is not initialized yet."
        );
        unsafe { &*self.cell.assume_init_ref().get() }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_usize_calc() {
        {
            let one = SpinCell::new(1usize);
            assert_eq!(1, *one);

            let two = one.wrapping_add(1);
            assert_eq!(two, 2);
        }
    }

    #[test]
    fn test_usize_uninit() {
        let uninit: SpinCell<usize> = SpinCell::uninit();
        unsafe {
            uninit.force_initialize(|| 1usize);
        }
        assert_eq!(*uninit, 1);
    }

    #[test]
    fn test_string() {
        let message = SpinCell::new(String::from("test message"));
        assert_eq!(*message, "test message");
    }

    #[test]
    fn test_try_init() {
        let uninit: SpinCell<usize> = SpinCell::uninit();
        uninit.try_initialize(|| 2).unwrap();
        assert_eq!(*uninit, 2);
    }
}

#[cfg(test)]
mod droptest {
    use super::*;

    #[cfg(test)]
    static CALL_COUNTER: SpinCell<usize> = SpinCell::new(0);
    #[cfg(test)]
    struct DropTest {}

    impl Drop for DropTest {
        fn drop(&mut self) {
            unsafe {
                CALL_COUNTER.force_initialize(|| 1);
            }
            println!("Struct DropTest has been dropped.");
        }
    }

    #[test]
    fn test_drop() {
        assert_eq!(*CALL_COUNTER, 0);
        {
            let _ = DropTest {};
        }
        assert_eq!(*CALL_COUNTER, 1);
    }
}
