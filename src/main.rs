use std::{
    collections::HashSet,
    fs::{create_dir_all, File},
    io::Write,
    path::PathBuf,
    process,
    sync::Arc,
};

use chrono::Local;
use clap::{Args, Parser, Subcommand, ValueEnum};
use rand::Rng;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use relaxation_analysis::{analyze, DRa};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    test: Test,
}

#[derive(Subcommand, Debug)]
enum Test {
    /// Randomly performs enqueue or dequeue operations, testing a single queue
    SingleRandom {
        /// The queue configuration to use
        #[command(flatten)]
        queue: QueueArg,

        /// The number of operations to run
        #[arg(short, long = "ops")]
        operations: usize,

        /// The number of initial items in the queue before starting the experiment
        #[arg(short = 'i', long)]
        prefill: usize,
    },

    /// Performs pairs of enqueue/dequeue operations, testing a single queue
    SingleAlternating {
        /// The queue configuration to use
        #[command(flatten)]
        queue: QueueArg,

        /// The number of operation pairs to run
        #[arg(short, long = "ops")]
        operations: usize,

        /// The number of initial items in the queue before starting the experiment
        #[arg(short = 'i', long)]
        prefill: usize,
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
        Test::SingleRandom {
            queue,
            operations,
            prefill,
        } => {
            let operations = (0..operations)
                .map(|_| rand::thread_rng().gen_bool(0.5))
                .collect();
            let mut queue = queue.init();
            let avg_error = avg(analyze(&mut queue, prefill, &operations));
            println!("{avg_error}");
        }
        Test::SingleAlternating {
            queue,
            operations,
            prefill,
        } => {
            let operations = (0..2 * operations).map(|i| i % 2 == 0).collect();
            let mut queue = queue.init();
            let errors = analyze(&mut queue, prefill, &operations);
            let avg_error = avg(errors);
            println!("{avg_error}");
            // queue.print_skewness();
        }
        Test::OpsAndPrefill {
            queue,
            operations,
            prefill,
            operations_distribution,
            output_name,
            runs,
        } => {
            assert_uniques(&operations);
            assert_uniques(&prefill);
            let shared_queue = Arc::new(queue);

            let results: Vec<((usize, usize), f32)> = operations
                .par_iter()
                .flat_map(|ops| {
                    let ops_vec = Arc::new(match operations_distribution {
                        OperationDistribution::RandomBalanced => (0..*ops)
                            .map(|_| rand::thread_rng().gen_bool(0.5))
                            .collect(),
                        OperationDistribution::Alternating => {
                            (0..*ops).map(|i| i % 2 == 0).collect()
                        }
                    });
                    let shared_queue = shared_queue.clone();
                    prefill.par_iter().map(move |pre| {
                        let shared_queue = shared_queue.clone();
                        let key = (*pre, *ops);
                        let mean: f32 = (0..runs)
                            .into_par_iter()
                            .map(|_| {
                                let mut queue = shared_queue.init();
                                let mean = avg(analyze(&mut queue, *pre, &ops_vec));
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
        } => {
            assert_uniques(&prefill);
            assert_uniques(&partials);

            let ops_vec = match operations_distribution {
                OperationDistribution::RandomBalanced => (0..operations)
                    .map(|_| rand::thread_rng().gen_bool(0.5))
                    .collect(),
                OperationDistribution::Alternating => (0..operations).map(|i| i % 2 == 0).collect(),
            };

            let results: Vec<((usize, usize), f32)> = partials
                .par_iter()
                .flat_map(|p| {
                    prefill.par_iter().map(|pre| {
                        let key = (*p, *pre);
                        let mean = (0..runs)
                            .into_par_iter()
                            .map(|_| {
                                let mut queue = queue.init(*p);
                                avg(analyze(&mut queue, *pre, &ops_vec))
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

fn avg(nbrs: Vec<usize>) -> f32 {
    let len = nbrs.len();
    nbrs.into_iter().sum::<usize>() as f32 / len as f32
}
