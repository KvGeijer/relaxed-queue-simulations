use rand::Rng;

fn main() {
    // For the progress-based one, the average error seems to scale with the number of partials
    // But the length-based one also scales with prefill and nbr_operations
    let prefill = 0;
    let nbr_operations = 10000;

    let operations = (0..nbr_operations)
        .map(|_| rand::thread_rng().gen_bool(0.5))
        .collect();

    println!("Using prefill {prefill}, and {} operations", nbr_operations);

    for partials in [1, 8, 64] {
        for heuristic in [true, false] {
            let queue = relaxation_analysis::d_ra::DRa::new(partials, 2, true, heuristic, true);
            let avg_error = avg(relaxation_analysis::relaxation_analysis::analyze(
                queue,
                prefill,
                &operations,
            ));
            println!(
                "2-RA with {:>3} partials and {:>10}-based heuristic. Avg error: {avg_error}",
                partials,
                if heuristic { "progress" } else { "length" }
            );
        }
    }
}

fn avg(nbrs: Vec<usize>) -> f32 {
    let len = nbrs.len();
    nbrs.into_iter().sum::<usize>() as f32 / len as f32
}
