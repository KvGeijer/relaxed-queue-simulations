#!/bin/sh

# The paper used runs=100, but we can see the pattern even with this smaller number. Increasing it might increase runtime (mitigated by parallelism)
runs=1

python3 pre-ops-heatmap.py -r $runs -s 64 -o 64 128 256 512 1024 2048 4096 8192 16384 32768 65536 131072 262144 524288 1048576 2097152 4194304 -i 64 128 256 512 1024 2048 4096 8192 16384 32768 65536 131072 262144 524288 1048576 --title "d-RA"  --heuristic length    --save_path d-RA
python3 pre-ops-heatmap.py -r $runs -s 64 -o 64 128 256 512 1024 2048 4096 8192 16384 32768 65536 131072 262144 524288 1048576 2097152 4194304 -i 64 128 256 512 1024 2048 4096 8192 16384 32768 65536 131072 262144 524288 1048576 --title "d-CBO" --heuristic operation --save_path d-CBO
echo "See output in 'd-RA.pdf' and 'd-CBO.pdf'"
