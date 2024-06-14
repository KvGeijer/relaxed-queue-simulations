import argparse
import subprocess
import json
import re
import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np

from matplotlib.colors import LogNorm


def parse_arguments():
    parser = argparse.ArgumentParser(
        description="Run Rust tests and plot heatmap.")
    parser.add_argument('-p', '--partials', type=int, help='partial queues')
    parser.add_argument('-o', '--operations', type=int,
                        nargs='+', help='operation count list')
    parser.add_argument('-i', '--prefill', type=int,
                        nargs='+', help='prefill list')
    parser.add_argument('--operation_json', type=str,
                        help='Path to operation JSON file')
    parser.add_argument('--length_json', type=str,
                        help='Path to length JSON file')
    parser.add_argument('-r', '--runs', type=int, default=1,
                        help='The number of runs to do for each data point [Default = 1]')
    parser.add_argument('-s', '--save_path', type=str,
                        help='Saves the heatmaps, to this path if supplied.')
    return parser.parse_args()


def run_rust_test(operations, partials, prefill, runs):
    operations_str = ' '.join(map(str, operations))
    prefill_str = ' '.join(map(str, prefill))
    base_command = f"cargo run -- ops-and-prefill -o {operations_str} -p {partials} -i {prefill_str} -r {runs}"

    length_result = subprocess.run(
        base_command + " --heuristic length", capture_output=True, text=True, shell=True)
    if length_result.returncode != 0:
        print("Error running the Rust command")
        print(length_result.stderr)
        exit()
    print(length_result.stdout, end="")

    operation_result = subprocess.run(
        base_command + " --heuristic operation", capture_output=True, text=True, shell=True)
    if operation_result.returncode != 0:
        print("Error running the Rust command")
        print(operation_result.stderr)
        exit()
    print(operation_result.stdout, end="")

    return (operation_result.stdout, length_result.stdout)


def extract_file_path(output):
    match = re.search(r'Writing output to: (\S+)', output)
    if match:
        return match.group(1)
    else:
        raise ValueError("Output file path not found in the command output.")


def read_data(filepath):
    with open(filepath, 'r') as file:
        data = json.load(file)
    return data


def parse_and_transform_data(operation_data, length_data):
    def transform(data):
        x_vals, y_vals, z_vals = [], [], []
        for key, value in data:
            p, it = eval(key)
            x_vals.append(p)
            y_vals.append(it)
            z_vals.append(value)
        return np.array(x_vals), np.array(y_vals), np.array(z_vals)

    return transform(operation_data), transform(length_data)


def plot_heatmap(operation_data, length_data, save_path, titles=['Length Heuristic', 'Operation Heuristic']):
    fig, axes = plt.subplots(ncols=2, width_ratios=[
                             0.45, 0.55], sharey=True, figsize=(6, 3))
    data_sets = [length_data, operation_data]

    # Determine the common z-limits for the color scale
    all_z = np.concatenate([data[2] for data in data_sets])
    vmin, vmax = np.min(all_z), np.max(all_z)

    for ax_idx, (ax, data, title) in enumerate(zip(axes, data_sets, titles)):
        x, y, z = data
        data_pivot = np.zeros((len(set(x)), len(set(y))))
        x_unique = sorted(set(x))
        y_unique = sorted(set(y))
        x_idx = {v: i for i, v in enumerate(x_unique)}
        y_idx = {v: i for i, v in enumerate(y_unique)}

        for xi, yi, zi in zip(x, y, z):
            data_pivot[x_idx[xi]][y_idx[yi]] = zi

        # Plot heatmap, show color bar only on the last subplot
        sns.heatmap(data_pivot, annot=False, fmt=".2f", ax=ax,
                    # Needed to not get ugly lines in pdf plot: https://stackoverflow.com/questions/27040557/remove-lines-separating-cells-in-seaborn-heatmap-when-saved-as-pdf
                    rasterized=True,
                    norm=LogNorm(vmin=vmin, vmax=vmax),
                    vmin=vmin, vmax=vmax, cbar=ax_idx == len(axes)-1,
                    cbar_kws={'label': 'Average Rank Error'})

        # Can include every value if you want
        x_ticks = list(range(0, len(x_unique)))[::2]
        y_ticks = list(range(0, len(y_unique)))[::2]
        x_tick_labels = y_unique
        y_tick_labels = x_unique
        # Remove comments to get power of two formatting
        # x_tick_labels = [r"$2^{{{}}}$".format(
        #     int(np.log2(value + 1))) for value in x_unique]
        # y_tick_labels = [r"$2^{{{}}}$".format(
        #     int(np.log2(value + 1))) for value in y_unique]
        x_tick_labels = [x_tick_labels[i] for i in x_ticks]
        y_tick_labels = [y_tick_labels[i] for i in y_ticks]
        ax.set_xticks([x + 0.5 for x in x_ticks])
        ax.set_yticks([y + 0.5 for y in y_ticks])
        # Change to 0, 45 or 90 depending on what you need
        ax.set_xticklabels(x_tick_labels, rotation=90)
        # Change to 0, 45 or 90 depending on what you need
        ax.set_yticklabels(y_tick_labels, rotation=0)

        ax.set_title(title)
        ax.set_xlabel('prefill')
        if ax_idx == 0:
            ax.set_ylabel('operations')

    plt.tight_layout()
    plt.show()

    if save_path:
        fig.savefig(save_path)


def main():
    args = parse_arguments()

    if args.operation_json and args.length_json:
        # If JSON paths are provided, use them directly
        operation_file_path = args.operation_json
        length_file_path = args.length_json
    else:
        if not (args.operations and args.partials and args.prefill):
            print(
                "Please provide operations, partials, and prefill arguments or specify JSON files.")
            return

        # Run Rust command to generate new JSON files
        operation_output, length_output = run_rust_test(
            args.operations, args.partials, args.prefill, args.runs)
        operation_file_path = extract_file_path(operation_output)
        length_file_path = extract_file_path(length_output)

    # Read data from the paths
    operation_data = read_data(operation_file_path)
    length_data = read_data(length_file_path)

    # Parse and plot data
    operation_data_parsed, length_data_parsed = parse_and_transform_data(
        operation_data, length_data)
    plot_heatmap(operation_data_parsed, length_data_parsed, args.save_path)


if __name__ == "__main__":
    main()
