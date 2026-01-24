#!/usr/bin/env python3
"""
Run PD-TSP benchmarks without PowerShell scripts.

Usage (from project root):
    python scripts/run_benchmarks.py --groups n100q1000 n100q45 --cost-modes distance quadratic linear-load \
        --runs 3 --time-limit 60 --build-gurobi

The script will:
- Build `cargo build --release --features gurobi` if `--build-gurobi` is set.
- For each group and cost-mode, iterate instances in `./benchmark_work/<group>` (or dataset dir if not prepared).
- For each instance and algorithm, call the `pd-tsp-solver` executable `solve` subcommand and save outputs/logs to `results/<group>_<mode>/`.

Note: run this from the repository root (pd-tsp-solver folder).
"""
import argparse
import subprocess
import sys
from pathlib import Path
import shutil

ROOT = Path(__file__).resolve().parents[1]
EXE = ROOT / 'target' / 'release' / 'pd-tsp-solver.exe'
DATASETS = Path(__file__).resolve().parents[2] / 'Datasets' / 'TS2004t2'
BENCHMARK_WORK = ROOT / 'benchmark_work'
RESULTS = ROOT / 'results'

# Algorithms must match the CLI ValueEnum names
ALGORITHMS = [
    'nn','greedy','savings','sweep','regret','cluster-first','multi-start','profit-density',
    'two-opt','vnd','sa','tabu','ils','ga','memetic','aco','mmas','hybrid','exact'
]

COST_FLAG_MAP = {
    'distance': 'distance',
    'quadratic': 'quadratic',
    'linear-load': 'linear-load'
}


def run(cmd, stdout_path=None, stderr_path=None, env=None):
    stdout_f = open(stdout_path, 'wb') if stdout_path else subprocess.DEVNULL
    stderr_f = open(stderr_path, 'wb') if stderr_path else subprocess.DEVNULL
    try:
        subprocess.check_call(cmd, stdout=stdout_f, stderr=stderr_f, env=env)
    finally:
        if stdout_path:
            stdout_f.close()
        if stderr_path:
            stderr_f.close()


def find_instances(group):
    # Prefer prepared benchmark_work folder, fallback to Datasets
    work_dir = BENCHMARK_WORK / group
    if work_dir.exists():
        dirp = work_dir
    else:
        # try dataset folder directly
        dirp = DATASETS
    files = sorted([p for p in dirp.glob(f'{group}*.tsp')])
    return files


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--groups', nargs='+', required=True)
    parser.add_argument('--cost-modes', nargs='+', default=['distance'])
    parser.add_argument('--runs', type=int, default=3)
    parser.add_argument('--time-limit', type=int, default=60)
    parser.add_argument('--alpha', type=float, default=0.1)
    parser.add_argument('--beta', type=float, default=0.5)
    parser.add_argument('--build-gurobi', action='store_true', help='Build release with gurobi feature')
    parser.add_argument('--include-exact', action='store_true', help='Include exact solver in runs (builds with gurobi)')
    parser.add_argument('--max-size', type=int, default=50)
    args = parser.parse_args()

    # Determine algorithm list: skip 'exact' unless building with gurobi or explicitly requested
    algs = ALGORITHMS.copy()
    if not args.build_gurobi and not args.include_exact:
        algs = [a for a in algs if a != 'exact']

    # Build if requested
    if args.build_gurobi:
        print('Building release with gurobi feature...')
        subprocess.check_call(['cargo', 'build', '--release', '--features', 'gurobi'], cwd=ROOT)
    elif args.include_exact:
        # User requested exact; build the binary with gurobi feature
        print('Building release with gurobi feature because --include-exact was set...')
        subprocess.check_call(['cargo', 'build', '--release', '--features', 'gurobi'], cwd=ROOT)
    else:
        # Ensure binary exists
        if not EXE.exists():
            print('Executable not found. Running cargo build --release...')
            subprocess.check_call(['cargo', 'build', '--release'], cwd=ROOT)

    for group in args.groups:
        instances = find_instances(group)
        if not instances:
            print(f'No instances found for group {group}, skipping')
            continue
        for mode in args.cost_modes:
            # accept alias 'linear' as 'linear-load'
            if mode == 'linear':
                mode = 'linear-load'
            print(f'=== Running group={group} mode={mode} ===')
            out_dir = RESULTS / f'{group}_{mode}'
            out_dir.mkdir(parents=True, exist_ok=True)
            for inst in instances:
                inst_name = inst.name
                print(f'-- Instance {inst_name}')
                for alg in algs:
                    for run_id in range(args.runs):
                        run_tag = f'{inst.stem}_{alg}_run{run_id}'
                        json_out = out_dir / f'{run_tag}.json'
                        log_out = out_dir / f'{run_tag}.log'

                        cmd = [str(EXE), 'solve', '-i', str(inst), '-a', alg, '-t', str(args.time_limit), '--max-profit', '100']
                        # cost function
                        if mode == 'quadratic':
                            cmd += ['--cost-function', 'quadratic', '--alpha', str(args.alpha)]
                        elif mode == 'linear-load':
                            cmd += ['--cost-function', 'linear-load', '--alpha', str(args.alpha)]
                        else:
                            cmd += ['--cost-function', 'distance']

                        # exact solver needs to have been built with gurobi feature; the binary is already built.
                        # add output json
                        cmd += ['--output', str(json_out)]

                        print(f'   Running {alg} (run {run_id})...')
                        try:
                            run(cmd, stdout_path=str(log_out), stderr_path=str(log_out))
                        except subprocess.CalledProcessError as e:
                            print(f'   ERROR running {alg} on {inst_name}: {e}. See {log_out} for details')

    print('All runs finished')


if __name__ == '__main__':
    main()
