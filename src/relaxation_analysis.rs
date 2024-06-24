use crate::{analyze_extra, DRa, ErrorTag};

/// Analyze relaxation properties of a relaxed queue (passed empty)
///
/// Returns sorted discrite probability density functions (pdf). These each have pdf_samples samples.
/// Pdfs returned: (
///     - Rank errors,
///     - Difference of enqueue nbr and dequeue nbr (for non-empty returns only),
///     - Difference between the partial queue load and average load at dequeue,
///     - Difference between the partial queue load and average load at enqueue (sampled from returned items),
/// )
pub fn analyze_distributions(
    relaxed_queue: &mut DRa<usize>,
    prefill: usize,
    operations: &Vec<bool>,
) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {
    let error_tags = analyze_extra(relaxed_queue, prefill, operations);

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

    let mut partial_deq_diff: Vec<f32> = error_tags
        .iter()
        .map(|tag| {
            // TODO: Should it be deq_nbr - 1? Too tired when writing
            let mean = tag.deq_nbr() as f32 / relaxed_queue.nbr_partials() as f32;
            tag.partial_nbr() as f32 - 1.0 - mean
        })
        .collect();
    partial_deq_diff.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mut partial_enq_diff: Vec<f32> = error_tags
        .iter()
        .filter_map(|tag| match tag {
            ErrorTag::ItemDequeue {
                enq_nbr,
                partial_nbr,
                ..
            } => {
                let mean = *enq_nbr as f32 / relaxed_queue.nbr_partials() as f32;
                Some(*partial_nbr as f32 - 1.0 - mean)
            }
            ErrorTag::EmptyDequeue { .. } => None,
        })
        .collect();
    partial_enq_diff.sort_by(|a, b| a.partial_cmp(b).unwrap());

    (
        rank_errors.into_iter().map(|val| val as f32).collect(),
        enq_deq_diffs.into_iter().map(|val| val as f32).collect(),
        partial_deq_diff,
        partial_enq_diff,
    )
    // (
    //     into_pdf(rank_errors, pdf_samples),
    //     into_pdf(enq_deq_diffs, pdf_samples),
    //     into_pdf(partial_deq_diff, pdf_samples),
    //     into_pdf(partial_enq_diff, pdf_samples),
    // )
}

// /// Takes a sorted vector of values and samples it into a pdf (using mean)
// fn into_pdf<T: num_traits::AsPrimitive<f32> + Sum<T> + Clone>(
//     values: Vec<T>,
//     samples: usize,
// ) -> Vec<f32> {
//     let per_sample = values.len() as f32 / samples as f32;

//     (0..samples)
//         .map(|i| {
//             let start = (per_sample * i as f32).round() as usize;
//             let stop = (per_sample * (i + 1) as f32).round() as usize;

//             values[start..stop].iter().cloned().sum::<T>().as_()
//         })
//         .collect()
// }
