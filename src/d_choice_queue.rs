use rand::{seq::SliceRandom, Rng};
use std::collections::VecDeque;

use crate::relaxed_fifo::RelaxedFifo;

// Singlethreaded implementation of a d-Choice relaxed queue
pub struct DChoiceQueue<T: PartialEq + Eq> {
    /// The sub-queues
    subqueues: Vec<SubQueue<T>>,

    /// How many subqueues to sample per operation
    d: usize,

    /// If true, cannot sample the same sub-queue several times for one d-choice
    uniques: bool,

    /// If true, uses the new operation-based heuristic, otherwise the length-based
    progress_heuristic: bool,

    /// If true, uses round robin when finding an empty queue
    empty_lin: bool,
}

impl<T: PartialEq + Eq> DChoiceQueue<T> {
    pub fn new(
        nbr_subqueues: usize,
        d: usize,
        uniques: bool,
        progress_heuristic: bool,
        empty_lin: bool,
    ) -> Self {
        Self {
            subqueues: (0..nbr_subqueues).map(|_| SubQueue::new()).collect(),
            d,
            uniques,
            progress_heuristic,
            empty_lin,
        }
    }

    /// Enqueues an item into the queue
    pub fn enqueue(&mut self, item: T) {
        // Find subqueue ind, depending on heuristic used (this is not super optimized)
        let subqueue_ind = if self.progress_heuristic {
            self.subqueue_inds()
                .into_iter()
                .min_by_key(|ind| self.subqueues[*ind].tail)
        } else {
            self.subqueue_inds()
                .into_iter()
                .min_by_key(|ind| self.subqueues[*ind].tail - self.subqueues[*ind].head)
        }
        .expect("Should always be able to find an index if d>0");

        self.subqueues[subqueue_ind].enqueue(item);
    }

    pub fn dequeue(&mut self) -> Option<T> {
        // Find subqueue ind, depending on heuristic used (this is not super optimized)
        let subqueue_ind = if self.progress_heuristic {
            self.subqueue_inds()
                .into_iter()
                .min_by_key(|ind| self.subqueues[*ind].head)
        } else {
            self.subqueue_inds()
                .into_iter()
                .max_by_key(|ind| self.subqueues[*ind].tail - self.subqueues[*ind].head)
        }
        .expect("Should always be able to find an index if d>0");

        match self.subqueues[subqueue_ind].dequeue() {
            None if self.empty_lin => {
                let mut ind = subqueue_ind;
                for _ in 0..self.subqueues.len() - 1 {
                    ind = (ind + 1) % self.subqueues.len();
                    if self.subqueues[ind].len() > 0 {
                        return self.subqueues[ind].dequeue();
                    }
                }
                None
            }
            otherwise => otherwise,
        }
    }

    /// As dequeue, but also returns the number of successfull dequeues on the chosen sub-queue
    pub fn dequeue_with_info(&mut self) -> (Option<T>, usize) {
        // Find sub-queue ind, depending on heuristic used (this is not super optimized)
        let subqueue_ind = if self.progress_heuristic {
            self.subqueue_inds()
                .into_iter()
                .min_by_key(|ind| self.subqueues[*ind].head)
        } else {
            self.subqueue_inds()
                .into_iter()
                .max_by_key(|ind| self.subqueues[*ind].tail - self.subqueues[*ind].head)
        }
        .expect("Should always be able to find an index if d>0");

        match self.subqueues[subqueue_ind].dequeue() {
            None if self.empty_lin => {
                let mut ind = subqueue_ind;
                for _ in 0..self.subqueues.len() - 1 {
                    ind = (ind + 1) % self.subqueues.len();
                    if self.subqueues[ind].len() > 0 {
                        return (self.subqueues[ind].dequeue(), self.subqueues[ind].head);
                    }
                }
                (None, self.subqueues[subqueue_ind].head)
            }
            otherwise => (otherwise, self.subqueues[subqueue_ind].head),
        }
    }

    /// Gets sub-queue inds, depending on allowing repeats of not
    fn subqueue_inds(&self) -> Vec<usize> {
        if self.uniques {
            (0..self.subqueues.len())
                .collect::<Vec<usize>>()
                .choose_multiple(&mut rand::thread_rng(), self.d)
                .cloned()
                .collect()
        } else {
            (0..self.d)
                .map(|_| rand::thread_rng().gen_range(0..self.subqueues.len()))
                .collect()
        }
    }

    pub fn print_skewness(&self) {
        let (mean_head, std_head) =
            std(&self.subqueues.iter().map(|p| p.head).collect::<Vec<usize>>());
        let (mean_tail, std_tail) =
            std(&self.subqueues.iter().map(|p| p.tail).collect::<Vec<usize>>());
        println!("Mean head: {mean_head}, std: {std_head}");
        println!("Mean tail: {mean_tail}, std: {std_tail}");
        print!("Heads: ");
        for p in self.subqueues.iter() {
            print!(" {}", p.head);
        }
        println!("");
        print!("Tails: ");
        for p in self.subqueues.iter() {
            print!(" {}", p.tail);
        }
        println!("");
    }

    pub fn nbr_subqueues(&self) -> usize {
        self.subqueues.len()
    }

    /// Returns the number of enqueues done on each sub-queue
    pub fn subqueue_enqueue_counts(&self) -> Vec<usize> {
        self.subqueues.iter().map(|p| p.tail).collect()
    }

    /// Returns the number of dequeues done on each partial queue
    pub fn subqueue_dequeue_counts(&self) -> Vec<usize> {
        self.subqueues.iter().map(|p| p.head).collect()
    }
}

fn std(values: &Vec<usize>) -> (f32, f32) {
    let mean = values.iter().cloned().sum::<usize>() as f32 / values.len() as f32;
    let std = (values
        .iter()
        .map(|val| (*val as f32 - mean) * (*val as f32 - mean))
        .sum::<f32>()
        / values.len() as f32)
        .sqrt();

    (mean, std)
}

struct SubQueue<T: PartialEq + Eq> {
    head: usize,
    tail: usize,
    fifo: VecDeque<T>,
}

impl<T: PartialEq + Eq> SubQueue<T> {
    fn new() -> Self {
        Self {
            head: 0,
            tail: 0,
            fifo: VecDeque::new(),
        }
    }

    fn enqueue(&mut self, item: T) {
        self.tail += 1;
        self.fifo.push_back(item)
    }

    fn dequeue(&mut self) -> Option<T> {
        let ret = self.fifo.pop_front();
        if ret.is_some() {
            self.head += 1;
        }
        ret
    }

    fn len(&self) -> usize {
        self.fifo.len()
    }
}

impl<T: PartialEq + Eq> RelaxedFifo<T> for DChoiceQueue<T> {
    fn enqueue(&mut self, item: T) {
        self.enqueue(item)
    }

    fn dequeue(&mut self) -> Option<T> {
        self.dequeue()
    }
}
