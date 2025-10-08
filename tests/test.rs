#![no_std]

use spincell::SpinCell;
#[test]
fn test_usize_calc() {
    let one = SpinCell::new(1usize);
    assert_eq!(1, *one);

    let two = one.wrapping_add(1);
    assert_eq!(two, 2);
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
fn test_try_init() {
    let uninit: SpinCell<usize> = SpinCell::uninit();
    uninit.try_initialize(|| 2).unwrap();
    assert_eq!(*uninit, 2);
}

#[cfg(test)]
mod droptest {
    use spincell::SpinCell;
    #[cfg(test)]
    static CALL_COUNTER: SpinCell<usize> = SpinCell::new(0);
    #[cfg(test)]
    struct DropTest {}

    impl Drop for DropTest {
        fn drop(&mut self) {
            unsafe {
                CALL_COUNTER.force_initialize(|| 1);
            }
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
