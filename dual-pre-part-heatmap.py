import argparse
import subprocess
import json
import re
import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np


def parse_arguments():
    parser = argparse.ArgumentParser(
        description="Run Rust tests and plot heatmap.")
    parser.add_argument('-o', '--operations', type=int,
                        help='Number of operations')
    parser.add_argument('-p', '--partials', type=int,
                        nargs='+', help='partials list')
    parser.add_argument('-i', '--prefill', type=int,
                        nargs='+', help='prefill list')
    parser.add_argument('--operation_json', type=str,
                        help='Path to operation JSON file')
    parser.add_argument('--length_json', type=str,
                        help='Path to length JSON file')
    return parser.parse_args()


def run_rust_test(operations, partials, prefill):
    partials_str = ' '.join(map(str, partials))
    prefill_str = ' '.join(map(str, prefill))
    base_command = f"cargo run -- partials-and-prefill -o {operations} -p {partials_str} -i {prefill_str}"

    length_result = subprocess.run(
        base_command + " --heuristic length", capture_output=True, text=True, shell=True)
    if length_result.returncode != 0:
        print("Error running the Rust command")
        print(length_result.stderr)
        exit()

    operation_result = subprocess.run(
        base_command + " --heuristic operation", capture_output=True, text=True, shell=True)
    if operation_result.returncode != 0:
        print("Error running the Rust command")
        print(operation_result.stderr)
        exit()

    print(operation_result.stdout, end="")
    print(length_result.stdout, end="")
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
        for key, value in data.items():
            p, it = eval(key)
            x_vals.append(p)
            y_vals.append(it)
            z_vals.append(value)
        return np.array(x_vals), np.array(y_vals), np.array(z_vals)

    return transform(operation_data), transform(length_data)


def plot_heatmap(operation_data, length_data, titles=['Length Heuristic', 'Operation Heuristic']):
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
        sns_heatmap = sns.heatmap(data_pivot, annot=False, fmt=".2f", ax=ax,
                                  xticklabels=y_unique, yticklabels=(
                                      x_unique if ax_idx == 1 else False),
                                  vmin=vmin, vmax=vmax, cbar=ax_idx == len(axes)-1)
        ax.set_title(title)
        ax.set_xlabel('prefill')
        if ax_idx == 0:
            ax.set_ylabel('partials')

    plt.tight_layout()
    plt.show()


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
            args.operations, args.partials, args.prefill)
        operation_file_path = extract_file_path(operation_output)
        length_file_path = extract_file_path(length_output)

    # Read data from the paths
    operation_data = read_data(operation_file_path)
    length_data = read_data(length_file_path)

    # Parse and plot data
    operation_data_parsed, length_data_parsed = parse_and_transform_data(
        operation_data, length_data)
    plot_heatmap(operation_data_parsed, length_data_parsed)


if __name__ == "__main__":
    main()
