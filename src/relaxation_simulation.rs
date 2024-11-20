use std::collections::VecDeque;

use crate::{relaxed_fifo::RelaxedFifo, DChoiceQueue};

/// Analyze a relaxed queue (passed empty), returning all rank errors for the operations
pub fn analyze_simple(
    relaxed_queue: &mut impl RelaxedFifo<usize>,
    prefill: usize,
    operations: &Vec<bool>,
) -> Vec<usize> {
    // Keep an ordered queue to the side
    let mut strict_queue = StrictQueue::new();

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

/// Keeps extra information about each dequeue, apart from just its rank error
pub enum ErrorTag {
    ItemDequeue {
        /// The rank error of the dequeued item
        rank_error: usize,

        /// The enqueue operation this was enqueued during (for average load calc)
        enq_nbr: usize,

        /// The dequeue operation this was dequeued during, not including empty returns
        deq_nbr: usize,

        /// The position this was enqueued at in the sub-queue
        sub_nbr: usize,
    },

    EmptyDequeue {
        /// Corresponds to how many items there were at the time of dequeue
        rank_error: usize,

        /// The dequeue operation this was dequeued during, not including empty returns
        deq_nbr: usize,

        /// The position in a sub-queue that was attempted to dequeue from
        sub_nbr: usize,
    },
}

impl ErrorTag {
    pub fn rank_error(&self) -> usize {
        match self {
            ErrorTag::ItemDequeue { rank_error, .. } => *rank_error,
            ErrorTag::EmptyDequeue { rank_error, .. } => *rank_error,
        }
    }

    pub fn deq_nbr(&self) -> usize {
        match self {
            ErrorTag::ItemDequeue { deq_nbr, .. } => *deq_nbr,
            ErrorTag::EmptyDequeue { deq_nbr, .. } => *deq_nbr,
        }
    }

    pub fn sub_nbr(&self) -> usize {
        match self {
            ErrorTag::ItemDequeue { sub_nbr, .. } => *sub_nbr,
            ErrorTag::EmptyDequeue { sub_nbr, .. } => *sub_nbr,
        }
    }
}

/// Analyze a relaxed queue (passed empty), returning rank error and extra information for all dequeues
pub fn analyze_extra(
    relaxed_queue: &mut DChoiceQueue<usize>,
    prefill: usize,
    operations: &Vec<bool>,
) -> Vec<ErrorTag> {
    // Keep an ordered queue to the side
    let mut strict_queue = StrictQueue::new();

    for item in 0..prefill {
        // Prefill
        strict_queue.enqueue(item);
        relaxed_queue.enqueue(item);
    }

    let mut error_tags = vec![];
    let mut enq_nbr = prefill;

    let mut deq_nbr = 0;
    for op in operations {
        if *op {
            // Enqueue
            strict_queue.enqueue(enq_nbr);
            relaxed_queue.enqueue(enq_nbr);
            enq_nbr += 1;
        } else {
            // Dequeue
            deq_nbr += 1;
            match relaxed_queue.dequeue_with_info() {
                (Some(item), sub_nbr) => error_tags.push(ErrorTag::ItemDequeue {
                    rank_error: strict_queue.relaxed_dequeue(item),
                    enq_nbr: item,
                    deq_nbr,
                    sub_nbr,
                }),
                (None, sub_nbr) => error_tags.push(ErrorTag::EmptyDequeue {
                    rank_error: strict_queue.len(),
                    deq_nbr,
                    sub_nbr,
                }),
            }
        }
    }

    error_tags
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
