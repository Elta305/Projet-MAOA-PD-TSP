#!/usr/bin/env python3
"""Aggregate JSON results from results/quadratic_sensitivity into CSV and generate plots.
"""
import json
from pathlib import Path
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns

ROOT = Path(__file__).resolve().parents[1]
RES_DIR = ROOT / 'results' / 'quadratic_sensitivity'
OUT_DIR = ROOT / 'report' / 'figs'
OUT_DIR.mkdir(parents=True, exist_ok=True)

rows = []
for alpha_dir in sorted(RES_DIR.glob('alpha_*')):
    alpha = float(alpha_dir.name.split('_',1)[1])
    for j in alpha_dir.glob('*.json'):
        try:
            with open(j,'r',encoding='utf-8') as f:
                data = json.load(f)
        except Exception as e:
            print('Failed to read', j, e)
            continue
        alg = data.get('algorithm', j.stem.split('_')[1] if '_' in j.stem else 'unknown')
        rows.append({
            'alpha': alpha,
            'algorithm': alg,
            'cost': data.get('cost'),
            'profit': data.get('total_profit'),
            'objective': data.get('objective'),
            'feasible': data.get('feasible'),
            'time': data.get('computation_time')
        })

if not rows:
    print('No data found in', RES_DIR)
    raise SystemExit(1)

df = pd.DataFrame(rows)
summary_csv = ROOT / 'results' / 'quadratic_sensitivity_summary.csv'
df.to_csv(summary_csv, index=False)
print('Wrote', summary_csv)

# Plot: objective vs alpha for each algorithm (line + points)
plt.figure(figsize=(8,5))
sns.lineplot(data=df, x='alpha', y='objective', hue='algorithm', marker='o')
plt.title('Objective vs alpha (quadratic term) — n100mosA')
plt.xlabel('alpha (coefficient for load^2)')
plt.ylabel('Objective (profit - cost)')
plt.tight_layout()
plt.savefig(OUT_DIR / 'quadratic_sensitivity_objective.png', dpi=200)
plt.savefig(OUT_DIR / 'quadratic_sensitivity_objective.pdf')
print('Saved plots to', OUT_DIR)

# Compute best algorithm per alpha (max objective)
best_per_alpha = df.groupby('alpha').apply(lambda g: g.loc[g['objective'].idxmax()])
best_per_alpha = best_per_alpha.reset_index(drop=True)
best_csv = ROOT / 'results' / 'quadratic_best_by_alpha.csv'
best_per_alpha.to_csv(best_csv, index=False)
print('Wrote', best_csv)

# Plot: best objective per alpha with algorithm label
plt.figure(figsize=(8,5))
sns.barplot(data=best_per_alpha, x='alpha', y='objective', hue='algorithm')
plt.title('Best algorithm per alpha (n100mosA)')
plt.xlabel('alpha')
plt.ylabel('Objective (best)')
plt.tight_layout()
plt.savefig(OUT_DIR / 'quadratic_best_by_alpha.png', dpi=200)
plt.savefig(OUT_DIR / 'quadratic_best_by_alpha.pdf')
print('Saved best-by-alpha plots to', OUT_DIR)

print('Done')
