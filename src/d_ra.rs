use rand::{seq::SliceRandom, Rng};
use std::collections::VecDeque;

use crate::relaxed_fifo::RelaxedFifo;

// Singlethreaded implementation of d-Ra
pub struct DRa<T: PartialEq + Eq> {
    /// The partial queues
    partials: Vec<Partial<T>>,

    /// How many partials to sample per operation
    d: usize,

    /// If true, cannot sample the same partial several times for one d-choice
    uniques: bool,
}

struct Partial<T: PartialEq + Eq> {
    head: usize,
    tail: usize,
    fifo: VecDeque<T>,
}

impl<T: PartialEq + Eq> DRa<T> {
    /// Enqueues an item into the D-Ra
    pub fn enqueue(&mut self, item: T) {
        let partial_ind = if self.uniques {
            (0..self.partials.len())
                .collect::<Vec<usize>>()
                .choose_multiple(&mut rand::thread_rng(), self.d)
                .cloned()
                .min_by_key(|ind| self.partials[*ind].tail)
                .expect("Should always be able to find an index if d>0")
        } else {
            (0..self.d)
                .map(|_| rand::thread_rng().gen_range(0..self.partials.len()))
                .min_by_key(|ind| self.partials[*ind].tail)
                .expect("Should always be able to find an index if d>0")
        };

        // TODO: Add option for length-based vs progress-based heuristic

        self.partials[partial_ind].enqueue(item);
    }

    pub fn dequeue(&mut self) -> Option<T> {
        let partial_ind = if self.uniques {
            (0..self.partials.len())
                .collect::<Vec<usize>>()
                .choose_multiple(&mut rand::thread_rng(), self.d)
                .cloned()
                .min_by_key(|ind| self.partials[*ind].head)
                .expect("Should always be able to find an index if d>0")
        } else {
            (0..self.d)
                .map(|_| rand::thread_rng().gen_range(0..self.partials.len()))
                .min_by_key(|ind| self.partials[*ind].head)
                .expect("Should always be able to find an index if d>0")
        };

        // TODO: Add a switch to toggle empty linearizable
        // TODO: Add option for length-based vs progress-based heuristic

        self.partials[partial_ind].dequeue()
    }
}

impl<T: PartialEq + Eq> Partial<T> {
    fn enqueue(&mut self, item: T) {
        self.tail += 1;
        self.fifo.push_back(item)
    }

    fn dequeue(&mut self) -> Option<T> {
        self.head += 1;
        self.fifo.pop_front()
    }
}

impl<T: PartialEq + Eq> RelaxedFifo<T> for DRa<T> {
    fn enqueue(&mut self, item: T) {
        self.enqueue(item)
    }

    fn dequeue(&mut self) -> Option<T> {
        self.dequeue()
    }
}
