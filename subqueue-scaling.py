import argparse
import subprocess
import json
import re
import matplotlib.pyplot as plt
import seaborn as sns
import pandas as pd
import numpy as np


def parse_arguments():
    parser = argparse.ArgumentParser(
        description="Run Rust tests and plot heatmap.")
    parser.add_argument('-s', '--subqueues', type=int, nargs="+",
                        help='List with number of sub-queues to use')
    parser.add_argument('-o', '--operations', type=int,
                        help='Number of operations to run')
    parser.add_argument('-i', '--prefill', type=int,
                        help='Number of items to insert before test start')
    parser.add_argument('-r', '--runs', type=int, default=1,
                        help='The number of runs to do for each data point [Possible: average, worst-one-percent] [Default = average]')
    parser.add_argument('--readout', type=str, default="average",
                        help='How to read out the error from each simulation [Default = average]')
    parser.add_argument('--save_path', type=str,
                        help='Saves the graph, to this path if supplied.')
    parser.add_argument('--old_json', type=str,
                        help='Path to old JSON file containing to plot')
    return parser.parse_args()


def run_rust_test(operations, subqueues, prefill, runs, readout):
    subqueues_str = ' '.join(map(str, subqueues))
    command = (
        f"cargo run -r -- subqueues-and-prefill -o {operations} -s {subqueues_str} -i {prefill} -r {runs}"
        f" --heuristic operation --readout {readout}"
    )

    result = subprocess.run(
        command, capture_output=True, text=True, shell=True)
    if result.returncode != 0:
        print("Error running the Rust command")
        print(result.stderr)
        exit()
    print(result.stdout, end="")

    return result.stdout


def extract_file_path(output):
    match = re.search(r'Writing output to: (\S+)', output)
    if match:
        return match.group(1)
    else:
        raise ValueError("Output file path not found in the command output.")


def read_and_parse_data(filepath):
    with open(filepath, 'r') as file:
        data = json.load(file)
        subqueue_errors = sorted(
            [(int(p_ops_str[1:].split(',')[0]), avg) for [p_ops_str, avg] in data], key=lambda pair: pair[0])
    return subqueue_errors


def plot_graph(data_points, save_path, error_column, title="Operation Heuristic Scalability"):
    # Create a DataFrame from the list of tuples
    df = pd.DataFrame(data_points, columns=['subqueues', error_column])

    # Apply the seaborn style
    sns.set_theme()

    fig, ax = plt.subplots()
    sns.lineplot(data=df, x='subqueues',
                 y=error_column, ax=ax, marker='o')

    # Setting the logarithmic scale for the x-axis with base 2
    ax.set_xscale("log", base=2)
    ax.get_xaxis().set_major_formatter(plt.FuncFormatter(
        lambda val, pos: f'${{2^{{{int(np.log2(val))}}}}}$'))
    ax.set_yscale("log", base=2)

    # Setting the title of the graph
    ax.set_title(title)

    plt.tight_layout()
    plt.show()

    if save_path:
        fig.savefig(save_path)


def main():
    args = parse_arguments()

    if args.old_json:
        # If JSON path is provided, use it directly
        file_path = args.old_json
    else:
        if not (args.operations and args.subqueues and args.prefill):
            print(
                "Please provide operations, subqueues, and prefill arguments or specify JSON file.")
            return

        # Run Rust command to generate new JSON files
        output = run_rust_test(
            args.operations, args.subqueues, args.prefill, args.runs, args.readout)
        file_path = extract_file_path(output)

    # Read data from the paths
    data_points = read_and_parse_data(file_path)

    # Parse and plot data
    error_column = {"average": "Average Rank Error",
                    "worst-one-percent": "Worst 1% point Rank Error"}[args.readout]
    plot_graph(data_points, args.save_path, error_column)


if __name__ == "__main__":
    main()
