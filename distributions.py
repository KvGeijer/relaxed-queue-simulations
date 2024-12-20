import argparse
import subprocess
import json
import seaborn as sns
import matplotlib.pyplot as plt


def main():
    # Setting up command line arguments
    parser = argparse.ArgumentParser(
        description="Run cargo command and plot distributions.")
    parser.add_argument('-s', '--subqueues',
                        help='Number of subqueues')
    parser.add_argument('-o', '--operations',
                        help='Number of operations to simulate')
    parser.add_argument('-i', '--prefill',
                        help='Size of prefill')
    parser.add_argument('-r', '--runs', help='How many runs to average over')
    parser.add_argument('--old_json', type=str,
                        help='Path to old JSON file containing to plot')

    args = parser.parse_args()

    if not args.old_json:
        # Constructing the command
        cmd = [
            "cargo", "run", "-r", "--", "distributions",
            "-s", args.subqueues,
            "-o", args.operations,
            "-i", args.prefill
        ]

        if args.runs:
            cmd.extend(["-r", args.runs])
        # if args.distribution_samples:
        #     cmd.extend(["-s", args.distribution_samples])

        # Running the subprocess command
        print("Running command:", ' '.join(cmd))
        result = subprocess.run(cmd, capture_output=True, text=True)
        print(result.stdout)

        # Extract the file path from the output
        output_line = [line for line in result.stdout.split(
            '\n') if "Writing output to:" in line]
        if not output_line:
            raise Exception(
                f"No output file found in the command response.\n{output_line}")

        file_path = output_line[0].split(": ")[1].strip()
    else:
        file_path = args.old_json

    # Load JSON data from the output file
    with open(file_path, 'r') as file:
        data = json.load(file)

    # # Plotting the distributions one by one
    # for distribution in data:
    #     name, values = distribution
    #     sns.kdeplot(values, fill=True)
    #     plt.title(f'PDF: {name}')
    #     plt.xlabel('Values')
    #     plt.ylabel('Density')
    #     plt.show()

    # Determine the layout of the subplots
    num_distributions = len(data)
    cols = 2
    rows = (num_distributions + cols - 1) // cols  # Calculate rows needed

    # Create a figure with subplots
    fig, axs = plt.subplots(rows, cols, figsize=(5 * cols, 5 * rows))
    axs = axs.flatten()  # Flatten the array to make indexing easier

    # Plotting the distributions
    for i, distribution in enumerate(data):
        name, values = distribution
        sns.kdeplot(values, fill=False, ax=axs[i])
        axs[i].set_title(f'PDF: {name}')
        axs[i].set_xlabel('Values')
        axs[i].set_ylabel('Density')

    # Adjust layout and show the plot
    plt.tight_layout()
    plt.show()


if __name__ == "__main__":
    main()
