use std::{
    collections::HashMap,
    fs::{create_dir_all, File},
    io::Write,
    path::PathBuf,
};

use chrono::Local;
use clap::{Args, Parser, Subcommand, ValueEnum};
use rand::Rng;
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

        /// The name of the output json file, ends up at "results/{name}-{datetime}.json"
        #[arg(long, default_value_t = format!("OpsAndPrefill"))]
        output_name: String,
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

        /// The name of the output json file, ends up at "results/{name}-{datetime}.json"
        #[arg(long, default_value_t = format!("PartialsAndPrefill"))]
        output_name: String,
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
            let queue = queue.init();
            let avg_error = avg(analyze(queue, prefill, &operations));
            println!("{avg_error}");
        }
        Test::SingleAlternating {
            queue,
            operations,
            prefill,
        } => {
            let operations = (0..2 * operations).map(|i| i % 2 == 0).collect();
            let queue = queue.init();
            let avg_error = avg(analyze(queue, prefill, &operations));
            println!("{avg_error}");
        }
        Test::OpsAndPrefill {
            queue,
            operations,
            prefill,
            operations_distribution,
            output_name,
        } => {
            let mut results: HashMap<(usize, usize), _> = HashMap::new();
            for ops in operations.iter() {
                let ops_vec = match operations_distribution {
                    OperationDistribution::RandomBalanced => (0..*ops)
                        .map(|_| rand::thread_rng().gen_bool(0.5))
                        .collect(),
                    OperationDistribution::Alternating => (0..*ops).map(|i| i % 2 == 0).collect(),
                };
                for pre in prefill.iter() {
                    let key = (*pre, *ops);
                    if results.contains_key(&key) {
                        eprintln!(
                            "WARNING: Duplicate ops {ops} or prefill {pre} in cli arguments!"
                        );
                    } else {
                        let queue = queue.init();
                        results.insert(key, avg(analyze(queue, *pre, &ops_vec)));
                    }
                }
            }

            // Inefficient way to get it to print nicely
            let string_keyed_results: HashMap<String, f32> = results
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
        } => {
            let mut results: HashMap<(usize, usize), _> = HashMap::new();
            let ops_vec = match operations_distribution {
                OperationDistribution::RandomBalanced => (0..operations)
                    .map(|_| rand::thread_rng().gen_bool(0.5))
                    .collect(),
                OperationDistribution::Alternating => (0..operations).map(|i| i % 2 == 0).collect(),
            };
            for p in partials.iter() {
                for pre in prefill.iter() {
                    let key = (*p, *pre);
                    if results.contains_key(&key) {
                        eprintln!(
                            "WARNING: Duplicate partials {p} or prefill {pre} in cli arguments!"
                        );
                    } else {
                        let queue = queue.init(*p);
                        results.insert(key, avg(analyze(queue, *pre, &ops_vec)));
                    }
                }
            }

            // Inefficient way to get it to print nicely
            let string_keyed_results: HashMap<String, f32> = results
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

fn avg(nbrs: Vec<usize>) -> f32 {
    let len = nbrs.len();
    nbrs.into_iter().sum::<usize>() as f32 / len as f32
}
