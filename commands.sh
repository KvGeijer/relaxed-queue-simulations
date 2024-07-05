#!/bin/sh

# Compares the two heuristics over differing operation and prefill counts. Tinker with script for nice axes
python3 dual-pre-ops-heatmap.py -p 16 -i 64 128 256 512 1024 2048 4096 8192 16384 32768 65536 131072 262144 524288 1048576 -o 64 128 256 512 1024 2048 4096 8192 16384 32768 65536 131072 262144 524288 1048576 -r 200

# See how the heuristic scales with p
python partial-scaling.py -p 2 4 8 16 32 64 128 256 512 1024 2048 4096 8192 -o 10000 -i 100000 -r 10 --readout average
python partial-scaling.py -p 2 4 8 16 32 64 128 256 512 1024 2048 4096 8192 -o 10000 -i 100000 -r 10 --readout worst-one-percent

# Look at different distributions of properties for dequeues
python distributions.py -p 1000 -o 100000 -i 10000 -r 5000
