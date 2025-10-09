# spincell
A small, lightweight thread-safe cell implementation targeting no_std environments.

## Features
- Small and lightweight: designed to be minimal and easy to audit.
- Targets no_std environments: uses core primitives (AtomicBool, UnsafeCell, MaybeUninit).
- Provides a Lazy/Delayed initialization API similar to LazyLock.
- Uses simple spinlock strategy to implement a thread-safe cell.

## Usage
```rust
fn main() {
    // SpinCell behaves like a LazyCell.
    let one = SpinCell::new(1usize);
    assert_eq!(1, *one);
}
```

## Future work / TODO
- Support for pluggable lock implementations (a lock_api-like abstraction). Allow users to choose different locking strategies (e.g., spin vs parking vs OS mutex) for performance and platform constraints.


## Compatibility and breaking changes
Important: starting with version 0.2.0 this crate introduces breaking changes and is not backwards compatible with the 0.1.x series. Versions up to 1.0.0 are beta releases and may contain breaking changes.


