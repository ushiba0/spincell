#![no_std]

use core::cell::UnsafeCell;
use core::mem::{ManuallyDrop, MaybeUninit};
use core::sync::atomic::{AtomicBool, Ordering};

pub struct SpinCell<T, G = fn() -> T> {
    // A simple spin lock for serialized initialization.
    lock: AtomicBool,
    // Whether the cell currently holds an initialized value.
    // Readers should load this with Acquire to observe initialized data.
    is_initialized: AtomicBool,
    cell: MaybeUninit<UnsafeCell<T>>,
    // Stored initializer function (consumed exactly once by the first
    // thread that successfully initializes). Wrapped in UnsafeCell so it
    // can be taken from &self during initialization.
    init_func: UnsafeCell<ManuallyDrop<G>>,
}

unsafe impl<T: Sync, G> Sync for SpinCell<T, G> {}
unsafe impl<T: Send, G> Send for SpinCell<T, G> {}

impl<T, G: FnOnce() -> T> SpinCell<T, G> {
    #[inline(always)]
    pub const fn new(init_func: G) -> SpinCell<T, G> {
        Self {
            lock: AtomicBool::new(false),
            is_initialized: AtomicBool::new(false),
            cell: MaybeUninit::uninit(),
            init_func: UnsafeCell::new(ManuallyDrop::new(init_func)),
        }
    }

    pub unsafe fn force_initialize(&self) {
        // Acquire the lock exclusively. Use Acquire on success so that the
        // subsequent reads/writes are properly ordered, and Relaxed on
        // failure to avoid unnecessary barriers.
        while self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }

        // If another thread initialized while we were spinning, just release
        // the lock and return.
        if self.is_initialized.load(Ordering::Acquire) {
            self.lock.store(false, Ordering::Release);
            return;
        }

        // Take the initializer and run it.
        let data = &mut *self.init_func.get();
        let init_func = ManuallyDrop::take(data);
        let value = init_func();

        let ptr = self.cell.as_ptr() as *mut UnsafeCell<T>;
        core::ptr::write(ptr, UnsafeCell::new(value));

        // Publish the initialized value. Use Release so readers that do an
        // Acquire load on `is_initialized` see the written data.
        self.is_initialized.store(true, Ordering::Release);

        // Release the lock.
        self.lock.store(false, Ordering::Release);
    }

    pub fn try_initialize(me: &SpinCell<T, G>) -> Result<(), ()> {
        // Lock SpinCell.
        // Fast path: if already initialized, return Err.
        if me.is_initialized.load(Ordering::Acquire) {
            return Err(());
        }

        // Not initialized.
        // `force_initialize` acquires the internal lock and re-checks the
        // initialized flag to ensure only one thread runs the initializer.
        unsafe {
            me.force_initialize();
        }
        Ok(())
    }
}

impl<T, G: FnOnce() -> T> core::ops::Deref for SpinCell<T, G> {
    type Target = T;
    fn deref(&self) -> &T {
        match SpinCell::try_initialize(self) {
            Ok(()) => {}  // Called force_initialize().
            Err(()) => {} // Cell has already been initilized.
        }
        unsafe { &*self.cell.assume_init_ref().get() }
    }
}

impl<T, G> Drop for SpinCell<T, G> {
    fn drop(&mut self) {
        // If the cell was initialized, drop the inner T in-place.
        if self.is_initialized.load(Ordering::Acquire) {
            let cell_ptr = self.cell.as_mut_ptr() as *mut UnsafeCell<T>;
            unsafe {
                // Safety: we have &mut self so there are no other references
                // to the contained T; drop it in-place.
                core::ptr::drop_in_place((*cell_ptr).get());
            }
        } else {
            // The cell was not initialized: the initializer is still
            // present and must be dropped. We have exclusive access via
            // &mut self, so it's safe to drop the ManuallyDrop<G>.
            unsafe {
                ManuallyDrop::drop(&mut *self.init_func.get());
            }
        }
    }
}
