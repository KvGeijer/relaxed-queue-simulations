# Relaxation Simulations for Choice-of-Two Queues

This repository contains code to analyze the relaxation errors in relaxed FIFO queues built on the random choice of two. Especially, it analyzes relaxation errors in the sequential [d-RA queue](https://doi.org/10.1007/978-3-642-39958-9_18) and the d-CBO from the new paper _Balanced Allocations over Efficient Queues: A Fast Relaxed FIFO Queue_ to be published in PPoPP 2025.

These queues are relaxed FIFO queues, which mean that dequeue operations return _one of the oldest items_ instead of necessarily the _oldest item_. This allows the to achieve far better performance in a multi-threaded program. For such a relaxed dequeue of item _x_, we call the number of items currently in the queue, which were inserted before _x_, as the _rank error_. Relaxed queues want a good trade-off between good performance and low rank errors.

This repository contains a Rust project which can simulate rank errors of the sequential d-RA and d-CBO queues in a parallel manner. These will not necessarily have the same pattern as the errors in concurrent executions, but they have been show to follow similar patterns, and simulating sequential executions in parallel allows for far faster analysis of different configurations.

## Usage

To run, you need to set up a Rust environment with Cargo ([https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install)).

Then, you can run simulations with `cargo run` (use also the `-r` flag for compiler optimizations). For example, with `cargo run -- -h` you see the different simulations available to run, and you can do a simple simulation of a single setting as follows:
``` sh
cargo  run -r -- single --subqueues 16 --ops 1000000 --prefill 1000
```
or you can run many simulations for different combinations of operations and pre-fill as below
``` sh
cargo  run -r -- ops-and-prefill --subqueues 16 --ops 1000 2000 3000 4000 --prefill 100 250 400 --heuristic operation
```

There are also Python scripts that help you visualize the results. The most usable is likely [pre-ops-heatmap.py](./pre-ops-heatmap.py) which plots a heatmap of the results for varying number of operations and prefill used. See for example [recreate-ppopp.sh](./recreate-ppopp.sh) for how to use it.

## Related Publications
* Balanced Allocations over Efficient Queues: A Fast Relaxed FIFO Queue
  * Kåre von Geijer, Philippas Tsigas, Elias Johansson, Sebastian Hermansson.
  * To appear in proceedings of the 30th ACM SIGPLAN Annual Symposium on Principles and Practice of Parallel Programming, PPoPP 2025.
  * Re-create Figure 1 with ``bash recreate-ppopp.sh``.
