use rand::Rng;

use crate::{analyze_extra, DChoiceQueue, ErrorTag};

/// Analyze relaxation properties of a relaxed queue (passed empty)
///
/// Returns sorted discrete probability density functions (pdf). These each have pdf_samples samples.
/// Pdfs returned: (
///     - Rank errors,
///     - Difference of enqueue nbr and dequeue nbr (for non-empty returns only),
///     - Difference between the partial queue load and average load at dequeue,
///     - Difference between the partial queue load and average load at enqueue (sampled from returned items),
///     - The partial enqueue counts at the end. Subtracted by the mean load, and sorted in ascending order
///     - The partial dequeue counts at the end. Subtracted by the mean load, and sorted in ascending order
/// )
pub fn analyze_distributions(
    relaxed_queue: &mut DChoiceQueue<usize>,
    prefill: usize,
    operations: &Vec<bool>,
) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {
    let extra_ops = rand::thread_rng().gen_range(0..relaxed_queue.nbr_subqueues());
    // A bit of a hack, but add some extra enqueue and dequeues at the end to get random mean values of loads
    let extended_operations = operations
        .iter()
        .cloned()
        .chain(std::iter::repeat(true).take(extra_ops))
        .chain(std::iter::repeat(false).take(extra_ops))
        .collect();

    let error_tags = analyze_extra(relaxed_queue, prefill, &extended_operations);

    let mut rank_errors: Vec<usize> = error_tags.iter().map(|tag| tag.rank_error()).collect();
    rank_errors.sort();

    let mut enq_deq_diffs: Vec<i64> = error_tags
        .iter()
        .filter_map(|tag| match tag {
            ErrorTag::ItemDequeue {
                enq_nbr, deq_nbr, ..
            } => Some(*enq_nbr as i64 - *deq_nbr as i64),
            ErrorTag::EmptyDequeue { .. } => None,
        })
        .collect();
    enq_deq_diffs.sort();

    let mut subqueue_deq_diff: Vec<f32> = error_tags
        .iter()
        .map(|tag| {
            // TODO: Should it be deq_nbr - 1? Too tired when writing
            let mean = tag.deq_nbr() as f32 / relaxed_queue.nbr_subqueues() as f32;
            tag.sub_nbr() as f32 - 1.0 - mean
        })
        .collect();
    subqueue_deq_diff.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mut subqueue_enq_diff: Vec<f32> = error_tags
        .iter()
        .filter_map(|tag| match tag {
            ErrorTag::ItemDequeue {
                enq_nbr,
                sub_nbr,
                ..
            } => {
                let mean = *enq_nbr as f32 / relaxed_queue.nbr_subqueues() as f32;
                Some(*sub_nbr as f32 - 1.0 - mean)
            }
            ErrorTag::EmptyDequeue { .. } => None,
        })
        .collect();
    subqueue_enq_diff.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mut enqueue_counts = relaxed_queue.subqueue_enqueue_counts();
    enqueue_counts.sort();
    let enqueue_avg =
        enqueue_counts.iter().cloned().sum::<usize>() as f32 / enqueue_counts.len() as f32;
    let enqueue_normlized_counts = enqueue_counts
        .into_iter()
        .map(|val| val as f32 - enqueue_avg)
        .collect();

    let mut dequeue_counts = relaxed_queue.subqueue_dequeue_counts();
    dequeue_counts.sort();
    let dequeue_avg =
        dequeue_counts.iter().cloned().sum::<usize>() as f32 / dequeue_counts.len() as f32;
    let dequeue_normlized_counts = dequeue_counts
        .into_iter()
        .map(|val| val as f32 - dequeue_avg)
        .collect();

    (
        rank_errors.into_iter().map(|val| val as f32).collect(),
        enq_deq_diffs.into_iter().map(|val| val as f32).collect(),
        subqueue_deq_diff,
        subqueue_enq_diff,
        enqueue_normlized_counts,
        dequeue_normlized_counts,
    )
}

