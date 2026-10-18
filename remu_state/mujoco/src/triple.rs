//! Lock-free single-writer/single-reader triple buffer: the simulator thread
//! publishes the latest state, the render thread reads it, and neither side
//! ever blocks.

use core::cell::{Cell, UnsafeCell};
use core::sync::atomic::{AtomicU8, Ordering};

/// Slot-index field of the state word.
const IDX_MASK: u8 = 0b11;
/// State word value: nothing published yet.
const EMPTY: u8 = IDX_MASK;
/// State word flag: the published slot has not been taken yet.
const FRESH: u8 = 0b100;

pub struct TripleBuffer<T> {
    slots: [UnsafeCell<T>; 3],
    /// `published slot index | FRESH`.
    state: AtomicU8,
    /// Slot owned by the writer; never observed by the reader.
    write: Cell<usize>,
    /// Slot owned by the reader; never written by the writer.
    read: Cell<usize>,
}

// Safety: `new` fixates the ownership split (writer 0, reader 2, nothing
// published), and every later transfer goes through `swap` on `state`: the
// writer trades its freshly published slot for the previous ready one, the
// reader trades the published slot for the one it holds, and `EMPTY` can only
// occur in the very first publish, which sends the writer to the third slot.
// Ownership therefore stays disjoint, and the AcqRel/Acquire orderings pair
// every slot write with the read that observes it.
unsafe impl<T: Send> Send for TripleBuffer<T> {}
unsafe impl<T: Send> Sync for TripleBuffer<T> {}

impl<T: Default> TripleBuffer<T> {
    pub fn new() -> Self {
        Self {
            slots: core::array::from_fn(|_| UnsafeCell::new(T::default())),
            state: AtomicU8::new(EMPTY),
            write: Cell::new(0),
            read: Cell::new(2),
        }
    }
}

impl<T: Default> Default for TripleBuffer<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TripleBuffer<T> {
    /// Replace the published value; never blocks.
    pub fn publish(&self, f: impl FnOnce(&mut T)) {
        let w = self.write.get();
        // Safety: the writer exclusively owns the write slot until the swap
        // below, and the closure cannot outlive that window.
        f(unsafe { &mut *self.slots[w].get() });
        let old = self.state.swap(w as u8 | FRESH, Ordering::AcqRel);
        let old_ready = (old & IDX_MASK) as usize;
        self.write.set(if old_ready == EMPTY as usize {
            (w + 1) % 3
        } else {
            old_ready
        });
    }

    /// Borrow the latest published value; never blocks, and re-returns the
    /// same slot until the writer publishes again.
    pub fn read(&self) -> &T {
        loop {
            let s = self.state.load(Ordering::Acquire);
            if s & FRESH == 0 {
                break;
            }
            // Take the published slot and hand the current one back.
            if self
                .state
                .compare_exchange_weak(
                    s,
                    self.read.get() as u8,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                self.read.set((s & IDX_MASK) as usize);
                break;
            }
        }
        // Safety: the reader exclusively owns the read slot (see Send/Sync).
        unsafe { &*self.slots[self.read.get()].get() }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;

    use super::*;

    #[test]
    fn read_returns_the_latest_publish() {
        let tb = TripleBuffer::<u32>::new();
        assert_eq!(*tb.read(), 0); // default, nothing published yet
        tb.publish(|v| *v = 1);
        assert_eq!(*tb.read(), 1);
        assert_eq!(*tb.read(), 1); // unchanged without a new publish
        tb.publish(|v| *v = 2);
        tb.publish(|v| *v = 3);
        assert_eq!(*tb.read(), 3); // intermediate frames may be skipped
    }

    #[test]
    fn concurrent_publishes_never_tear_or_rewind() {
        let tb = Arc::new(TripleBuffer::<[u64; 2]>::new());
        let writer = {
            let tb = Arc::clone(&tb);
            thread::spawn(move || {
                for i in 1..=100_000u64 {
                    tb.publish(|v| *v = [i, i]);
                }
            })
        };
        let mut last = 0u64;
        for _ in 0..100_000 {
            let v = tb.read();
            assert_eq!(v[0], v[1], "torn value");
            assert!(v[0] >= last, "rewound from {last} to {}", v[0]);
            last = v[0];
        }
        writer.join().unwrap();
    }
}
