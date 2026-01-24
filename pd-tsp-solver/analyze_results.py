#!/usr/bin/env python3
"""
Script pour analyser les résultats du benchmark et générer des visualisations
Prend en compte les différentes fonctions de coût (distance, quadratic, linear-load)
"""

import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import os
import sys
import numpy as np

# Configuration
sns.set_style("whitegrid")
sns.set_palette("husl")
plt.rcParams['figure.figsize'] = (14, 10)
plt.rcParams['font.size'] = 10

def load_results(results_dir='results_complete'):
    """Charge tous les fichiers CSV de résultats"""
    results = {}
    
    quick_file = os.path.join(results_dir, 'quick_test_all_costs.csv')
    if os.path.exists(quick_file):
        results['quick'] = pd.read_csv(quick_file)
        print(f"✓ Loaded {len(results['quick'])} quick test results")
    
    exact_file = os.path.join(results_dir, 'exact_solver_all_costs.csv')
    if os.path.exists(exact_file):
        results['exact'] = pd.read_csv(exact_file)
        print(f"✓ Loaded {len(results['exact'])} exact solver results")
    
    constructive_file = os.path.join(results_dir, 'constructive_all_costs.csv')
    if os.path.exists(constructive_file):
        results['constructive'] = pd.read_csv(constructive_file)
        print(f"✓ Loaded {len(results['constructive'])} constructive results")
    
    metaheuristic_file = os.path.join(results_dir, 'metaheuristics_all_costs.csv')
    if os.path.exists(metaheuristic_file):
        results['metaheuristic'] = pd.read_csv(metaheuristic_file)
        print(f"✓ Loaded {len(results['metaheuristic'])} metaheuristic results")
    
    return results

def plot_cost_function_comparison(df, output_dir='results_complete'):
    """Compare les coûts selon la fonction de coût pour chaque algorithme"""
    if df is None or df.empty:
        print("⚠ No data for cost function comparison")
        return
    
    plt.figure(figsize=(16, 10))
    
    # Grouper par algorithme et fonction de coût
    grouped = df.groupby(['Algorithm', 'CostFunction'])['Cost'].mean().reset_index()
    
    # Créer un graphique pour chaque fonction de coût
    cost_funcs = grouped['CostFunction'].unique()
    n_funcs = len(cost_funcs)
    
    for idx, cost_func in enumerate(cost_funcs, 1):
        plt.subplot(2, 2, idx)
        data = grouped[grouped['CostFunction'] == cost_func].sort_values('Cost')
        
        plt.barh(data['Algorithm'], data['Cost'], color='steelblue', alpha=0.7)
        plt.xlabel('Coût moyen')
        plt.title(f'Fonction de coût: {cost_func}')
        plt.grid(True, alpha=0.3, axis='x')
    
    plt.tight_layout()
    output_file = os.path.join(output_dir, 'cost_function_comparison.png')
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"✓ Saved {output_file}")
    plt.close()

def plot_constructive_vs_metaheuristic(constructive_df, metaheuristic_df, output_dir='results_complete'):
    """Compare heuristiques constructives vs métaheuristiques"""
    if constructive_df is None or metaheuristic_df is None:
        print("⚠ No data for constructive vs metaheuristic comparison")
        return
    
    # Combiner les données
    constructive_df['Type'] = 'Constructive'
    metaheuristic_df['Type'] = 'Metaheuristic'
    combined = pd.concat([constructive_df, metaheuristic_df])
    
    plt.figure(figsize=(16, 8))
    
    # Pour chaque fonction de coût
    cost_funcs = combined['CostFunction'].unique()
    for idx, cost_func in enumerate(cost_funcs, 1):
        plt.subplot(2, 2, idx)
        data = combined[combined['CostFunction'] == cost_func]
        
        # Box plot
        sns.boxplot(data=data, x='Type', y='AvgCost', hue='Type')
        plt.title(f'Fonction: {cost_func}')
        plt.ylabel('Coût moyen')
        plt.xlabel('')
    
    plt.tight_layout()
    output_file = os.path.join(output_dir, 'constructive_vs_metaheuristic.png')
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"✓ Saved {output_file}")
    plt.close()

def plot_exact_vs_best_heuristic(exact_df, heuristic_df, output_dir='results_complete'):
    """Compare solution exacte vs meilleure heuristique"""
    if exact_df is None or exact_df.empty or heuristic_df is None or heuristic_df.empty:
        print("⚠ No data for exact vs heuristic comparison")
        return
    
    plt.figure(figsize=(16, 10))
    
    cost_funcs = exact_df['CostFunction'].unique()
    
    for idx, cost_func in enumerate(cost_funcs, 1):
        plt.subplot(2, 2, idx)
        
        exact_data = exact_df[exact_df['CostFunction'] == cost_func]
        heuristic_data = heuristic_df[heuristic_df['CostFunction'] == cost_func]
        
        # Trouver les instances communes
        common_instances = set(exact_data['Instance']) & set(heuristic_data['Instance'])
        
        if not common_instances:
            continue
        
        gaps = []
        instances = []
        exact_costs = []
        heuristic_costs = []
        
        for inst in sorted(common_instances):
            exact_cost = exact_data[exact_data['Instance'] == inst]['Cost'].iloc[0]
            
            # Prendre le meilleur coût parmi toutes les heuristiques
            heuristic_cost = heuristic_data[heuristic_data['Instance'] == inst]['MinCost'].min()
            
            if pd.notna(exact_cost) and pd.notna(heuristic_cost) and exact_cost > 0:
                gap = ((heuristic_cost - exact_cost) / exact_cost) * 100
                gaps.append(gap)
                instances.append(inst)
                exact_costs.append(exact_cost)
                heuristic_costs.append(heuristic_cost)
        
        if gaps:
            x = np.arange(len(instances))
            width = 0.35
            
            plt.bar(x - width/2, exact_costs, width, label='Exact', alpha=0.8)
            plt.bar(x + width/2, heuristic_costs, width, label='Best Heuristic', alpha=0.8)
            
            plt.xlabel('Instance')
            plt.ylabel('Coût')
            plt.title(f'Exact vs Heuristic ({cost_func})\nGap moyen: {sum(gaps)/len(gaps):.2f}%')
            plt.xticks(x, instances, rotation=45, ha='right')
            plt.legend()
            plt.grid(True, alpha=0.3, axis='y')
    
    plt.tight_layout()
    output_file = os.path.join(output_dir, 'exact_vs_heuristic_all_costs.png')
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"✓ Saved {output_file}")
    plt.close()

def plot_algorithm_ranking(constructive_df, metaheuristic_df, output_dir='results_complete'):
    """Classement des algorithmes par fonction de coût"""
    if constructive_df is None or metaheuristic_df is None:
        print("⚠ No data for algorithm ranking")
        return
    
    combined = pd.concat([constructive_df, metaheuristic_df])
    
    plt.figure(figsize=(16, 12))
    
    cost_funcs = combined['CostFunction'].unique()
    
    for idx, cost_func in enumerate(cost_funcs, 1):
        plt.subplot(2, 2, idx)
        
        data = combined[combined['CostFunction'] == cost_func]
        ranking = data.groupby('Algorithm')['AvgCost'].mean().sort_values()
        
        colors = ['gold' if i == 0 else 'silver' if i == 1 else 'chocolate' if i == 2 else 'steelblue' 
                  for i in range(len(ranking))]
        
        plt.barh(ranking.index, ranking.values, color=colors, alpha=0.8)
        plt.xlabel('Coût moyen')
        plt.title(f'Classement - {cost_func}')
        plt.grid(True, alpha=0.3, axis='x')
    
    plt.tight_layout()
    output_file = os.path.join(output_dir, 'algorithm_ranking.png')
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"✓ Saved {output_file}")
    plt.close()

def plot_performance_profiles(constructive_df, metaheuristic_df, exact_df, output_dir='results_complete'):
    """Profils de performance (temps vs qualité)"""
    if constructive_df is None or metaheuristic_df is None:
        print("⚠ No data for performance profiles")
        return
    
    combined = pd.concat([constructive_df, metaheuristic_df])
    
    plt.figure(figsize=(16, 10))
    
    cost_funcs = combined['CostFunction'].unique()
    
    for idx, cost_func in enumerate(cost_funcs, 1):
        plt.subplot(2, 2, idx)
        
        data = combined[combined['CostFunction'] == cost_func]
        
        for algo in data['Algorithm'].unique():
            algo_data = data[data['Algorithm'] == algo]
            avg_cost = algo_data['AvgCost'].mean()
            avg_time = algo_data['AvgTime'].mean()
            plt.scatter(avg_time, avg_cost, s=150, alpha=0.7, label=algo)
        
        plt.xlabel('Temps moyen (s)')
        plt.ylabel('Coût moyen')
        plt.title(f'Performance - {cost_func}')
        plt.legend(bbox_to_anchor=(1.05, 1), loc='upper left', fontsize=8)
        plt.grid(True, alpha=0.3)
        plt.xscale('log')
    
    plt.tight_layout()
    output_file = os.path.join(output_dir, 'performance_profiles.png')
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"✓ Saved {output_file}")
    plt.close()

def generate_latex_tables(exact_df, constructive_df, metaheuristic_df, output_dir='results_complete'):
    """Génère des tableaux LaTeX récapitulatifs"""
    
    # Table 1: Exact solver results
    if exact_df is not None and not exact_df.empty:
        output_file = os.path.join(output_dir, 'exact_results_table.tex')
        with open(output_file, 'w', encoding='utf-8') as f:
            f.write("\\begin{table}[H]\n")
            f.write("\\centering\n")
            f.write("\\caption{Résultats du solveur exact (Gurobi)}\n")
            f.write("\\begin{tabular}{llcccc}\n")
            f.write("\\toprule\n")
            f.write("Instance & Fonction & Coût & Optimal & Gap (\\%) & Temps (s) \\\\\n")
            f.write("\\midrule\n")
            
            for _, row in exact_df.iterrows():
                f.write(f"{row['Instance']} & {row['CostFunction']} & {row['Cost']:.2f} & "
                       f"{row['Optimal']} & {row['Gap']} & {row['Time']} \\\\\n")
            
            f.write("\\bottomrule\n")
            f.write("\\end{tabular}\n")
            f.write("\\label{tab:exact_results}\n")
            f.write("\\end{table}\n")
        print(f"✓ Saved {output_file}")
    
    # Table 2: Best algorithms per cost function
    if constructive_df is not None and metaheuristic_df is not None:
        combined = pd.concat([constructive_df, metaheuristic_df])
        output_file = os.path.join(output_dir, 'best_algorithms_table.tex')
        
        with open(output_file, 'w', encoding='utf-8') as f:
            f.write("\\begin{table}[H]\n")
            f.write("\\centering\n")
            f.write("\\caption{Meilleurs algorithmes par fonction de coût}\n")
            f.write("\\begin{tabular}{lllcc}\n")
            f.write("\\toprule\n")
            f.write("Fonction de coût & Algorithme & Coût moyen & Coût min & Temps (s) \\\\\n")
            f.write("\\midrule\n")
            
            for cost_func in combined['CostFunction'].unique():
                data = combined[combined['CostFunction'] == cost_func]
                best = data.groupby('Algorithm')['AvgCost'].mean().sort_values().head(5)
                
                for algo in best.index:
                    algo_data = data[data['Algorithm'] == algo]
                    avg_cost = algo_data['AvgCost'].mean()
                    min_cost = algo_data['MinCost'].min()
                    avg_time = algo_data['AvgTime'].mean()
                    f.write(f"{cost_func} & {algo} & {avg_cost:.2f} & {min_cost:.2f} & {avg_time:.4f} \\\\\n")
                
                f.write("\\midrule\n")
            
            f.write("\\bottomrule\n")
            f.write("\\end{tabular}\n")
            f.write("\\label{tab:best_algorithms}\n")
            f.write("\\end{table}\n")
        print(f"✓ Saved {output_file}")

def main():
    print("="*70)
    print("  PD-TSP Solver - Analyse Complète des Résultats")
    print("="*70)
    print()
    
    output_dir = 'results_complete'
    os.makedirs(output_dir, exist_ok=True)
    
    results = load_results(output_dir)
    
    if not results:
        print("❌ Aucun résultat trouvé!")
        print("   Veuillez d'abord exécuter le script Python `scripts/run_benchmarks.py` pour générer les résultats")
        return 1
    
    print("\n" + "="*70)
    print("  Génération des visualisations")
    print("="*70)
    print()
    
    # Visualisations
    if 'quick' in results:
        plot_cost_function_comparison(results['quick'], output_dir)
    
    if 'constructive' in results and 'metaheuristic' in results:
        plot_constructive_vs_metaheuristic(results['constructive'], results['metaheuristic'], output_dir)
        plot_algorithm_ranking(results['constructive'], results['metaheuristic'], output_dir)
        plot_performance_profiles(results['constructive'], results['metaheuristic'], 
                                 results.get('exact'), output_dir)
    
    if 'exact' in results:
        heuristic_df = pd.concat([results.get('constructive', pd.DataFrame()), 
                                 results.get('metaheuristic', pd.DataFrame())])
        if not heuristic_df.empty:
            plot_exact_vs_best_heuristic(results['exact'], heuristic_df, output_dir)
    
    # Tableaux LaTeX
    print("\n" + "="*70)
    print("  Génération des tableaux LaTeX")
    print("="*70)
    print()
    
    generate_latex_tables(results.get('exact'), results.get('constructive'), 
                         results.get('metaheuristic'), output_dir)
    
    print("\n" + "="*70)
    print("  Analyse terminée!")
    print("="*70)
    print(f"\nTous les fichiers générés sont dans: {output_dir}/")
    print("\nFichiers PNG générés:")
    print("  - cost_function_comparison.png: Comparaison des fonctions de coût")
    print("  - constructive_vs_metaheuristic.png: Constructives vs Métaheuristiques")
    print("  - algorithm_ranking.png: Classement des algorithmes")
    print("  - performance_profiles.png: Profils performance (temps vs qualité)")
    print("  - exact_vs_heuristic_all_costs.png: Comparaison exact vs heuristiques")
    print("\nFichiers LaTeX générés:")
    print("  - exact_results_table.tex: Résultats du solveur exact")
    print("  - best_algorithms_table.tex: Meilleurs algorithmes par fonction de coût")
    
    return 0

if __name__ == '__main__':
    sys.exit(main())
