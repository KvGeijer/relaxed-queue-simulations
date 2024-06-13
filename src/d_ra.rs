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

    /// If true, uses the new progress-based heuristic, otherwise the length-based
    progress_heuristic: bool,

    /// If true, uses round robin when finding an empty queue
    empty_lin: bool,
}

impl<T: PartialEq + Eq> DRa<T> {
    pub fn new(
        nbr_partials: usize,
        d: usize,
        uniques: bool,
        progress_heuristic: bool,
        empty_lin: bool,
    ) -> Self {
        Self {
            partials: (0..nbr_partials).map(|_| Partial::new()).collect(),
            d,
            uniques,
            progress_heuristic,
            empty_lin,
        }
    }

    /// Enqueues an item into the D-Ra
    pub fn enqueue(&mut self, item: T) {
        // Find partial ind, depending on heuristic used (this is not super optimized)
        let partial_ind = if self.progress_heuristic {
            self.partial_inds()
                .into_iter()
                .min_by_key(|ind| self.partials[*ind].tail)
        } else {
            self.partial_inds()
                .into_iter()
                .min_by_key(|ind| self.partials[*ind].tail - self.partials[*ind].head)
        }
        .expect("Should always be able to find an index if d>0");

        self.partials[partial_ind].enqueue(item);
    }

    pub fn dequeue(&mut self) -> Option<T> {
        // Find partial ind, depending on heuristic used (this is not super optimized)
        let partial_ind = if self.progress_heuristic {
            self.partial_inds()
                .into_iter()
                .min_by_key(|ind| self.partials[*ind].head)
        } else {
            self.partial_inds()
                .into_iter()
                .max_by_key(|ind| self.partials[*ind].tail - self.partials[*ind].head)
        }
        .expect("Should always be able to find an index if d>0");

        match self.partials[partial_ind].dequeue() {
            None if self.empty_lin => {
                let mut ind = partial_ind;
                for _ in 0..self.partials.len() - 1 {
                    ind = (ind + 1) % self.partials.len();
                    if self.partials[ind].len() > 0 {
                        return self.partials[ind].dequeue();
                    }
                }
                None
            }
            otherwise => otherwise,
        }
    }

    /// Gets partial inds, depending on allowing repeats of not
    fn partial_inds(&self) -> Vec<usize> {
        if self.uniques {
            (0..self.partials.len())
                .collect::<Vec<usize>>()
                .choose_multiple(&mut rand::thread_rng(), self.d)
                .cloned()
                .collect()
        } else {
            (0..self.d)
                .map(|_| rand::thread_rng().gen_range(0..self.partials.len()))
                .collect()
        }
    }

    pub fn print_skewness(&self) {
        let (mean_head, std_head) =
            std(&self.partials.iter().map(|p| p.head).collect::<Vec<usize>>());
        let (mean_tail, std_tail) =
            std(&self.partials.iter().map(|p| p.tail).collect::<Vec<usize>>());
        println!("Mean head: {mean_head}, std: {std_head}");
        println!("Mean tail: {mean_tail}, std: {std_tail}");
        print!("Heads: ");
        for p in self.partials.iter() {
            print!(" {}", p.head);
        }
        println!("");
        print!("Tails: ");
        for p in self.partials.iter() {
            print!(" {}", p.tail);
        }
        println!("");
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

struct Partial<T: PartialEq + Eq> {
    head: usize,
    tail: usize,
    fifo: VecDeque<T>,
}

impl<T: PartialEq + Eq> Partial<T> {
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

impl<T: PartialEq + Eq> RelaxedFifo<T> for DRa<T> {
    fn enqueue(&mut self, item: T) {
        self.enqueue(item)
    }

    fn dequeue(&mut self) -> Option<T> {
        self.dequeue()
    }
}
