use std::{
    collections::{BinaryHeap, HashSet},
    fs::{create_dir_all, File},
    io::Write,
    path::PathBuf,
    process,
    sync::Arc,
};

use chrono::Local;
use clap::{Args, Parser, Subcommand, ValueEnum};
use rand::{seq::SliceRandom, thread_rng};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use relaxation_analysis::{analyze_distributions, analyze_simple, DRa};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    test: Test,
}

#[derive(Subcommand, Debug)]
enum Test {
    /// Runs a single test
    Single {
        /// The queue configuration to use
        #[command(flatten)]
        queue: QueueArg,

        /// The number of operations to run
        #[arg(short, long = "ops")]
        operations: usize,

        /// The number of initial items in the queue before starting the experiment
        #[arg(short = 'i', long)]
        prefill: usize,

        /// How to generate the operations
        #[arg(value_enum, long = "ops-distr", default_value_t = OperationDistribution::RandomBalanced)]
        operations_distribution: OperationDistribution,

        /// How to readout the rank error from a single simulation
        #[arg(value_enum, long = "readout", default_value_t = ErrorReadout::Average)]
        error_readout: ErrorReadout,
    },

    /// Performsrmany tests for a queue, for combinations of operations and prefill
    OpsAndPrefill {
        /// The queue configuration to use
        #[command(flatten)]
        queue: QueueArg,

        /// The number of operations
        #[arg(short, long = "ops", value_delimiter = ' ', num_args = 1..)]
        operations: Vec<usize>,

        /// The number of initial items in the queue before starting the experiment
        #[arg(short = 'i', long, value_delimiter = ' ', num_args = 1..)]
        prefill: Vec<usize>,

        /// How to generate the operations
        #[arg(value_enum, long = "ops-distr", default_value_t = OperationDistribution::RandomBalanced)]
        operations_distribution: OperationDistribution,

        /// The name of the output json file, ends up at "results/{output_name}-{datetime}.json"
        #[arg(long, default_value_t = format!("OpsAndPrefill"))]
        output_name: String,

        /// The number of runs to average over for each data point
        #[arg(short, long, default_value_t = 1)]
        runs: usize,

        /// How to readout the rank error from a single simulation
        #[arg(value_enum, long = "readout", default_value_t = ErrorReadout::Average)]
        error_readout: ErrorReadout,
    },

    /// Tests all combinations of partial queues and prefill
    PartialsAndPrefill {
        /// The queue configuration to use
        #[command(flatten)]
        queue: QueueConfig,

        /// The number of operations to run
        #[arg(short, long = "ops")]
        operations: usize,

        /// All partial configurations to test
        #[arg(short, value_delimiter = ' ', num_args = 1..)]
        partials: Vec<usize>,

        /// The number of initial items in the queue before starting the experiment
        #[arg(short = 'i', long, value_delimiter = ' ', num_args = 1..)]
        prefill: Vec<usize>,

        /// How to generate the operations
        #[arg(value_enum, long = "ops-distr", default_value_t = OperationDistribution::RandomBalanced)]
        operations_distribution: OperationDistribution,

        /// The name of the output json file, ends up at "results/{output_name}-{datetime}.json"
        #[arg(long, default_value_t = format!("PartialsAndPrefill"))]
        output_name: String,

        /// The number of runs to average over for each data point
        #[arg(short, long, default_value_t = 1)]
        runs: usize,

        /// How to readout the rank error from a single simulation
        #[arg(value_enum, long = "readout", default_value_t = ErrorReadout::Average)]
        error_readout: ErrorReadout,
    },

    Distributions {
        /// The queue configuration to use
        #[command(flatten)]
        queue: QueueArg,

        /// The number of operations to run
        #[arg(short, long = "ops")]
        operations: usize,

        /// The number of initial items in the queue before starting the experiment
        #[arg(short = 'i', long)]
        prefill: usize,

        /// How to generate the operations
        #[arg(value_enum, long = "ops-distr", default_value_t = OperationDistribution::RandomBalanced)]
        operations_distribution: OperationDistribution,

        /// The name of the output json file, ends up at "results/{output_name}-{datetime}.json"
        #[arg(long, default_value_t = format!("Distributions"))]
        output_name: String,

        /// The number of runs to average over for each data point
        #[arg(short, long, default_value_t = 1)]
        runs: usize,
    },
}

#[derive(Args, Debug)]
struct QueueArg {
    /// The number of partial queues to use
    #[arg(short, long)]
    partials: usize,

    /// Further config about how the queues works
    #[command(flatten)]
    config: QueueConfig,
}

#[derive(Args, Debug)]
struct QueueConfig {
    /// The number of partials to sample for each operation (d).
    #[arg(short = 'd', long, default_value_t = 2)]
    sample_nbr: usize,

    /// What sampling heuristic to use
    #[arg(value_enum, long, default_value_t = Heuristic::Operation)]
    heuristic: Heuristic,

    /// What index sampling method to use
    #[arg(value_enum, long, default_value_t = Sampling::Naive)]
    sampling: Sampling,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, ValueEnum)]
enum Heuristic {
    /// Length-based heuristic, as in the original d-RA load balancer.
    Length,

    /// Operation-based heuristic, which is our new and improved heuristic.
    Operation,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, ValueEnum)]
enum Sampling {
    /// Just samples d values at random
    Naive,

    /// Does not sample the same index twice
    Uniques,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, ValueEnum)]
enum OperationDistribution {
    /// Randomly generate at equal probability
    RandomBalanced,

    /// Sequentially alternates enqueue and dequeues
    Alternating,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, ValueEnum)]
enum ErrorReadout {
    /// Reports the average rank error from each simulation
    Average,

    /// Reports the data point from each experiment that is at the 99-percentile quantile
    WorstOnePercent,
}

impl ErrorReadout {
    fn readout(&self, nbrs: Vec<usize>) -> f32 {
        let len = nbrs.len();
        match self {
            ErrorReadout::Average => nbrs.into_iter().sum::<usize>() as f32 / len as f32,
            ErrorReadout::WorstOnePercent => {
                let track_nbr = len / 100;
                let mut heap = BinaryHeap::with_capacity(track_nbr);
                for error in nbrs.into_iter() {
                    if heap.len() < track_nbr {
                        heap.push(-(error as i64));
                    } else if -heap.peek().unwrap() < error as i64 {
                        heap.pop();
                        heap.push(-(error as i64));
                    }
                }
                -heap.pop().unwrap() as f32
            }
        }
    }
}

impl QueueArg {
    fn init(&self) -> DRa<usize> {
        self.config.init(self.partials)
    }
}

impl QueueConfig {
    fn init(&self, partials: usize) -> DRa<usize> {
        DRa::new(
            partials,
            self.sample_nbr,
            self.sampling == Sampling::Uniques,
            self.heuristic == Heuristic::Operation,
            true,
        )
    }
}

fn main() {
    let cli = Cli::parse();

    // For the progress-based one, the average error seems to scale with the number of partials
    // But the length-based one also scales with prefill and nbr_operations

    match cli.test {
        Test::Single {
            queue,
            operations,
            prefill,
            operations_distribution,
            error_readout,
        } => {
            let operations = gen_ops(operations_distribution, operations);
            let mut queue = queue.init();
            let avg_error = error_readout.readout(analyze_simple(&mut queue, prefill, &operations));
            println!("{avg_error}");
        }
        Test::OpsAndPrefill {
            queue,
            operations,
            prefill,
            operations_distribution,
            output_name,
            runs,
            error_readout,
        } => {
            assert_uniques(&operations);
            assert_uniques(&prefill);
            let shared_queue = Arc::new(queue);

            let results: Vec<((usize, usize), f32)> = operations
                .par_iter()
                .flat_map(|ops| {
                    let ops_vec = Arc::new(gen_ops(operations_distribution, *ops));
                    let shared_queue = shared_queue.clone();
                    prefill.par_iter().map(move |pre| {
                        let shared_queue = shared_queue.clone();
                        let key = (*pre, *ops);
                        let mean: f32 = (0..runs)
                            .into_par_iter()
                            .map(|_| {
                                let mut queue = shared_queue.init();
                                let mean = error_readout
                                    .readout(analyze_simple(&mut queue, *pre, &ops_vec));
                                mean
                            })
                            .reduce(|| 0.0, |a, b| a + b)
                            / runs as f32;
                        (key, mean)
                    })
                })
                .collect();

            // Inefficient way to get it to print nicely
            let string_keyed_results: Vec<(String, f32)> = results
                .into_iter()
                .map(|((pre, ops), avg)| (format!("({pre}, {ops})"), avg))
                .collect();
            let serialized_output = serde_json::to_string_pretty(&string_keyed_results)
                .expect("Could not serialize the output.");

            let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
            // TODO: Don't always save it in results, in case we want to run from somewhere else
            let folder = "results";
            let path = PathBuf::from(folder).join(format!("{output_name}-{timestamp}.json"));

            // Create directory and file
            create_dir_all(folder).expect("Could not create the results dir");
            let mut file = File::create(&path).expect("Failed to create file");
            file.write_all(serialized_output.as_bytes())
                .expect("Failed to write output to file");

            println!("Writing output to: {}", path.to_string_lossy());
        }
        Test::PartialsAndPrefill {
            queue,
            operations,
            partials,
            prefill,
            operations_distribution,
            output_name,
            runs,
            error_readout,
        } => {
            assert_uniques(&prefill);
            assert_uniques(&partials);

            let ops_vec = gen_ops(operations_distribution, operations);

            let results: Vec<((usize, usize), f32)> = partials
                .par_iter()
                .flat_map(|p| {
                    prefill.par_iter().map(|pre| {
                        let key = (*p, *pre);
                        let mean = (0..runs)
                            .into_par_iter()
                            .map(|_| {
                                let mut queue = queue.init(*p);
                                error_readout.readout(analyze_simple(&mut queue, *pre, &ops_vec))
                            })
                            .reduce(|| 0.0, |a, b| a + b)
                            / runs as f32;
                        (key, mean)
                    })
                })
                .collect();

            // Inefficient way to get it to print nicely
            let string_keyed_results: Vec<(String, f32)> = results
                .into_iter()
                .map(|((pre, ops), avg)| (format!("({pre}, {ops})"), avg))
                .collect();
            let serialized_output = serde_json::to_string_pretty(&string_keyed_results)
                .expect("Could not serialize the output.");

            let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
            // TODO: Don't always save it in results, in case we want to run from somewhere else
            let folder = "results";
            let path = PathBuf::from(folder).join(format!("{output_name}-{timestamp}.json"));

            // Create directory and file
            create_dir_all(folder).expect("Could not create the results dir");
            let mut file = File::create(&path).expect("Failed to create file");
            file.write_all(serialized_output.as_bytes())
                .expect("Failed to write output to file");

            println!("Writing output to: {}", path.to_string_lossy());
        }
        Test::Distributions {
            queue,
            operations,
            prefill,
            output_name,
            runs,
            operations_distribution,
        } => {
            let ops_vec = gen_ops(operations_distribution, operations);

            // Average each data point in the distributions over all the runs
            let mut rank_errors = vec![0f32; operations / 2];
            let mut enq_deq_diffs = vec![0f32; operations / 2];
            let mut partial_deq_diffs = vec![0f32; operations / 2];
            let mut partial_enq_diffs = vec![0f32; operations / 2];

            let results: Vec<_> = (0..runs)
                .into_par_iter()
                .map(|_| {
                    let mut queue = queue.init();
                    analyze_distributions(&mut queue, prefill, &ops_vec)
                })
                .collect();

            results.into_iter().for_each(
                |(
                    new_rank_errors,
                    new_enq_deq_diffs,
                    new_partial_deq_diffs,
                    new_partial_enq_diffs,
                )| {
                    // Sum up all values in each x point
                    for i in 0..rank_errors.len() {
                        rank_errors[i] += new_rank_errors[i];
                        enq_deq_diffs[i] += new_enq_deq_diffs[i];
                        partial_deq_diffs[i] += new_partial_deq_diffs[i];
                        partial_enq_diffs[i] += new_partial_enq_diffs[i];
                    }
                },
            );

            // Average the values
            rank_errors
                .iter_mut()
                .for_each(|item| *item = *item / runs as f32);
            enq_deq_diffs
                .iter_mut()
                .for_each(|item| *item = *item / runs as f32);
            partial_deq_diffs
                .iter_mut()
                .for_each(|item| *item = *item / runs as f32);
            partial_enq_diffs
                .iter_mut()
                .for_each(|item| *item = *item / runs as f32);

            let string_keyed_results = [
                ("Rank Errors", rank_errors),
                ("Enq-Deq id difference", enq_deq_diffs),
                ("Deq load offset", partial_deq_diffs),
                ("Enq load offset", partial_enq_diffs),
            ];

            let serialized_output = serde_json::to_string_pretty(&string_keyed_results)
                .expect("Could not serialize the output.");

            let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
            // TODO: Don't always save it in results, in case we want to run from somewhere else
            let folder = "results";
            let path = PathBuf::from(folder).join(format!("{output_name}-{timestamp}.json"));

            // Create directory and file
            create_dir_all(folder).expect("Could not create the results dir");
            let mut file = File::create(&path).expect("Failed to create file");
            file.write_all(serialized_output.as_bytes())
                .expect("Failed to write output to file");

            println!("Writing output to: {}", path.to_string_lossy());
        }
    }
}

fn gen_ops(distr: OperationDistribution, operations: usize) -> Vec<bool> {
    match distr {
        OperationDistribution::RandomBalanced => {
            let mut ops_vec: Vec<bool> = std::iter::repeat(true)
                .take(operations / 2)
                .chain(std::iter::repeat(false).take(operations / 2))
                .collect();
            ops_vec.shuffle(&mut thread_rng());
            ops_vec
        }
        OperationDistribution::Alternating => (0..operations).map(|i| i % 2 == 0).collect(),
    }
}

/// Exits the program if the sent in vector has any duplicates.
fn assert_uniques<I, T>(iter: I)
where
    I: IntoIterator<Item = T>,
    T: PartialEq + Eq + std::hash::Hash,
{
    let mut seen = HashSet::new();
    for item in iter {
        if !seen.insert(item) {
            eprintln!("Duplicate found. Exiting program.");
            process::exit(1); // Exit with an error code.
        }
    }
}
