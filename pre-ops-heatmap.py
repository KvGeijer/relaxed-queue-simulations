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
    parser.add_argument('-s', '--subqueues', type=int, help='The number of sub-queues to use.')
    parser.add_argument('-o', '--operations', type=int,
                        nargs='+', help='Operation count list')
    parser.add_argument('-i', '--prefill', type=int,
                        nargs='+', help='Prefill list')
    parser.add_argument('--earlier_json', type=str,
                        help='Path to earlier JSON file for just plotting')
    parser.add_argument('-r', '--runs', type=int, default=1,
                        help='The number of runs to do for each data point [Default = 1]')
    parser.add_argument('--save_path', type=str,
                        help='Saves the heatmaps to this path if supplied.')
    parser.add_argument('--heuristic', type=str, default="length",
                        help='heuristic (length/operation) [Default=length]')
    parser.add_argument('--title', type=str,
                        help='The tile of the heapmap, defaults to no title.')
    parser.add_argument('--color_bounds', type=float, nargs='*',
                        help='Puts upper and lower bounds on the heatmap color-bar.')
    parser.add_argument('--hide_colorbar', action='store_true',
                        help='Hides the color bar of the heatmap')
    parser.add_argument('--hide_ylabel', action='store_true',
                        help='Hides the ylabel and tick values of the heatmap')
    return parser.parse_args()


def run_rust_test(operations, subqueues, prefill, runs, heuristic):
    operations_str = ' '.join(map(str, operations))
    prefill_str = ' '.join(map(str, prefill))
    command = f"cargo run -r -- ops-and-prefill -o {operations_str} -s {subqueues} -i {prefill_str} -r {runs} --heuristic {heuristic}"
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


def plot_heatmap(x, y, z, save_path, title, color_bounds, hide_colorbar, hide_ylabel):
    if hide_colorbar and not hide_ylabel:
        fig, ax = plt.subplots(figsize=(3, 3.2))
    elif not hide_colorbar and hide_ylabel:
        fig, ax = plt.subplots(figsize=(3.3, 3.2))
    elif hide_colorbar and hide_ylabel:
        fig, ax = plt.subplots(figsize=(2.5, 3.2))
    elif not hide_colorbar and not hide_ylabel:
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
    hmap = sns.heatmap(data_pivot, annot=False, fmt=".2f", ax=ax,
                       # Needed to not get ugly lines in pdf plot: https://stackoverflow.com/questions/27040557/remove-lines-separating-cells-in-seaborn-heatmap-when-saved-as-pdf
                       rasterized=True,
                       norm=LogNorm(vmin=vmin, vmax=vmax),
                       vmin=vmin, vmax=vmax, cbar=not hide_colorbar)

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
        ax.set_title(title, fontsize=18)

    ax.tick_params(labelsize=11.5)

    ax.set_xlabel('prefill', fontsize=16)

    if not hide_ylabel:
        ax.set_ylabel('operations', fontsize=16)
        ax.set_yticklabels(y_tick_labels, rotation=0)
    else:
        ax.set_yticklabels([])

    if not hide_colorbar:
        hmap.collections[0].colorbar.ax.tick_params(labelsize=11.5)
        hmap.collections[0].colorbar.set_label(
            'Average Rank Error', fontsize=16)

    plt.tight_layout(pad=0)

    if save_path:
        fig.savefig(f'{save_path}.pdf', format='pdf')
    else:
        plt.show()



def main():
    args = parse_arguments()

    if args.earlier_json:
        file_path = args.earlier_json
    else:
        # Run Rust command to generate new JSON files
        rust_output = run_rust_test(
            args.operations, args.subqueues, args.prefill, args.runs, args.heuristic)
        file_path = extract_file_path(rust_output)

    # Read data from the paths
    json_data = read_data(file_path)

    # Parse and plot data
    x, y, z = parse_and_transform_data(json_data)
    plot_heatmap(x, y, z, args.save_path, args.title,
                 args.color_bounds, args.hide_colorbar, args.hide_ylabel)


if __name__ == "__main__":
    main()
