#![no_std]
use spincell::SpinCell;

#[test]
fn test_one_plus_one() {
    let one = SpinCell::new(|| 1u8);
    assert_eq!(1, *one);
    let two = one.wrapping_add(1);
    assert_eq!(two, 2);
}

#[cfg(test)]
mod droptest {
    use core::sync::atomic::{AtomicUsize, Ordering};

    use spincell::SpinCell;
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    struct DropTest {
        my_data: usize,
    }

    impl Drop for DropTest {
        fn drop(&mut self) {
            COUNTER.store(1, Ordering::Release);
        }
    }

    #[test]
    fn test_drop() {
        assert_eq!(COUNTER.load(Ordering::Acquire), 0);
        {
            let data = SpinCell::new(|| DropTest { my_data: 123 });
            assert_eq!(data.my_data, 123);
        }
        assert_eq!(COUNTER.load(Ordering::Acquire), 1);
    }
}
