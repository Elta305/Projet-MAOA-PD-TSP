#!/usr/bin/env python3
import json
from pathlib import Path
import matplotlib.pyplot as plt
import csv

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / 'results' / 'n50_single'
OUTDIR = ROOT / 'report' / 'figs'
OUTDIR.mkdir(parents=True, exist_ok=True)

files = sorted(RESULTS.glob('n50mosA_*.json'))
data = []
for f in files:
    with open(f, 'r', encoding='utf-8') as fh:
        j = json.load(fh)
    alg = j.get('algorithm')
    cost = j.get('cost')
    obj = j.get('objective')
    time = j.get('computation_time')
    data.append({'alg': alg, 'cost': cost, 'objective': obj, 'time': time, 'file': f.name})

# Save CSV summary
csv_path = OUTDIR / 'n50mosA_summary.csv'
with open(csv_path, 'w', newline='', encoding='utf-8') as csvf:
    writer = csv.DictWriter(csvf, fieldnames=['alg','cost','objective','time','file'])
    writer.writeheader()
    for row in data:
        writer.writerow(row)

# Plot objectives (higher is better) and runtimes
algs = [d['alg'] for d in data]
objs = [d['objective'] for d in data]
times = [d['time'] for d in data]

fig, ax1 = plt.subplots(figsize=(8,4))
bar = ax1.bar(algs, objs, color='C0')
ax1.set_ylabel('Objective (profit - travel)', color='C0')
ax1.set_title('n50mosA: Objective and Computation Time')
ax1.tick_params(axis='y', labelcolor='C0')

ax2 = ax1.twinx()
ax2.plot(algs, times, color='C1', marker='o')
ax2.set_ylabel('Time (s)', color='C1')
ax2.tick_params(axis='y', labelcolor='C1')

plt.tight_layout()
out_png = OUTDIR / 'n50mosA_results.png'
plt.savefig(out_png, dpi=200)
print(f'Wrote {csv_path} and {out_png}')
