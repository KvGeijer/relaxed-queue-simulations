use std::collections::VecDeque;

use crate::relaxed_fifo::RelaxedFifo;

fn analyze(mut empty_queue: impl RelaxedFifo<usize>, prefill: usize, operations: Vec<bool>) {
    // Keep an ordered queue to the side
    let mut strict_queue = StrictQueue::new();

    // Keep empty in parameter name to make it super clear
    let mut relaxed_queue = empty_queue;

    for item in 0..prefill {
        // Prefill
        strict_queue.enqueue(item);
        relaxed_queue.enqueue(item);
    }

    for op in operations {
        if op { // Enqueue
        } else { // Dequeue
        }
    }
}

struct StrictQueue {
    deque: VecDeque<(usize, bool)>,
}

impl StrictQueue {
    fn new() -> Self {
        Self {
            deque: VecDeque::new(),
        }
    }

    fn enqueue(&mut self, item: usize) {
        self.deque.push_back((item, true));
    }

    /// Returns this relaxation distance of the dequeued item
    fn relaxed_dequeue(&mut self, item: usize) -> usize {
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
}
