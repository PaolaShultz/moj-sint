use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

pub const QUEUE_CAPACITY: usize = 1024;

struct Shared<T: Copy> {
    slots: [UnsafeCell<MaybeUninit<T>>; QUEUE_CAPACITY],
    read: AtomicUsize,
    write: AtomicUsize,
    overflow: AtomicU64,
}

unsafe impl<T: Copy + Send> Send for Shared<T> {}
unsafe impl<T: Copy + Send> Sync for Shared<T> {}

pub struct Producer<T: Copy>(Arc<Shared<T>>);
pub struct Consumer<T: Copy>(Arc<Shared<T>>);

pub fn channel<T: Copy>() -> (Producer<T>, Consumer<T>) {
    let shared = Arc::new(Shared {
        slots: std::array::from_fn(|_| UnsafeCell::new(MaybeUninit::uninit())),
        read: AtomicUsize::new(0),
        write: AtomicUsize::new(0),
        overflow: AtomicU64::new(0),
    });
    (Producer(shared.clone()), Consumer(shared))
}

impl<T: Copy> Producer<T> {
    pub fn push(&self, value: T) -> bool {
        let write = self.0.write.load(Ordering::Relaxed);
        let next = (write + 1) % QUEUE_CAPACITY;
        if next == self.0.read.load(Ordering::Acquire) {
            self.0.overflow.fetch_add(1, Ordering::Relaxed);
            return false;
        }
        // SAFETY: this SPSC producer is the only writer and never writes the
        // slot still owned by the consumer.
        unsafe { (*self.0.slots[write].get()).write(value) };
        self.0.write.store(next, Ordering::Release);
        true
    }

    pub fn overflow_count(&self) -> u64 {
        self.0.overflow.load(Ordering::Relaxed)
    }
}

impl<T: Copy> Consumer<T> {
    pub fn pop(&self) -> Option<T> {
        let read = self.0.read.load(Ordering::Relaxed);
        if read == self.0.write.load(Ordering::Acquire) {
            return None;
        }
        // SAFETY: acquire observed the producer's initialized slot and the
        // consumer is its only reader.
        let value = unsafe { (*self.0.slots[read].get()).assume_init_read() };
        self.0
            .read
            .store((read + 1) % QUEUE_CAPACITY, Ordering::Release);
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifo_is_bounded_and_counts_overflow_without_blocking() {
        let (producer, consumer) = channel();
        for value in 0..QUEUE_CAPACITY - 1 {
            assert!(producer.push(value));
        }
        assert!(!producer.push(9999));
        assert_eq!(producer.overflow_count(), 1);
        for expected in 0..QUEUE_CAPACITY - 1 {
            assert_eq!(consumer.pop(), Some(expected));
        }
        assert_eq!(consumer.pop(), None);
    }

    #[test]
    fn producer_and_consumer_transfer_across_threads() {
        let (producer, consumer) = channel();
        let writer = std::thread::spawn(move || {
            for value in 0..100_u32 {
                while !producer.push(value) {
                    std::hint::spin_loop();
                }
            }
        });
        let mut received = Vec::new();
        while received.len() < 100 {
            if let Some(value) = consumer.pop() {
                received.push(value);
            } else {
                std::thread::yield_now();
            }
        }
        writer.join().unwrap();
        assert_eq!(received, (0..100).collect::<Vec<_>>());
    }
}
