use crossbeam::queue::ArrayQueue;
use crate::domain::entities::Event;
use crate::error::{AllSourceError, Result};
use std::sync::Arc;

/// Lock-free bounded event queue for high-throughput ingestion
///
/// # Design Pattern
/// Uses crossbeam's lock-free MPMC (Multi-Producer, Multi-Consumer) queue
/// to eliminate contention in the hot path of event ingestion.
///
/// # Benefits
/// - **Zero Lock Contention**: Multiple threads can push/pop concurrently
/// - **Predictable Latency**: No waiting for locks
/// - **High Throughput**: Optimized for concurrent access
/// - **Backpressure Handling**: Returns error when full
///
/// # Performance
/// - Push: ~10-20ns (lock-free)
/// - Pop: ~10-20ns (lock-free)
/// - vs RwLock: 100-500ns (with contention)
///
/// # Example
/// ```ignore
/// let queue = LockFreeEventQueue::new(10000);
///
/// // Multiple producers can push concurrently
/// let event = Event::from_strings(...)?;
/// queue.try_push(event)?;
///
/// // Multiple consumers can pop concurrently
/// if let Some(event) = queue.try_pop() {
///     // Process event
/// }
/// ```
#[derive(Clone)]
pub struct LockFreeEventQueue {
    queue: Arc<ArrayQueue<Event>>,
    capacity: usize,
}

impl LockFreeEventQueue {
    /// Create new lock-free event queue with fixed capacity
    ///
    /// # Arguments
    /// - `capacity`: Maximum number of events the queue can hold
    ///
    /// # Capacity Guidelines
    /// - Small: 1,000-10,000 events (low memory, fast overflow)
    /// - Medium: 10,000-100,000 events (balanced)
    /// - Large: 100,000-1,000,000 events (high memory, slow overflow)
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: Arc::new(ArrayQueue::new(capacity)),
            capacity,
        }
    }

    /// Try to push an event to the queue (non-blocking)
    ///
    /// Returns an error if the queue is full. Callers should implement
    /// backpressure handling (e.g., retry with exponential backoff, or
    /// return HTTP 503 Service Unavailable).
    ///
    /// # Performance
    /// - Lock-free operation (~10-20ns)
    /// - No waiting on contention
    /// - Constant time O(1)
    pub fn try_push(&self, event: Event) -> Result<()> {
        self.queue
            .push(event)
            .map_err(|_| AllSourceError::QueueFull(
                format!("Event queue at capacity ({})", self.capacity)
            ))
    }

    /// Try to pop an event from the queue (non-blocking)
    ///
    /// Returns `None` if the queue is empty.
    ///
    /// # Performance
    /// - Lock-free operation (~10-20ns)
    /// - No waiting on contention
    /// - Constant time O(1)
    pub fn try_pop(&self) -> Option<Event> {
        self.queue.pop()
    }

    /// Get current queue length
    ///
    /// Note: This is approximate in concurrent scenarios due to race
    /// conditions between length check and actual push/pop operations.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Check if queue is empty
    ///
    /// Note: Result may be stale in concurrent scenarios.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Check if queue is full
    ///
    /// Note: Result may be stale in concurrent scenarios.
    pub fn is_full(&self) -> bool {
        self.queue.len() == self.capacity
    }

    /// Get queue capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get approximate fill percentage (0.0 to 1.0)
    pub fn fill_ratio(&self) -> f64 {
        self.len() as f64 / self.capacity as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    fn create_test_event(id: u32) -> Event {
        Event::from_strings(
            "test.event".to_string(),
            format!("entity-{}", id),
            "default".to_string(),
            json!({"id": id}),
            None,
        )
        .unwrap()
    }

    #[test]
    fn test_create_queue() {
        let queue = LockFreeEventQueue::new(100);
        assert_eq!(queue.capacity(), 100);
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
        assert!(!queue.is_full());
    }

    #[test]
    fn test_push_and_pop() {
        let queue = LockFreeEventQueue::new(10);

        let event = create_test_event(1);
        queue.try_push(event.clone()).unwrap();

        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());

        let popped = queue.try_pop().unwrap();
        assert_eq!(popped.entity_id(), event.entity_id());
        assert!(queue.is_empty());
    }

    #[test]
    fn test_queue_full() {
        let queue = LockFreeEventQueue::new(3);

        // Fill queue
        queue.try_push(create_test_event(1)).unwrap();
        queue.try_push(create_test_event(2)).unwrap();
        queue.try_push(create_test_event(3)).unwrap();

        assert!(queue.is_full());

        // Try to push when full
        let result = queue.try_push(create_test_event(4));
        assert!(result.is_err());
        assert!(matches!(result, Err(AllSourceError::QueueFull(_))));
    }

    #[test]
    fn test_pop_empty_queue() {
        let queue = LockFreeEventQueue::new(10);
        assert!(queue.try_pop().is_none());
    }

    #[test]
    fn test_multiple_push_pop() {
        let queue = LockFreeEventQueue::new(100);

        // Push 10 events
        for i in 0..10 {
            queue.try_push(create_test_event(i)).unwrap();
        }

        assert_eq!(queue.len(), 10);

        // Pop all events
        let mut count = 0;
        while queue.try_pop().is_some() {
            count += 1;
        }

        assert_eq!(count, 10);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_fill_ratio() {
        let queue = LockFreeEventQueue::new(100);

        assert_eq!(queue.fill_ratio(), 0.0);

        for i in 0..50 {
            queue.try_push(create_test_event(i)).unwrap();
        }

        assert_eq!(queue.fill_ratio(), 0.5);

        for i in 50..100 {
            queue.try_push(create_test_event(i)).unwrap();
        }

        assert_eq!(queue.fill_ratio(), 1.0);
    }

    #[test]
    fn test_concurrent_producers() {
        let queue = LockFreeEventQueue::new(10000);
        let queue_clone1 = queue.clone();
        let queue_clone2 = queue.clone();

        let handle1 = thread::spawn(move || {
            for i in 0..1000 {
                let _ = queue_clone1.try_push(create_test_event(i));
            }
        });

        let handle2 = thread::spawn(move || {
            for i in 1000..2000 {
                let _ = queue_clone2.try_push(create_test_event(i));
            }
        });

        handle1.join().unwrap();
        handle2.join().unwrap();

        // Should have approximately 2000 events (some may have been lost if queue was full)
        let final_len = queue.len();
        assert!(final_len >= 1900 && final_len <= 2000);
    }

    #[test]
    fn test_concurrent_producers_and_consumers() {
        let queue = LockFreeEventQueue::new(1000);
        let produced = Arc::new(AtomicUsize::new(0));
        let consumed = Arc::new(AtomicUsize::new(0));

        // Producer thread
        let queue_prod = queue.clone();
        let produced_clone = produced.clone();
        let producer = thread::spawn(move || {
            for i in 0..500 {
                while queue_prod.try_push(create_test_event(i)).is_err() {
                    // Retry if queue is full
                    thread::yield_now();
                }
                produced_clone.fetch_add(1, Ordering::Relaxed);
            }
        });

        // Consumer thread
        let queue_cons = queue.clone();
        let consumed_clone = consumed.clone();
        let consumer = thread::spawn(move || {
            let mut count = 0;
            while count < 500 {
                if queue_cons.try_pop().is_some() {
                    count += 1;
                    consumed_clone.fetch_add(1, Ordering::Relaxed);
                } else {
                    thread::yield_now();
                }
            }
        });

        producer.join().unwrap();
        consumer.join().unwrap();

        assert_eq!(produced.load(Ordering::Relaxed), 500);
        assert_eq!(consumed.load(Ordering::Relaxed), 500);
        assert!(queue.is_empty());
    }
}
