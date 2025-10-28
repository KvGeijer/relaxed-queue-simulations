use rand::Rng;
use std::cmp::max;
use std::collections::BTreeSet;

/// Simulates the simple d-choice, analyzing the gap between the min and max bucket
///
/// Uses buckets = n bins, and operations = m balls which are allocated, with d for d-choice.
/// Returns:
///     - A vector of the minmax gap at each time index
///     - The maximum minmax gap at each time
///     - The mean minmax gap
pub fn analyze_minmax_gap(buckets: usize, operations: usize, d: usize) -> (Vec<usize>, usize, f32) {
    let mut bins = vec![0; buckets];
    // Fast way to look up min
    let mut min_tracker = MinTracker::new(&bins);

    // The largest and smallest buckets
    let mut min_load = 0;
    let mut max_load = 0;

    // For the output
    let mut minmax_gaps = Vec::<usize>::with_capacity(operations);

    for _t in 0..operations {
        let index = (0..d)
            .map(|_| rand::thread_rng().gen_range(0..buckets))
            .min_by_key(|i| bins[*i])
            .unwrap();

        let old_load = bins[index];
        bins[index] += 1;
        min_tracker.inc(index, old_load);

        if old_load == min_load {
            min_load = min_tracker.peek_min();
        }
        max_load = max(max_load, old_load + 1);

        let minmax_gap = max_load - min_load;
        minmax_gaps.push(minmax_gap);
    }

    let minmax_mean = minmax_gaps.iter().map(|v| *v as f32).sum::<f32>() / minmax_gaps.len() as f32;
    let minmax_max = minmax_gaps.iter().cloned().max().unwrap();
    (minmax_gaps, minmax_max, minmax_mean)
}

struct MinTracker {
    // Sorted set of (bin_load, bin_index)
    sorted: BTreeSet<(usize, usize)>,
}

impl MinTracker {
    pub fn new(initial: &Vec<usize>) -> Self {
        let mut sorted = BTreeSet::new();
        for (i, v) in initial.iter().enumerate() {
            sorted.insert((*v, i));
        }
        Self { sorted }
    }

    /// Current minimum value (and its index)
    pub fn peek_min(&self) -> usize {
        self.sorted.first().map(|(v, _i)| *v).unwrap()
    }

    /// Increment (v, i) to (v + 1, i)
    pub fn inc(&mut self, i: usize, v: usize) {
        self.sorted.remove(&(v, i));
        self.sorted.insert((v + 1, i));
    }
}
