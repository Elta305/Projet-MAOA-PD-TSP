#!/usr/bin/env python3
"""
Generate comprehensive plots for PD-TSP benchmark results
"""

import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np
from pathlib import Path
import sys

sns.set_style("whitegrid")
plt.rcParams['figure.dpi'] = 300
plt.rcParams['savefig.dpi'] = 300
plt.rcParams['font.size'] = 10

def load_results(results_dir):
    """Load benchmark results from CSV"""
    results_path = Path(results_dir) / "results.csv"
    stats_path = Path(results_dir) / "statistics.csv"
    
    if not results_path.exists():
        print(f"Error: {results_path} not found")
        return None, None
    
    results = pd.read_csv(results_path)
    stats = pd.read_csv(stats_path) if stats_path.exists() else None
    
    return results, stats

def plot_algorithm_performance(stats, output_dir):
    """Plot average performance by algorithm"""
    fig, axes = plt.subplots(2, 2, figsize=(14, 10))
    
    # Sort by average cost
    stats_sorted = stats.sort_values('avg_cost')
    
    # Cost comparison
    ax = axes[0, 0]
    ax.barh(stats_sorted['algorithm'], stats_sorted['avg_cost'], color='steelblue')
    ax.set_xlabel('Average Cost')
    ax.set_title('Algorithm Performance - Average Cost')
    ax.grid(axis='x', alpha=0.3)
    
    # Time comparison
    ax = axes[0, 1]
    ax.barh(stats_sorted['algorithm'], stats_sorted['avg_time'], color='coral')
    ax.set_xlabel('Average Time (seconds)')
    ax.set_title('Algorithm Performance - Average Time')
    ax.set_xscale('log')
    ax.grid(axis='x', alpha=0.3)
    
    # Feasibility rate
    ax = axes[1, 0]
    stats_sorted['feasibility_rate'] = (stats_sorted['feasible'] / stats_sorted['total']) * 100
    ax.barh(stats_sorted['algorithm'], stats_sorted['feasibility_rate'], color='green', alpha=0.7)
    ax.set_xlabel('Feasibility Rate (%)')
    ax.set_title('Algorithm Feasibility Rate')
    ax.set_xlim(0, 105)
    ax.grid(axis='x', alpha=0.3)
    
    # Cost vs Time scatter
    ax = axes[1, 1]
    ax.scatter(stats['avg_time'], stats['avg_cost'], s=100, alpha=0.6)
    for idx, row in stats.iterrows():
        ax.annotate(row['algorithm'], (row['avg_time'], row['avg_cost']), 
                   fontsize=8, alpha=0.7)
    ax.set_xlabel('Average Time (seconds)')
    ax.set_ylabel('Average Cost')
    ax.set_title('Cost vs Time Trade-off')
    ax.set_xscale('log')
    ax.grid(alpha=0.3)
    
    plt.tight_layout()
    plt.savefig(output_dir / 'algorithm_performance.png', bbox_inches='tight')
    plt.close()
    print(f"Saved: {output_dir / 'algorithm_performance.png'}")

def plot_instance_size_scaling(results, output_dir):
    """Plot how algorithms scale with instance size"""
    results['instance_size'] = results['instance'].str.extract(r'n(\d+)').astype(int)
    
    fig, axes = plt.subplots(1, 2, figsize=(14, 5))
    
    # Group by algorithm and size
    grouped = results.groupby(['algorithm', 'instance_size']).agg({
        'cost': 'mean',
        'time': 'mean',
        'feasible': 'mean'
    }).reset_index()
    
    # Cost scaling
    ax = axes[0]
    for algo in grouped['algorithm'].unique():
        data = grouped[grouped['algorithm'] == algo]
        ax.plot(data['instance_size'], data['cost'], marker='o', label=algo, linewidth=2)
    ax.set_xlabel('Instance Size (number of nodes)')
    ax.set_ylabel('Average Cost')
    ax.set_title('Solution Quality vs Instance Size')
    ax.legend(bbox_to_anchor=(1.05, 1), loc='upper left', fontsize=8)
    ax.grid(alpha=0.3)
    
    # Time scaling
    ax = axes[1]
    for algo in grouped['algorithm'].unique():
        data = grouped[grouped['algorithm'] == algo]
        ax.plot(data['instance_size'], data['time'], marker='o', label=algo, linewidth=2)
    ax.set_xlabel('Instance Size (number of nodes)')
    ax.set_ylabel('Average Time (seconds)')
    ax.set_title('Computation Time vs Instance Size')
    ax.set_yscale('log')
    ax.legend(bbox_to_anchor=(1.05, 1), loc='upper left', fontsize=8)
    ax.grid(alpha=0.3)
    
    plt.tight_layout()
    plt.savefig(output_dir / 'size_scaling.png', bbox_inches='tight')
    plt.close()
    print(f"Saved: {output_dir / 'size_scaling.png'}")

def plot_distribution_analysis(results, output_dir):
    """Plot cost distribution for each algorithm"""
    fig, axes = plt.subplots(2, 1, figsize=(14, 10))
    
    # Box plot
    ax = axes[0]
    algorithms = sorted(results['algorithm'].unique())
    data_to_plot = [results[results['algorithm'] == algo]['cost'].values 
                    for algo in algorithms]
    bp = ax.boxplot(data_to_plot, labels=algorithms, patch_artist=True)
    for patch in bp['boxes']:
        patch.set_facecolor('lightblue')
    ax.set_ylabel('Cost')
    ax.set_title('Cost Distribution by Algorithm')
    ax.grid(axis='y', alpha=0.3)
    plt.setp(ax.xaxis.get_majorticklabels(), rotation=45, ha='right')
    
    # Violin plot for top algorithms
    ax = axes[1]
    top_algos = results.groupby('algorithm')['cost'].mean().nsmallest(8).index.tolist()
    data_violin = results[results['algorithm'].isin(top_algos)]
    parts = ax.violinplot([data_violin[data_violin['algorithm'] == algo]['cost'].values 
                           for algo in top_algos],
                          positions=range(len(top_algos)),
                          showmeans=True, showmedians=True)
    ax.set_xticks(range(len(top_algos)))
    ax.set_xticklabels(top_algos, rotation=45, ha='right')
    ax.set_ylabel('Cost')
    ax.set_title('Cost Distribution - Top 8 Algorithms')
    ax.grid(axis='y', alpha=0.3)
    
    plt.tight_layout()
    plt.savefig(output_dir / 'distribution_analysis.png', bbox_inches='tight')
    plt.close()
    print(f"Saved: {output_dir / 'distribution_analysis.png'}")

def plot_convergence_comparison(results, output_dir):
    """Plot convergence characteristics"""
    if 'iterations' not in results.columns:
        print("Skipping convergence plot - no iteration data")
        return
    
    fig, axes = plt.subplots(1, 2, figsize=(14, 5))
    
    # Filter metaheuristics with iteration data
    metaheuristics = results[results['iterations'].notna()]
    
    if len(metaheuristics) == 0:
        print("No convergence data available")
        return
    
    # Iterations vs Cost
    ax = axes[0]
    for algo in metaheuristics['algorithm'].unique():
        data = metaheuristics[metaheuristics['algorithm'] == algo]
        ax.scatter(data['iterations'], data['cost'], label=algo, alpha=0.6, s=50)
    ax.set_xlabel('Iterations')
    ax.set_ylabel('Cost')
    ax.set_title('Cost vs Iterations')
    ax.legend(fontsize=8)
    ax.grid(alpha=0.3)
    
    # Iterations distribution
    ax = axes[1]
    grouped = metaheuristics.groupby('algorithm')['iterations'].mean().sort_values()
    ax.barh(grouped.index, grouped.values, color='purple', alpha=0.7)
    ax.set_xlabel('Average Iterations')
    ax.set_title('Average Iterations by Algorithm')
    ax.grid(axis='x', alpha=0.3)
    
    plt.tight_layout()
    plt.savefig(output_dir / 'convergence_analysis.png', bbox_inches='tight')
    plt.close()
    print(f"Saved: {output_dir / 'convergence_analysis.png'}")

def generate_latex_table(stats, output_path):
    """Generate LaTeX table for report"""
    stats_sorted = stats.sort_values('avg_cost').head(15)
    
    latex = r"""\begin{table}[H]
\centering
\caption{Performance des algorithmes sur les instances de benchmark}
\label{tab:results}
\begin{tabular}{lrrrrr}
\toprule
\textbf{Algorithme} & \textbf{Coût moy.} & \textbf{Temps moy. (s)} & \textbf{Faisable} & \textbf{Total} & \textbf{Taux (\%)} \\
\midrule
"""
    
    for _, row in stats_sorted.iterrows():
        rate = (row['feasible'] / row['total']) * 100 if row['total'] > 0 else 0
        latex += f"{row['algorithm']} & {row['avg_cost']:.2f} & {row['avg_time']:.4f} & "
        latex += f"{row['feasible']:.0f} & {row['total']:.0f} & {rate:.1f} \\\\\n"
    
    latex += r"""\bottomrule
\end{tabular}
\end{table}
"""
    
    with open(output_path, 'w', encoding='utf-8') as f:
        f.write(latex)
    
    print(f"Saved LaTeX table: {output_path}")

def main():
    if len(sys.argv) < 2:
        print("Usage: python generate_plots.py <results_directory>")
        sys.exit(1)
    
    results_dir = Path(sys.argv[1])
    
    if not results_dir.exists():
        print(f"Error: {results_dir} does not exist")
        sys.exit(1)
    
    print(f"Loading results from {results_dir}...")
    results, stats = load_results(results_dir)
    
    if results is None:
        sys.exit(1)
    
    # Create plots directory
    plots_dir = results_dir / "plots"
    plots_dir.mkdir(exist_ok=True)
    
    print("\nGenerating visualizations...")
    
    if stats is not None:
        plot_algorithm_performance(stats, plots_dir)
        generate_latex_table(stats, plots_dir / "results_table.tex")
    
    if len(results) > 0:
        plot_instance_size_scaling(results, plots_dir)
        plot_distribution_analysis(results, plots_dir)
        plot_convergence_comparison(results, plots_dir)
    
    print("\n✓ All visualizations generated successfully!")
    print(f"  Output directory: {plots_dir}")

if __name__ == "__main__":
    main()
