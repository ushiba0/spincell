# spincell
A small, lightweight thread-safe cell implementation targeting no_std environments.

## Features
- Small and lightweight: designed to be minimal and easy to audit.
- Targets no_std environments: uses core primitives (AtomicBool, UnsafeCell, MaybeUninit).
- Provides a Lazy/Delayed initialization API similar to LazyLock.
- Uses a spinlock to implement a thread-safe cell.

## Usage
```rust
fn main() {
    // SpinCell behaves like a normal cell.
    let one = SpinCell::new(1usize);
    assert_eq!(1, *one);

    // Supports lazy initialization.
    let lazy_init_one = SpinCell::uninit();
    // try_initialize accepts a closure that constructs T, returns Err(()) if
    // the cell was already initialized.
    lazy_init_one.try_initialize(|| 1).unwrap();
    assert_eq!(1, *lazy_init_one);

    // Supports containing structs.
    let message = SpinCell::new(String::from("Hi!"));
    assert_eq!(*message, "Hi!");

    // Accessing an uninitialized cell will panic.
    let uninit: SpinCell<usize> = SpinCell::uninit();
    let _ = *uninit; // Panic!
}
```

## Future work / TODO
- Improve TOCTOU behavior during initialization. Current try_initialize / force_initialize interactions can lead to races where a thread observes the cell as uninitialized and another thread initializes it concurrently. Possible fixes include using an atomic compare-exchange to acquire initialization rights, using Arc-based publish-swap, or employing an epoch/RCU-style reclamation mechanism.
- Support for pluggable lock implementations (a lock_api-like abstraction). Allow users to choose different locking strategies (e.g., spin vs parking vs OS mutex) for performance and platform constraints.
