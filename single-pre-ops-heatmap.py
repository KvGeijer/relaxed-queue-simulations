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
        description="Run Rust tests and plot a single heatmap.")
    parser.add_argument('-p', '--partials', type=int, help='partial queues')
    parser.add_argument('-o', '--operations', type=int,
                        nargs='+', help='operation count list')
    parser.add_argument('-i', '--prefill', type=int,
                        nargs='+', help='prefill list')
    parser.add_argument('--earlier_json', type=str,
                        help='Path to earlier JSON file')
    parser.add_argument('-r', '--runs', type=int, default=1,
                        help='The number of runs to do for each data point [Default = 1]')
    parser.add_argument('-s', '--save_path', type=str,
                        help='Saves the heatmaps, to this path if supplied.')
    parser.add_argument('--heuristic', type=str, default="length",
                        help='heuristic (length/operation) [Default=length]')
    parser.add_argument('--title', type=str,
                        help='The tile of the heapmap, defaults to no title.')
    parser.add_argument('--color_bounds', type=float, nargs='*',
                        help='Puts upper and lower bounds on the heatmap color-bar.')
    parser.add_argument('--hide_colorbar', action='store_true',
                        help='Hides the color bar of the heatmap')
    return parser.parse_args()


def run_rust_test(operations, partials, prefill, runs, heuristic):
    operations_str = ' '.join(map(str, operations))
    prefill_str = ' '.join(map(str, prefill))
    command = f"cargo run -- ops-and-prefill -o {operations_str} -p {partials} -i {prefill_str} -r {runs} --heuristic {heuristic}"
    result = subprocess.run(
        command, capture_output=True, text=True, shell=True)
    if result.returncode != 0:
        print("Error running the Rust command")
        print(result.stderr)
        exit()
    print(result.stdout)
    return result.stdout


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


def parse_and_transform_data(data):
    x_vals = []
    y_vals = []
    z_vals = []
    for pre_ops, value in data:
        prefill, ops = eval(pre_ops)
        x_vals.append(prefill)
        y_vals.append(ops)
        z_vals.append(value)
    return np.array(x_vals), np.array(y_vals), np.array(z_vals)


def plot_heatmap(x, y, z, save_path, title, color_bounds, hide_colorbar):
    if hide_colorbar:
        fig, ax = plt.subplots(figsize=(2.81, 3.2))
    else:
        fig, ax = plt.subplots(figsize=(3.8, 3.2))

    data_pivot = np.zeros((len(set(y)), len(set(x))))
    x_unique = sorted(set(x))
    y_unique = sorted(set(y))
    x_idx = {v: i for i, v in enumerate(x_unique)}
    y_idx = {v: i for i, v in enumerate(y_unique)}

    if color_bounds is None:
        vmin, vmax = np.min(z), np.max(z)
        print(f"Using colorbar bounds {vmin} and {vmax}")
    else:
        vmin, vmax = color_bounds[0], color_bounds[1]

    for xi, yi, zi in zip(x, y, z):
        data_pivot[y_idx[yi]][x_idx[xi]] = zi

    # Plot heatmap
    sns.heatmap(data_pivot, annot=False, fmt=".2f", ax=ax,
                # Needed to not get ugly lines in pdf plot: https://stackoverflow.com/questions/27040557/remove-lines-separating-cells-in-seaborn-heatmap-when-saved-as-pdf
                rasterized=True,
                norm=LogNorm(vmin=vmin, vmax=vmax),
                vmin=vmin, vmax=vmax, cbar=not hide_colorbar,
                cbar_kws={'label': 'Average Rank Error'})

    # Can include every value if you want
    x_ticks = list(range(0, len(x_unique)))[::2]
    y_ticks = list(range(0, len(y_unique)))[::2]
    # x_tick_labels = y_unique
    # y_tick_labels = x_unique
    # Remove comments to get power of two formatting
    x_tick_labels = [r"$2^{{{}}}$".format(
        int(np.log2(value + 1))) for value in x_unique][::2]
    y_tick_labels = [r"$2^{{{}}}$".format(
        int(np.log2(value + 1))) for value in y_unique][::2]
    # x_tick_labels = [x_tick_labels[i] for i in x_ticks]
    # y_tick_labels = [y_tick_labels[i] for i in y_ticks]
    ax.set_xticks([x + 0.5 for x in x_ticks])
    ax.set_yticks([y + 0.5 for y in y_ticks])
    # Change to 0, 45 or 90 depending on what you need
    ax.set_xticklabels(x_tick_labels, rotation=0)
    # Change to 0, 45 or 90 depending on what you need

    if title is not None:
        ax.set_title(title)
    ax.set_xlabel('prefill')

    if not hide_colorbar:
        ax.set_ylabel('operations')
        ax.set_yticklabels(y_tick_labels, rotation=0)
    else:
        ax.set_yticklabels([])
        # ax.set_yticklabels(None, rotation=0)

    plt.tight_layout()
    plt.show()

    if save_path:
        fig.savefig(save_path)


def main():
    args = parse_arguments()

    if args.earlier_json:
        file_path = args.earlier_json
    else:
        # Run Rust command to generate new JSON files
        rust_output = run_rust_test(
            args.operations, args.partials, args.prefill, args.runs, args.heuristic)
        file_path = extract_file_path(rust_output)

    # Read data from the paths
    json_data = read_data(file_path)

    # Parse and plot data
    x, y, z = parse_and_transform_data(json_data)
    plot_heatmap(x, y, z, args.save_path, args.title,
                 args.color_bounds, args.hide_colorbar)


if __name__ == "__main__":
    main()
