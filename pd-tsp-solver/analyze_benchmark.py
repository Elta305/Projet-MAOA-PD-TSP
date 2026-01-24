#!/usr/bin/env python3
"""
Analyse des résultats du benchmark n100mosA
Génère des plots pour le rapport
"""

import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np

# Configuration
sns.set_style("whitegrid")
plt.rcParams['figure.figsize'] = (12, 8)
plt.rcParams['font.size'] = 11

# Charger les résultats
df = pd.read_csv('results/n100_benchmark/results.csv')

# Classifier les algorithmes
def classify_algorithm(alg):
    if any(x in alg for x in ['NearestNeighbor', 'GreedyInsertion', 'FarthestInsertion', 
                                'Savings', 'Sweep', 'Regret', 'ClusterFirst']):
        return 'Constructive'
    elif any(x in alg for x in ['2-Opt', 'Swap', 'Relocation', 'Or-Opt', 'VND']):
        return 'Local Search'
    elif any(x in alg for x in ['SA-', 'TabuSearch', 'ILS-', 'GA-', 'MA-', 'ACO-', 'MMAS-']):
        return 'Metaheuristic'
    elif 'Gurobi' in alg:
        return 'Exact'
    return 'Other'

df['category'] = df['algorithm'].apply(classify_algorithm)

# 1. Comparaison par catégorie
fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

# Coût moyen par catégorie
category_stats = df.groupby('category').agg({
    'cost': ['mean', 'min', 'max'],
    'time': 'mean'
}).round(2)

categories = category_stats.index
costs_mean = category_stats[('cost', 'mean')]
costs_min = category_stats[('cost', 'min')]
times = category_stats[('time', 'mean')]

x = np.arange(len(categories))
width = 0.35

bars1 = ax1.bar(x - width/2, costs_mean, width, label='Coût moyen', color='steelblue', alpha=0.7)
bars2 = ax1.bar(x + width/2, costs_min, width, label='Meilleur coût', color='darkgreen', alpha=0.7)

ax1.set_xlabel('Catégorie d\'algorithme')
ax1.set_ylabel('Coût')
ax1.set_title('Comparaison de la qualité des solutions par catégorie')
ax1.set_xticks(x)
ax1.set_xticklabels(categories, rotation=15)
ax1.legend()
ax1.grid(axis='y', alpha=0.3)

# Temps par catégorie
bars = ax2.barh(categories, times, color='coral', alpha=0.7)
ax2.set_xlabel('Temps moyen (s)')
ax2.set_title('Temps de calcul moyen par catégorie')
ax2.grid(axis='x', alpha=0.3)

plt.tight_layout()
plt.savefig('report/figs/benchmark_by_category.pdf', dpi=300, bbox_inches='tight')
plt.savefig('report/figs/benchmark_by_category.png', dpi=300, bbox_inches='tight')
print("✓ Saved benchmark_by_category.pdf/png")
plt.close()

# 2. Top 10 algorithmes
fig, ax = plt.subplots(figsize=(12, 8))
top10 = df.nsmallest(10, 'cost').sort_values('cost')
colors = ['darkgreen' if cat == 'Exact' else 'steelblue' if cat == 'Metaheuristic' 
          else 'coral' if cat == 'Local Search' else 'lightblue' 
          for cat in top10['category']]

ax.barh(range(len(top10)), top10['cost'], color=colors, alpha=0.7)
ax.set_yticks(range(len(top10)))
ax.set_yticklabels(top10['algorithm'])
ax.set_xlabel('Coût de la solution')
ax.set_title('Top 10 des meilleurs algorithmes sur n100mosA')
ax.grid(axis='x', alpha=0.3)

# Légende
from matplotlib.patches import Patch
legend_elements = [
    Patch(facecolor='darkgreen', alpha=0.7, label='Exact'),
    Patch(facecolor='steelblue', alpha=0.7, label='Metaheuristic'),
    Patch(facecolor='coral', alpha=0.7, label='Local Search'),
    Patch(facecolor='lightblue', alpha=0.7, label='Constructive')
]
ax.legend(handles=legend_elements, loc='lower right')

plt.tight_layout()
plt.savefig('report/figs/top10_algorithms.pdf', dpi=300, bbox_inches='tight')
plt.savefig('report/figs/top10_algorithms.png', dpi=300, bbox_inches='tight')
print("✓ Saved top10_algorithms.pdf/png")
plt.close()

# 3. Temps vs Qualité (scatter)
fig, ax = plt.subplots(figsize=(12, 8))

for cat in df['category'].unique():
    subset = df[df['category'] == cat]
    ax.scatter(subset['time'], subset['cost'], label=cat, s=100, alpha=0.6)

ax.set_xlabel('Temps de calcul (s)')
ax.set_ylabel('Coût de la solution')
ax.set_title('Compromis temps/qualité des algorithmes')
ax.set_xscale('log')
ax.legend()
ax.grid(True, alpha=0.3)

plt.tight_layout()
plt.savefig('report/figs/time_vs_quality.pdf', dpi=300, bbox_inches='tight')
plt.savefig('report/figs/time_vs_quality.png', dpi=300, bbox_inches='tight')
print("✓ Saved time_vs_quality.pdf/png")
plt.close()

# 4. Heuristiques constructives détaillées
constructive = df[df['category'] == 'Constructive'].sort_values('cost')
fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

ax1.barh(constructive['algorithm'], constructive['cost'], color='lightblue', alpha=0.7)
ax1.set_xlabel('Coût')
ax1.set_title('Qualité des heuristiques constructives')
ax1.grid(axis='x', alpha=0.3)

ax2.barh(constructive['algorithm'], constructive['time']*1000, color='coral', alpha=0.7)
ax2.set_xlabel('Temps (ms)')
ax2.set_title('Temps de calcul des heuristiques constructives')
ax2.grid(axis='x', alpha=0.3)

plt.tight_layout()
plt.savefig('report/figs/constructive_heuristics.pdf', dpi=300, bbox_inches='tight')
plt.savefig('report/figs/constructive_heuristics.png', dpi=300, bbox_inches='tight')
print("✓ Saved constructive_heuristics.pdf/png")
plt.close()

# 5. Métaheuristiques - Moyennes par type
metaheuristics = df[df['category'] == 'Metaheuristic'].copy()
metaheuristics['base_alg'] = metaheuristics['algorithm'].apply(lambda x: x.rsplit('-', 1)[0])
meta_stats = metaheuristics.groupby('base_alg').agg({
    'cost': ['mean', 'std', 'min', 'max'],
    'time': 'mean'
}).round(2)

fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

# Qualité moyenne avec écart-type
base_algs = meta_stats.index
means = meta_stats[('cost', 'mean')]
stds = meta_stats[('cost', 'std')]

ax1.bar(base_algs, means, yerr=stds, capsize=5, color='steelblue', alpha=0.7)
ax1.set_ylabel('Coût moyen ± std')
ax1.set_title('Performance des métaheuristiques (3 runs)')
ax1.set_xticklabels(base_algs, rotation=45)
ax1.grid(axis='y', alpha=0.3)

# Temps moyen
times = meta_stats[('time', 'mean')]
ax2.bar(base_algs, times, color='coral', alpha=0.7)
ax2.set_ylabel('Temps moyen (s)')
ax2.set_title('Temps de calcul des métaheuristiques')
ax2.set_xticklabels(base_algs, rotation=45)
ax2.grid(axis='y', alpha=0.3)

plt.tight_layout()
plt.savefig('report/figs/metaheuristics_comparison.pdf', dpi=300, bbox_inches='tight')
plt.savefig('report/figs/metaheuristics_comparison.png', dpi=300, bbox_inches='tight')
print("✓ Saved metaheuristics_comparison.pdf/png")
plt.close()

# 6. Résumé statistique
print("\n" + "="*70)
print("RÉSUMÉ STATISTIQUE")
print("="*70)
print("\nMeilleur algorithme:", df.loc[df['cost'].idxmin(), 'algorithm'])
print("Meilleur coût:", df['cost'].min())
print("\nMeilleur exact (Gurobi):")
exact = df[df['category'] == 'Exact'].iloc[0]
print(f"  Coût: {exact['cost']:.2f}")
print(f"  Gap: {exact['gap_to_best']:.2f}%")
print(f"  Lower bound: {exact['lower_bound']:.2f}")
print(f"  Temps: {exact['time']:.2f}s")

print("\nStatistiques par catégorie:")
print(category_stats)

print("\n✓ Toutes les visualisations ont été générées dans report/figs/")
