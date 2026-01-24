#!/usr/bin/env python3
import pandas as pd
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
bench_csv = ROOT / 'results' / 'n100_benchmark' / 'results.csv'
quad_csv = ROOT / 'results' / 'quadratic_sensitivity_summary.csv'
out_txt = ROOT / 'results' / 'linear_vs_quadratic_alpha0.1.txt'

if not bench_csv.exists():
    print('Benchmark CSV not found:', bench_csv)
    raise SystemExit(1)
if not quad_csv.exists():
    print('Quadratic summary not found:', quad_csv)
    raise SystemExit(1)

b = pd.read_csv(bench_csv)
q = pd.read_csv(quad_csv)

# map names: convert ILS-run0 -> ILS
b['alg_short'] = b['algorithm'].str.replace('-run\d+','',regex=True).str.replace('"','').str.strip()
# select algorithms of interest
algs = ['ILS','ACO','MMAS','GeneticAlgorithm','GreedyInsertion']
rows = []
for alg in algs:
    # for benchmark, pick best cost for this algorithm (min cost)
    sub = b[b['alg_short'].str.contains(alg, regex=False)]
    if sub.empty:
        bench_cost = None
    else:
        bench_cost = sub['cost'].min()
    # in quadratic csv, algorithm names differ: ACO, ILS, MMAS, GeneticAlgorithm, GreedyInsertion
    qsub = q[(q['algorithm']==alg) & (q['alpha']==0.1)]
    quad_cost = qsub['cost'].values[0] if not qsub.empty else None
    # compute objective using profit from quad (assume profit constant across runs)
    profit = qsub['profit'].values[0] if not qsub.empty else None
    bench_obj = profit - bench_cost if (profit is not None and bench_cost is not None) else None
    quad_obj = qsub['objective'].values[0] if not qsub.empty else None
    pct_change_cost = (quad_cost - bench_cost)/bench_cost*100 if (bench_cost and quad_cost) else None
    rows.append({'algorithm':alg,'bench_cost':bench_cost,'quad_cost':quad_cost,'profit':profit,'bench_objective':bench_obj,'quad_objective':quad_obj,'pct_cost_change':pct_change_cost})

with open(out_txt,'w',encoding='utf-8') as f:
    f.write('Comparison linear (benchmark) vs quadratic (alpha=0.1) — n100mosA\n')
    f.write('\n')
    for r in rows:
        f.write(f"Algorithm: {r['algorithm']}\n")
        f.write(f"  Linear cost (best): {r['bench_cost']}\n")
        f.write(f"  Quadratic cost (alpha=0.1): {r['quad_cost']}\n")
        f.write(f"  Profit used: {r['profit']}\n")
        f.write(f"  Linear objective: {r['bench_objective']}\n")
        f.write(f"  Quadratic objective: {r['quad_objective']}\n")
        pct = r['pct_cost_change']
        if pct is None:
            f.write("  Cost change: N/A\n\n")
        else:
            f.write(f"  Cost change: {pct:.2f}%\n\n")
print('Wrote', out_txt)
