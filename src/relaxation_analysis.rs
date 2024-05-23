use std::collections::VecDeque;

use crate::relaxed_fifo::RelaxedFifo;

/// Analyze a relaxed queue (passed empty), returning all rank errors for the operations
pub fn analyze(
    empty_queue: impl RelaxedFifo<usize>,
    prefill: usize,
    operations: &Vec<bool>,
) -> Vec<usize> {
    // Keep an ordered queue to the side
    let mut strict_queue = StrictQueue::new();

    // Keep empty in parameter name to make it super clear
    let mut relaxed_queue = empty_queue;

    for item in 0..prefill {
        // Prefill
        strict_queue.enqueue(item);
        relaxed_queue.enqueue(item);
    }

    let mut rank_errors = vec![];
    let mut enq_nbr = prefill;

    for op in operations {
        if *op {
            // Enqueue
            strict_queue.enqueue(enq_nbr);
            relaxed_queue.enqueue(enq_nbr);
            enq_nbr += 1;
        } else {
            // Dequeue
            if let Some(item) = relaxed_queue.dequeue() {
                rank_errors.push(strict_queue.relaxed_dequeue(item));
            } else {
                // Treat empty returns as real operations (some queues might not be empty linearizable)
                rank_errors.push(strict_queue.len());
            }
        }
    }

    rank_errors
}

struct StrictQueue {
    deque: VecDeque<(usize, bool)>,
    len: usize,
}

impl StrictQueue {
    fn new() -> Self {
        Self {
            deque: VecDeque::new(),
            len: 0,
        }
    }

    fn enqueue(&mut self, item: usize) {
        self.deque.push_back((item, true));
        self.len += 1;
    }

    /// Returns the relaxation distance of the dequeued item
    fn relaxed_dequeue(&mut self, item: usize) -> usize {
        // Always decrease len by 1 when dequeueing an item
        assert!(self.len > 0, "Cannot dequeue from an empty strict queue");
        self.len -= 1;

        if self.deque.front().expect("Item must exist").0 == item {
            // Relaxation error is 0, and we can empty the deque
            self.deque.pop_front();
            loop {
                match self.deque.front() {
                    Some((_item, false)) => self.deque.pop_front(),
                    _ => return 0,
                };
            }
        } else {
            // The item is not first, so don't have to worry about removing old garbage
            let mut rank_error = 0;
            for (other, exists) in self.deque.iter_mut() {
                if *other == item {
                    *exists = false;
                    return rank_error;
                } else if *exists {
                    rank_error += 1;
                }
            }
            panic!("Could not find dequeued item")
        }
    }

    /// Returns the number of live items in the queue
    fn len(&mut self) -> usize {
        self.len
    }
}
