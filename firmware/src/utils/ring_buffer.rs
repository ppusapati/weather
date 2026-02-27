/// Lock-free SPSC (Single Producer, Single Consumer) ring buffer.
///
/// Used for passing WeatherReadings between cores:
/// - Core 0 (producer): sensor acquisition
/// - Core 1 (consumer): communication tasks
///
/// Uses atomic operations for thread safety without locks.

use core::sync::atomic::{AtomicUsize, Ordering};

/// Fixed-capacity ring buffer for inter-core communication.
pub struct RingBuffer<T, const N: usize> {
    buffer: [Option<T>; N],
    head: AtomicUsize,
    tail: AtomicUsize,
}

// We need this because the array of Option<T> doesn't implement Default
// in a const context, so we provide a helper.
impl<T: Clone + Default, const N: usize> RingBuffer<T, N> {
    /// Create a new empty ring buffer.
    pub fn new() -> Self {
        Self {
            buffer: core::array::from_fn(|_| None),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Push an item into the buffer. Returns false if the buffer is full.
    pub fn push(&mut self, item: T) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let next_head = (head + 1) % N;

        if next_head == self.tail.load(Ordering::Acquire) {
            return false; // Buffer full
        }

        self.buffer[head] = Some(item);
        self.head.store(next_head, Ordering::Release);
        true
    }

    /// Pop an item from the buffer. Returns None if empty.
    pub fn pop(&mut self) -> Option<T> {
        let tail = self.tail.load(Ordering::Relaxed);

        if tail == self.head.load(Ordering::Acquire) {
            return None; // Buffer empty
        }

        let item = self.buffer[tail].take();
        self.tail.store((tail + 1) % N, Ordering::Release);
        item
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Relaxed) == self.tail.load(Ordering::Relaxed)
    }

    /// Check if the buffer is full.
    pub fn is_full(&self) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);
        (head + 1) % N == tail
    }

    /// Get the number of items in the buffer.
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);
        if head >= tail {
            head - tail
        } else {
            N - tail + head
        }
    }

    /// Get the capacity of the buffer.
    pub fn capacity(&self) -> usize {
        N - 1
    }

    /// Clear all items from the buffer.
    pub fn clear(&mut self) {
        while self.pop().is_some() {}
    }
}
