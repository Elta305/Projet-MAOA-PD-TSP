#!/usr/bin/env python3
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import os

sns.set(style='whitegrid')
# repository root (script is in report/)
repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
figs_dir = os.path.join(repo_root, 'report', 'figs')
os.makedirs(figs_dir, exist_ok=True)

# Load results
n100_csv = os.path.join(repo_root, 'results', 'benchmark_complete_n100', 'results.csv')
# dedicated quadratics CSV for n100 (try plural then singular)
n100_quadratic_csv = os.path.join(repo_root, 'results', 'benchmark_complete_n100', 'results_quadratics.csv')
n100_quadratic_csv_alt = os.path.join(repo_root, 'results', 'benchmark_complete_n100', 'results_quadratic.csv')
# use the project's dedicated beta results file
beta_csv = os.path.join(repo_root, 'results', 'benchmark_complete', 'results_beta.csv')
n200_csv = os.path.join(repo_root, 'results', 'benchmark_n200_quick', 'results_n200_quick.csv')

def safe_read(path):
    try:
        return pd.read_csv(path)
    except Exception as e:
        print(f'Could not read {path}: {e}')
        return None

df_n100 = safe_read(n100_csv)
df_beta = safe_read(beta_csv)
df_n200 = safe_read(n200_csv)
df_n100_quad = safe_read(n100_quadratic_csv)
if df_n100_quad is None:
    df_n100_quad = safe_read(n100_quadratic_csv_alt)

def normalize_algo(name):
    if pd.isna(name):
        return name
    s = str(name).lower()
    if 'exact' in s or 'gurobi' in s:
        return 'Exact'
    if 'memetic' in s:
        return 'Memetic'
    if 'genetic' in s or (s == 'ga') or ('ga' in s and 'm' not in s):
        return 'GA'
    if 'mmas' in s:
        return 'MMAS'
    if 'aco' in s and 'mmas' not in s:
        return 'ACO'
    if 'hybrid' in s:
        return 'Hybrid'
    # common short names
    if s in ['nn','nnheuristic','nearestneighbor']:
        return 'NN'
    return str(name)

# Plot: average cost by algorithm (n100)
if df_n100 is not None:
    agg = df_n100[df_n100['cost']!='N/A'].copy()
    agg['cost'] = agg['cost'].astype(float)
    agg['algorithm'] = agg['algorithm'].apply(normalize_algo)
    stats = agg.groupby('algorithm')['cost'].agg(['mean','std','min','count']).reset_index()
    plt.figure(figsize=(10,6))
    sns.barplot(data=stats.sort_values('mean'), x='mean', y='algorithm', palette='viridis')
    plt.xlabel('Coût moyen')
    plt.title('Coût moyen par algorithme (n100)')
    plt.tight_layout()
    plt.savefig(os.path.join(figs_dir, 'n100_cost_by_algo.png'), dpi=300)
    plt.close()

# Plot: feasibility rate by algorithm
if df_n100 is not None:
    df_n100['feasible_flag'] = df_n100['feasible'].astype(str).str.lower().map({'true':1,'false':0})
    df_n100['algorithm'] = df_n100['algorithm'].apply(normalize_algo)
    feas = df_n100.groupby('algorithm')['feasible_flag'].mean().reset_index()
    plt.figure(figsize=(10,6))
    sns.barplot(data=feas.sort_values('feasible_flag', ascending=False), x='feasible_flag', y='algorithm', palette='magma')
    plt.xlabel('Taux de faisabilité')
    plt.title('Taux de faisabilité par algorithme (n100)')
    plt.tight_layout()
    plt.savefig(os.path.join(figs_dir, 'n100_feasibility_by_algo.png'), dpi=300)
    plt.close()

# Plot: beta sensitivity (quadratic-cost runs)
if df_beta is not None:
    dfb = df_beta[df_beta['cost']!='N/A'].copy()
    dfb['cost'] = dfb['cost'].astype(float)
    # Select quadratic-cost rows only
    if 'cost_function' in dfb.columns:
        dfq = dfb[dfb['cost_function'].str.lower().isin(['quadratic','quad','quadratic_load'])]
    else:
        dfq = dfb.copy()
    # Fallback: also look in df_n100 for quadratic runs
    if dfq.empty and df_n100 is not None:
        if 'cost_function' in df_n100.columns:
            tmp = df_n100[df_n100['cost_function'].str.lower().isin(['quadratic','quad','quadratic_load'])]
        else:
            tmp = pd.DataFrame()
        if not tmp.empty:
            dfq = tmp.copy()
    if not dfq.empty:
        dfq = dfq.copy()
        dfq['algorithm'] = dfq['algorithm'].apply(normalize_algo)
        # ensure beta numeric
        if 'beta' in dfq.columns:
            dfq['beta'] = pd.to_numeric(dfq['beta'], errors='coerce')
        else:
            dfq['beta'] = 0.0
        if dfq['beta'].nunique() <= 1:
            print('Warning: single beta value found; beta sweep may not have been run.')
        plt.figure(figsize=(10,6))
        sns.lineplot(data=dfq, x='beta', y='cost', hue='algorithm', marker='o')
        # prefer log x-scale for beta visualization if positive
        try:
            if (dfq['beta'] > 0).any():
                plt.xscale('log')
        except Exception:
            pass
        plt.xlabel('Beta (log scale)')
        plt.ylabel('Coût')
        plt.title('Sensibilité de β (coût quadratique)')
        plt.tight_layout()
        plt.savefig(os.path.join(figs_dir, 'beta_sensitivity.png'), dpi=300)
        plt.close()

# Plot: n200 quick comparison
if df_n200 is not None:
    dfn = df_n200[df_n200['cost']!='N/A'].copy()
    dfn['cost'] = dfn['cost'].astype(float)
    plt.figure(figsize=(10,6))
    sns.barplot(data=dfn, x='cost', y='algorithm', palette='cubehelix')
    plt.xlabel('Coût')
    plt.title('Résultats quick n200')
    plt.tight_layout()
    plt.savefig(os.path.join(figs_dir, 'n200_quick_costs.png'), dpi=300)
    plt.close()

print('Plots saved to', figs_dir)

# ------- Additional analysis: linear -> quadratic deltas and beta sweep summary
def compute_linear_vs_quadratic(df_linear, df_quad):
    # df_linear: results.csv (linear-load), df_quad: results_quadratic.csv (beta=0.01)
    if df_linear is None or df_quad is None:
        return None
    # filter linear rows
    lin = df_linear[df_linear['cost']!='N/A'].copy()
    lin['algorithm'] = lin['algorithm'].apply(normalize_algo)
    lin = lin[lin['cost_function'].str.contains('linear', na=False) | lin['cost_function'].str.contains('distance', na=False)]
    lin['cost'] = lin['cost'].astype(float)
    lin_mean = lin.groupby('algorithm')['cost'].mean().reset_index().rename(columns={'cost':'cost_linear'})

    qd = df_quad[df_quad['cost']!='N/A'].copy()
    qd['algorithm'] = qd['algorithm'].apply(normalize_algo)
    qd = qd[qd['cost_function'].str.contains('quadratic', na=False)]
    qd['cost'] = qd['cost'].astype(float)
    qd_mean = qd.groupby('algorithm')['cost'].mean().reset_index().rename(columns={'cost':'cost_quadratic'})

    merged = pd.merge(lin_mean, qd_mean, on='algorithm', how='inner')
    merged['pct_delta'] = (merged['cost_quadratic'] - merged['cost_linear']) / merged['cost_linear'] * 100.0
    return merged.sort_values('pct_delta', ascending=False)

delta_df = compute_linear_vs_quadratic(df_n100, df_n100_quad)
if delta_df is not None and not delta_df.empty:
    csv_out = os.path.join(figs_dir, 'linear_vs_quadratic_deltas.csv')
    delta_df.to_csv(csv_out, index=False)
    plt.figure(figsize=(10,6))
    sns.barplot(data=delta_df.sort_values('pct_delta'), x='pct_delta', y='algorithm', palette='coolwarm')
    plt.xlabel('Variation relative (%) du coût (quadratique vs linéaire)')
    plt.title('Variation moyenne du coût: quadratique (β=0.01) vs linéaire (moyenne par algorithme)')
    plt.tight_layout()
    plt.savefig(os.path.join(figs_dir, 'linear_vs_quadratic_delta.png'), dpi=300)
    plt.close()

# Beta sweep comparative plot for selected betas
if df_beta is not None:
    db = df_beta[df_beta['cost']!='N/A'].copy()
    db = db[db['cost_function'].str.lower().str.contains('quadratic', na=False)]
    db['algorithm'] = db['algorithm'].apply(normalize_algo)
    db['beta'] = pd.to_numeric(db['beta'], errors='coerce')
    # select betas of interest
    betas_of_interest = [0.01, 0.1, 1.0]
    db_sel = db[db['beta'].isin(betas_of_interest)].copy()
    if not db_sel.empty:
        summary = db_sel.groupby(['beta','algorithm'])['cost'].mean().reset_index()
        plt.figure(figsize=(10,6))
        sns.lineplot(data=summary, x='beta', y='cost', hue='algorithm', marker='o')
        plt.xscale('log')
        plt.xlabel('β (log scale)')
        plt.ylabel('Coût moyen')
        plt.title('Sensibilité: coût moyen par algorithme pour β ∈ {0.01,0.1,1}')
        plt.tight_layout()
        plt.savefig(os.path.join(figs_dir, 'beta_sweep_compare.png'), dpi=300)
        plt.close()
        # also save the summary table
        summary.to_csv(os.path.join(figs_dir, 'beta_sweep_summary.csv'), index=False)
# Additional plot: time vs cost (linear & quadratic) with Pareto front
def pareto_front(df, x_col='time', y_col='cost'):
    # return non-dominated points (minimize both x and y)
    sub = df[[x_col, y_col]].dropna()
    pts = sub.values.tolist()
    orig_idxs = list(sub.index)
    pareto_idxs = []
    for i, (x_i, y_i) in enumerate(pts):
        dominated = False
        for j, (x_j, y_j) in enumerate(pts):
            if j == i:
                continue
            if x_j <= x_i and y_j <= y_i and (x_j < x_i or y_j < y_i):
                dominated = True
                break
        if not dominated:
            pareto_idxs.append(orig_idxs[i])
    # return rows from the original dataframe preserving labels
    return df.loc[pareto_idxs]

if df_n100 is not None:
    combined = df_n100[df_n100['cost']!='N/A'].copy()
    combined['cost'] = combined['cost'].astype(float)
    # normalize/parse time column
    if 'time' in combined.columns:
        combined['time'] = pd.to_numeric(combined['time'], errors='coerce')
    elif 'runtime' in combined.columns:
        combined['time'] = pd.to_numeric(combined['runtime'], errors='coerce')
    else:
        combined['time'] = pd.Series([None]*len(combined))
    combined['algorithm'] = combined['algorithm'].apply(normalize_algo)
    # select linear runs only (exclude quadratic)
    if 'cost_function' in combined.columns:
        combined = combined[combined['cost_function'].str.lower().isin(['distance','linear','linearload','linear-load','linear_load']) | combined['cost_function'].isna()]
    # Average across instances: compute mean time and mean cost per algorithm
    try:
        grouped = combined.groupby('algorithm')[['time','cost']].mean().reset_index()
    except Exception:
        grouped = combined[['algorithm','time','cost']].copy()
    plt.figure(figsize=(10,6))
    sns.scatterplot(data=grouped, x='time', y='cost', hue='algorithm', s=100)
    # compute pareto front on averaged points
    try:
        pf = pareto_front(grouped, x_col='time', y_col='cost')
        pf_sorted = pf.sort_values('time')
        plt.plot(pf_sorted['time'], pf_sorted['cost'], color='black', linestyle='--', label='Pareto front')
        plt.legend()
    except Exception as e:
        print('Pareto front failed:', e)
    plt.xlabel('Temps moyen (s)')
    plt.ylabel('Coût moyen')
    plt.title('Comparaison Temps moyen vs Coût moyen (linéaire) — moyenne sur instances')
    # Apply logarithmic scale on x (time) to show small time values more clearly
    try:
        if (grouped['time'].dropna() > 0).any():
            plt.xscale('log')
    except Exception:
        pass
    plt.tight_layout()
    plt.savefig(os.path.join(figs_dir, 'time_vs_cost_pareto.png'), dpi=300)
    plt.close()

# Additional Pareto plot for quadratic cost (prefer dedicated quadratics CSV if present)
if df_n100_quad is not None and not df_n100_quad.empty:
    src_q = df_n100_quad
else:
    src_q = df_n100
if src_q is not None:
    combined_q = src_q[src_q['cost']!='N/A'].copy()
    combined_q['cost'] = combined_q['cost'].astype(float)
    # normalize/parse time column
    if 'time' in combined_q.columns:
        combined_q['time'] = pd.to_numeric(combined_q['time'], errors='coerce')
    elif 'runtime' in combined_q.columns:
        combined_q['time'] = pd.to_numeric(combined_q['runtime'], errors='coerce')
    else:
        combined_q['time'] = pd.Series([None]*len(combined_q))
    combined_q['algorithm'] = combined_q['algorithm'].apply(normalize_algo)
    # select quadratic runs only
    if 'cost_function' in combined_q.columns:
        combined_q = combined_q[combined_q['cost_function'].str.lower().isin(['quadratic','quad','quadratic_load'])]
    # Average across instances: compute mean time and mean cost per algorithm
    try:
        grouped_q = combined_q.groupby('algorithm')[['time','cost']].mean().reset_index()
    except Exception:
        grouped_q = combined_q[['algorithm','time','cost']].copy()
    plt.figure(figsize=(10,6))
    sns.scatterplot(data=grouped_q, x='time', y='cost', hue='algorithm', s=100)
    # compute pareto front on averaged points
    try:
        pfq = pareto_front(grouped_q, x_col='time', y_col='cost')
        pfq_sorted = pfq.sort_values('time')
        plt.plot(pfq_sorted['time'], pfq_sorted['cost'], color='black', linestyle='--', label='Pareto front')
        plt.legend()
    except Exception as e:
        print('Pareto front (quadratic) failed:', e)
    plt.xlabel('Temps moyen (s)')
    plt.ylabel('Coût moyen (quadratique)')
    plt.title('Comparaison Temps moyen vs Coût moyen (quadratique) — moyenne sur instances')
    # Apply logarithmic scale on x (time)
    try:
        if (grouped_q['time'].dropna() > 0).any():
            plt.xscale('log')
    except Exception:
        pass
    plt.tight_layout()
    plt.savefig(os.path.join(figs_dir, 'time_vs_cost_pareto_quadratic.png'), dpi=300)
    plt.close()
