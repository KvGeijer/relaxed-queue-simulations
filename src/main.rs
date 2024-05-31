use clap::{Args, Parser, Subcommand, ValueEnum};
use rand::Rng;
use relaxation_analysis::{analyze, DRa};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Optional name of output file. Defaults to "results/{test}-{datetime}.json"
    #[arg(long)]
    output_file: Option<String>,

    #[command(subcommand)]
    test: Test,
}

#[derive(Subcommand)]
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
}

#[derive(Args)]
struct QueueArg {
    /// The number of partial queues to use
    #[arg(short, long)]
    partials: usize,

    /// The number of partials to sample for each operation (d)
    #[arg(short = 'd', long)]
    sample_nbr: usize,

    /// What sampling heuristic to use
    #[arg(value_enum, long, default_value_t = Heuristic::Operation)]
    heuristic: Heuristic,

    /// What index sampling method to use
    #[arg(value_enum, long, default_value_t = Sampling::Naive)]
    sampling: Sampling,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Heuristic {
    /// Length-based heuristic, as in the original d-RA load balancer.
    Length,

    /// Operation-based heuristic, which is our new and improved heuristic.
    Operation,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Sampling {
    /// Just samples d values at random
    Naive,

    /// Does not sample the same index twice
    Uniques,
}

impl QueueArg {
    fn init(&self) -> DRa<usize> {
        DRa::new(
            self.partials,
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
    }
}

fn avg(nbrs: Vec<usize>) -> f32 {
    let len = nbrs.len();
    nbrs.into_iter().sum::<usize>() as f32 / len as f32
}
