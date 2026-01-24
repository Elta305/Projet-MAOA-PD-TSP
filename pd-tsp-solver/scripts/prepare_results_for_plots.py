#!/usr/bin/env python3
import json
from pathlib import Path
import csv
import pandas as pd

ROOT = Path(__file__).resolve().parents[1]
RESULTS_DIR = ROOT / 'results' / 'n50_single'
OUT_DIR = ROOT / 'results_n50_single'
OUT_DIR.mkdir(parents=True, exist_ok=True)

rows = []
for f in sorted(RESULTS_DIR.glob('n50mosA_*.json')):
    with open(f, 'r', encoding='utf-8') as fh:
        j = json.load(fh)
    alg = j.get('algorithm')
    cost = j.get('cost')
    time = j.get('computation_time')
    feasible = 1 if j.get('feasible') else 0
    iterations = j.get('iterations') if j.get('iterations') is not None else ''
    instance = 'n50mosA'
    # capture cost function from filename convention (quadratic suffix) or j
    cf = 'distance'
    if f.name.endswith('_quadratic.json'):
        cf = 'quadratic'
    rows.append({'instance': instance, 'algorithm': alg, 'cost': cost, 'time': time, 'feasible': feasible, 'iterations': iterations, 'cost_function': cf})

# Write results.csv
results_csv = OUT_DIR / 'results.csv'
with open(results_csv, 'w', newline='', encoding='utf-8') as csvf:
    writer = csv.DictWriter(csvf, fieldnames=['instance','algorithm','cost','time','feasible','iterations','cost_function'])
    writer.writeheader()
    for r in rows:
        writer.writerow(r)

# Compute statistics per algorithm
df = pd.DataFrame(rows)
stats = df.groupby('algorithm').agg(avg_cost=('cost','mean'), avg_time=('time','mean'), feasible=('feasible','sum'), total=('instance','count')).reset_index()
stats_csv = OUT_DIR / 'statistics.csv'
stats.to_csv(stats_csv, index=False)

print(f'Wrote {results_csv} and {stats_csv}')

# Also create minimal results_complete CSVs for analyze_results.py
RC = ROOT / 'results_complete'
RC.mkdir(parents=True, exist_ok=True)

# Constructive: take algorithms considered 'constructive'
constructive_algs = ['NearestNeighbor','GreedyInsertion','MultiStart']
constructive = df[df['algorithm'].isin(constructive_algs)].copy()
if not constructive.empty:
    constructive['CostFunction'] = constructive['cost_function']
    constructive['Algorithm'] = constructive['algorithm']
    constructive['Instance'] = 'n50mosA'
    constructive['AvgCost'] = constructive['cost']
    constructive['AvgTime'] = constructive['time']
    constructive['MinCost'] = constructive['cost']
    constructive[['Algorithm','Instance','CostFunction','AvgCost','AvgTime','MinCost']].to_csv(RC / 'constructive_all_costs.csv', index=False)

# Metaheuristic: ACO, GA
meta = df[df['algorithm'].isin(['ACO','GeneticAlgorithm'])].copy()
if not meta.empty:
    meta['CostFunction'] = meta['cost_function']
    meta['Algorithm'] = meta['algorithm']
    meta['Instance'] = 'n50mosA'
    meta['AvgCost'] = meta['cost']
    meta['AvgTime'] = meta['time']
    meta['MinCost'] = meta['cost']
    meta[['Algorithm','Instance','CostFunction','AvgCost','AvgTime','MinCost']].to_csv(RC / 'metaheuristics_all_costs.csv', index=False)

print('Prepared results for generate_plots.py and analyze_results.py under:', OUT_DIR, RC)
