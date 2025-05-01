# drones2

A Rust project for solving a vehicle routing problem variant, implementing various metaheuristic optimization algorithms like Simulated Annealing (SA), Adaptive Large Neighborhood Search (ALNS), and a experimental Pooled search strategy.

### Prerequisites

This project is built with Rust. If you don't have Rust installed, the recommended way is to use `rustup`.

**Installing Rust with `rustup`:**

1. Open your terminal.
2. Visit the official Rust website ([https://www.rust-lang.org/](https://www.rust-lang.org/)) and follow the instructions under "Install Rust". Generally, you can run the following command:
   `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

3.  Follow the on-screen prompts. The default installation (`recommended`) is usually sufficient. This will install the latest stable Rust toolchain.
4.  To make the `cargo` command available, you might need to add Rust's bin directory to your PATH. The installer will usually suggest adding this line to your shell's profile file (like `~/.bashrc`, `~/.zshrc`, etc.):

    After adding the line, either restart your terminal or run the command manually.

### Cloning the Repository

Clone the project from its GitHub repository:

`git clone https://github.com/janinge/drones2.git`

`cd drones2`

### Building the Project

To build the project in release mode (optimized), navigate to the project root directory (`drones2/`) and run:

`cargo build --release`

The compiled binaries will be located in `./target/release/`.

## Usage

The project provides several binary executables (`alns`, `sa`, `pooled`) implementing different search algorithms.

### Binaries

*   `alns`: Runs the Adaptive Large Neighborhood Search algorithm (`src/bin/alns.rs`).
*   `sa`: Runs the Simulated Annealing algorithm (`src/bin/sa.rs`).
*   `pooled`: Runs a "pooled" search strategy (likely a variant of SA or ALNS with specific temperature control targeted to a time limit) (`src/bin/pooled.rs`).

### Command-Line Arguments

The binaries typically accept the following arguments (defined in `src/utils/io.rs`):

*   `-p`, `--prefix <PREFIX>`: Path to a directory containing problem files, or a base path for problem files (optional, use with `--file` or to enumerate a directory).
*   `-f`, `--file <FILE>...`: Path to one or more specific problem files (optional, takes precedence over `--prefix` if both are directories). Can be specified multiple times.
*   `-r`, `--runs <RUNS>`: Number of times to run the algorithm with the same parameters for each included instance (default: `1`).
*   `-l`, `--time-limit <TIME_LIMIT>`: Maximum running time in seconds for each run/instance (optional, if not provided, some binaries might fallback to iteration limits or a default time limit like 1800s as seen in `pooled.rs`).
*   `--t0 <T0>`: Initial temperature for annealing/temperature-based algorithms (optional, if not provided, it might be determined by a warm-up phase as in `pooled.rs`).
*   `--t-final <T_FINAL>`: Final temperature (default: `10.0`). Used in the temperature cooling schedule.

### Examples

Assuming your problem files are in a directory named `data/` at the project root:

`./target/release/pooled -p data/ -f Call_18_Vehicle_5.txt -f Call_35_Vehicle_7.txt --runs 1 --time-limit 120`

