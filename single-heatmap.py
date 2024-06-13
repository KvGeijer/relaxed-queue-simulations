import argparse
import subprocess
import json
import re
import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np

def parse_arguments():
    parser = argparse.ArgumentParser(description="Run Rust tests and plot heatmap.")
    parser.add_argument('-o', '--operations', type=int, required=True, help='Number of operations')
    parser.add_argument('-p', '--partials', type=int, nargs='+', required=True, help='partials list')
    parser.add_argument('-i', '--prefill', type=int, nargs='+', required=True, help='prefill list')
    parser.add_argument('--heuristic', type=str, default="length", help='heuristic (length/operation) [Default=length]')
    parser.add_argument('-r', '--runs', type=int, default=1, help='The number of runs to do for each data point [Default = 1]')
    return parser.parse_args()

def run_rust_test(operations, partials, prefill, runs, heuristic):
    partials_str = ' '.join(map(str, partials))
    prefill_str = ' '.join(map(str, prefill))
    command = f"cargo run -- partials-and-prefill -o {operations} -p {partials_str} -i {prefill_str} -r {runs} --heuristic {heuristic}"
    result = subprocess.run(command, capture_output=True, text=True, shell=True)
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
    for key, value in data:
        p, it = eval(key)
        x_vals.append(p)
        y_vals.append(it)
        z_vals.append(value)
    return np.array(x_vals), np.array(y_vals), np.array(z_vals)

def plot_heatmap(x, y, z):
    data_pivot = np.zeros((len(set(x)), len(set(y))))
    x_unique = sorted(set(x))
    y_unique = sorted(set(y))
    x_idx = {v: i for i, v in enumerate(x_unique)}
    y_idx = {v: i for i, v in enumerate(y_unique)}
    
    for xi, yi, zi in zip(x, y, z):
        data_pivot[x_idx[xi]][y_idx[yi]] = zi
    
    sns.heatmap(data_pivot, annot=True, fmt=".2f", xticklabels=y_unique, yticklabels=x_unique)
    plt.title('D-Ra Relaxation')
    plt.xlabel('prefill')
    plt.ylabel('partials')
    plt.show()

def main():
    args = parse_arguments()
    output = run_rust_test(args.operations, args.partials, args.prefill, args.runs, args.heuristic)
    file_path = extract_file_path(output)
    data = read_data(file_path)
    x, y, z = parse_and_transform_data(data)
    plot_heatmap(x, y, z)

if __name__ == "__main__":
    main()

