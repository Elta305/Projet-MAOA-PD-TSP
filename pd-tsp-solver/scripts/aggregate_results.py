#!/usr/bin/env python3
import json
from pathlib import Path
import csv
import pandas as pd

ROOT = Path(__file__).resolve().parents[1]
RESULTS_ROOT = ROOT / 'results'
OUT_DIR = RESULTS_ROOT / 'aggregated'
OUT_DIR.mkdir(parents=True, exist_ok=True)

rows = []
for jf in sorted(RESULTS_ROOT.rglob('*.json')):
    try:
        j = json.load(open(jf, 'r', encoding='utf-8'))
    except Exception:
        continue
    alg = j.get('algorithm') or j.get('Algorithm') or jf.stem.split('_')[1] if '_' in jf.stem else jf.stem
    cost = j.get('cost') or j.get('travel_cost') or j.get('Cost')
    time = j.get('computation_time') or j.get('time') or j.get('computationTime')
    feasible = 1 if j.get('feasible') else 0
    iterations = j.get('iterations') if j.get('iterations') is not None else ''
    # cost function: prefer explicit field, else infer from parent folder name or filename
    cf = j.get('cost_function') or j.get('costFunction')
    if not cf:
        parts = jf.parts
        # look for folder like results/<group>_<mode>
        for p in parts[::-1]:
            if '_' in p and p.startswith('n'):
                cf = p.split('_')[-1]
                break
    if not cf:
        if jf.stem.endswith('_quadratic'):
            cf = 'quadratic'
        elif jf.stem.endswith('_linear-load') or jf.stem.endswith('_linear_load'):
            cf = 'linear-load'
        else:
            cf = 'distance'

    instance = jf.stem.split('_')[0]
    rows.append({'instance': instance, 'algorithm': alg, 'cost': cost, 'time': time, 'feasible': feasible, 'iterations': iterations, 'cost_function': cf})

results_csv = OUT_DIR / 'results.csv'
with open(results_csv, 'w', newline='', encoding='utf-8') as csvf:
    writer = csv.DictWriter(csvf, fieldnames=['instance','algorithm','cost','time','feasible','iterations','cost_function'])
    writer.writeheader()
    for r in rows:
        writer.writerow(r)

df = pd.DataFrame(rows)
if not df.empty:
    stats = df.groupby('algorithm').agg(avg_cost=('cost','mean'), avg_time=('time','mean'), feasible=('feasible','sum'), total=('instance','count')).reset_index()
    stats_csv = OUT_DIR / 'statistics.csv'
    stats.to_csv(stats_csv, index=False)
    print('Wrote aggregated results to', OUT_DIR)
else:
    print('No results found under', RESULTS_ROOT)
