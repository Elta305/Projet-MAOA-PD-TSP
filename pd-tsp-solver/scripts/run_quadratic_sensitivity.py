#!/usr/bin/env python3
"""Run sensitivity analysis for quadratic cost on n100mosA for selected algorithms.

Usage: run from repository root:
    python scripts/run_quadratic_sensitivity.py

Produces outputs in `results/quadratic_sensitivity/alpha_<val>/`.
"""
import subprocess
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
EXE = ROOT / 'target' / 'release' / 'pd-tsp-solver.exe'
INSTANCE = Path('..') / 'Datasets' / 'TS2004t2' / 'n100mosA.tsp'
RESULTS = ROOT / 'results' / 'quadratic_sensitivity'

# representative algorithms to compare (exclude 'exact' — quadratic cost not supported by Gurobi exact solver)
ALGS = ['greedy', 'ils', 'aco', 'mmas', 'ga']

# alpha values to sweep (alpha == coefficient of W^2)
ALPHAS = [0.0, 0.01, 0.05, 0.1, 0.2, 0.5, 1.0]
TIME_LIMIT = 60

def run_cmd(cmd, log_path):
    print('RUN:', ' '.join(cmd))
    with open(log_path, 'wb') as f:
        try:
            subprocess.check_call(cmd, stdout=f, stderr=subprocess.STDOUT)
        except subprocess.CalledProcessError as e:
            print(f'Command failed: {e}; see {log_path}')

def main():
    if not EXE.exists():
        print('Executable not found; please run `cargo build --release` first')
        sys.exit(1)

    for alpha in ALPHAS:
        out_dir = RESULTS / f'alpha_{alpha}'
        out_dir.mkdir(parents=True, exist_ok=True)
        for alg in ALGS:
            tag = f'n100mosA_{alg}_alpha{alpha}'
            json_out = out_dir / f'{tag}.json'
            log_out = out_dir / f'{tag}.log'

            cmd = [str(EXE), 'solve', '-i', str(INSTANCE), '-a', alg, '-t', str(TIME_LIMIT), '--cost-function', 'quadratic', '--alpha', str(alpha), '--output', str(json_out)]
            # keep verbose off to reduce log size; the exe prints summary to stdout which we capture
            run_cmd(cmd, str(log_out))

    print('Sensitivity runs finished; results under', RESULTS)

if __name__ == '__main__':
    main()
