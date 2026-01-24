//! Benchmarking and experimentation module for PD-TSP.
//! 
//! Provides tools for running experiments, collecting statistics,
//! and comparing algorithm performance.

use crate::instance::PDTSPInstance;
use crate::solution::Solution;
use crate::heuristics::construction::*;
use crate::heuristics::local_search::*;
use crate::heuristics::genetic::{GeneticAlgorithm, GAConfig, MemeticAlgorithm};
use crate::heuristics::aco::{AntColonyOptimization, ACOConfig, MaxMinAntSystem};
use crate::exact::{GurobiSolver, GurobiConfig, ExactResult};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

/// Result of running a single algorithm on an instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmResult {
    /// Algorithm name
    pub algorithm: String,
    /// Instance name
    pub instance: String,
    /// Instance dimension
    pub dimension: usize,
    /// Instance capacity
    pub capacity: i32,
    /// Solution cost
    pub cost: f64,
    /// Whether solution is feasible
    pub feasible: bool,
    /// Computation time in seconds
    pub time: f64,
    /// Number of iterations (if applicable)
    pub iterations: Option<usize>,
    /// Gap to best known (if available)
    pub gap_to_best: Option<f64>,
    /// Lower bound (if available)
    pub lower_bound: Option<f64>,
}

/// Aggregated statistics for an algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmStatistics {
    /// Algorithm name
    pub algorithm: String,
    /// Number of instances solved
    pub num_instances: usize,
    /// Number of feasible solutions
    pub num_feasible: usize,
    /// Average cost
    pub avg_cost: f64,
    /// Best cost
    pub best_cost: f64,
    /// Worst cost
    pub worst_cost: f64,
    /// Standard deviation of cost
    pub std_cost: f64,
    /// Average time
    pub avg_time: f64,
    /// Total time
    pub total_time: f64,
    /// Average gap to best known
    pub avg_gap: Option<f64>,
}

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Number of runs per algorithm (for stochastic methods)
    pub num_runs: usize,
    /// Time limit per run in seconds
    pub time_limit: f64,
    /// Run exact solver
    pub run_exact: bool,
    /// Exact solver time limit
    pub exact_time_limit: f64,
    /// Run in parallel
    pub parallel: bool,
    /// Save intermediate results
    pub save_results: bool,
    /// Output directory
    pub output_dir: String,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        BenchmarkConfig {
            num_runs: 5,
            time_limit: 60.0,
            run_exact: false,
            exact_time_limit: 300.0,
            parallel: true,
            save_results: true,
            output_dir: "results".to_string(),
        }
    }
}

/// Benchmarking engine
pub struct Benchmark {
    config: BenchmarkConfig,
    results: Vec<AlgorithmResult>,
    best_known: HashMap<String, f64>,
}

impl Benchmark {
    pub fn new(config: BenchmarkConfig) -> Self {
        Benchmark {
            config,
            results: Vec::new(),
            best_known: HashMap::new(),
        }
    }
    
    /// Set best known solution for an instance
    pub fn set_best_known(&mut self, instance_name: &str, cost: f64) {
        self.best_known.insert(instance_name.to_string(), cost);
    }
    
    /// Run all construction heuristics on an instance
    pub fn run_construction_heuristics(&mut self, instance: &PDTSPInstance) {
        let heuristics: Vec<Box<dyn ConstructionHeuristic + Send + Sync>> = vec![
            Box::new(NearestNeighborHeuristic::new()),
            Box::new(GreedyInsertionHeuristic::new()),
            Box::new(GreedyInsertionHeuristic::farthest()),
            Box::new(SavingsHeuristic::new()),
            Box::new(SweepHeuristic::new()),
            Box::new(RegretInsertionHeuristic::new(2)),
            Box::new(RegretInsertionHeuristic::new(3)),
            Box::new(ClusterFirstHeuristic::new()),
        ];
        
        for heuristic in heuristics {
            let solution = heuristic.construct(instance);
            self.record_result(instance, &solution);
        }
    }
    
    /// Run all local search methods on an initial solution
    pub fn run_local_search(&mut self, instance: &PDTSPInstance, initial: Solution) {
        let searches: Vec<(&str, Box<dyn LocalSearch + Send + Sync>)> = vec![
            ("2-Opt", Box::new(TwoOptSearch::new())),
            ("Swap", Box::new(SwapSearch::new())),
            ("Relocation", Box::new(RelocationSearch::new())),
            ("Or-Opt", Box::new(OrOptSearch::new())),
            ("VND", Box::new(VND::with_standard_operators())),
        ];
        
        for (name, search) in searches {
            let mut solution = initial.clone();
            let start = std::time::Instant::now();
            search.improve(instance, &mut solution);
            solution.computation_time = start.elapsed().as_secs_f64();
            solution.algorithm = format!("{} + {}", initial.algorithm, name);
            self.record_result(instance, &solution);
        }
    }
    
    /// Run metaheuristics on an instance
    pub fn run_metaheuristics(&mut self, instance: &PDTSPInstance) {
        
        for seed in 0..self.config.num_runs {
            let mut sa = SimulatedAnnealing::new();
            sa.seed = seed as u64;
            
            let mut solution = self.get_initial_solution(instance);
            let start = std::time::Instant::now();
            sa.improve(instance, &mut solution);
            solution.computation_time = start.elapsed().as_secs_f64();
            solution.algorithm = format!("SA-run{}", seed);
            self.record_result(instance, &solution);
        }
        
        
        let ts = TabuSearch::new();
        let mut solution = self.get_initial_solution(instance);
        let start = std::time::Instant::now();
        ts.improve(instance, &mut solution);
        solution.computation_time = start.elapsed().as_secs_f64();
        solution.algorithm = "TabuSearch".to_string();
        self.record_result(instance, &solution);
        
        
        for seed in 0..self.config.num_runs {
            let mut ils = IteratedLocalSearch::new();
            ils.seed = seed as u64;
            
            let mut solution = self.get_initial_solution(instance);
            let start = std::time::Instant::now();
            ils.improve(instance, &mut solution);
            solution.computation_time = start.elapsed().as_secs_f64();
            solution.algorithm = format!("ILS-run{}", seed);
            self.record_result(instance, &solution);
        }
        
        
        for seed in 0..self.config.num_runs {
            let ga_config = GAConfig {
            seed: seed as u64,
            population_size: 50,
            max_generations: 200,
            time_limit: self.config.time_limit,
            ..Default::default()
            };

            let mut ga = GeneticAlgorithm::new(instance.clone(), ga_config);
            let solution = ga.run();

            let mut result = AlgorithmResult {
            algorithm: format!("GA-run{}", seed),
            instance: instance.name.clone(),
            dimension: instance.dimension,
            capacity: instance.capacity,
            cost: solution.cost,
            feasible: solution.feasible,
            time: solution.computation_time,
            iterations: solution.iterations,
            gap_to_best: None,
            lower_bound: None,
            };

            if let Some(&best) = self.best_known.get(&instance.name) {
            result.gap_to_best = Some((result.cost - best) / best * 100.0);
            }

            self.results.push(result);
        }
        
        for seed in 0..self.config.num_runs {
            let ga_config = GAConfig {
                seed: seed as u64,
                time_limit: self.config.time_limit,
                ..Default::default()
            };
            
            let mut ma = MemeticAlgorithm::with_config(instance.clone(), ga_config);
            let solution = ma.run();
            
            let mut result = AlgorithmResult {
                algorithm: format!("MA-run{}", seed),
                instance: instance.name.clone(),
                dimension: instance.dimension,
                capacity: instance.capacity,
                cost: solution.cost,
                feasible: solution.feasible,
                time: solution.computation_time,
                iterations: solution.iterations,
                gap_to_best: None,
                lower_bound: None,
            };
            
            if let Some(&best) = self.best_known.get(&instance.name) {
                result.gap_to_best = Some((result.cost - best) / best * 100.0);
            }
            
            self.results.push(result);
        }
        
        
        for seed in 0..self.config.num_runs {
            let aco_config = ACOConfig {
                seed: seed as u64,
                num_ants: 15,
                max_iterations: 100,
                time_limit: self.config.time_limit,
                ..Default::default()
            };
            
            let mut aco = AntColonyOptimization::new(instance.clone(), aco_config);
            let solution = aco.run();
            
            let mut result = AlgorithmResult {
                algorithm: format!("ACO-run{}", seed),
                instance: instance.name.clone(),
                dimension: instance.dimension,
                capacity: instance.capacity,
                cost: solution.cost,
                feasible: solution.feasible,
                time: solution.computation_time,
                iterations: solution.iterations,
                gap_to_best: None,
                lower_bound: None,
            };
            
            if let Some(&best) = self.best_known.get(&instance.name) {
                result.gap_to_best = Some((result.cost - best) / best * 100.0);
            }
            
            self.results.push(result);
        }
        
        
        for seed in 0..self.config.num_runs {
            let aco_config = ACOConfig {
                seed: seed as u64,
                num_ants: 15,
                max_iterations: 100,
                time_limit: self.config.time_limit,
                ..Default::default()
            };
            
            let mut mmas = MaxMinAntSystem::new(instance.clone(), aco_config);
            let solution = mmas.run();
            
            let mut result = AlgorithmResult {
                algorithm: format!("MMAS-run{}", seed),
                instance: instance.name.clone(),
                dimension: instance.dimension,
                capacity: instance.capacity,
                cost: solution.cost,
                feasible: solution.feasible,
                time: solution.computation_time,
                iterations: solution.iterations,
                gap_to_best: None,
                lower_bound: None,
            };
            
            if let Some(&best) = self.best_known.get(&instance.name) {
                result.gap_to_best = Some((result.cost - best) / best * 100.0);
            }
            
            self.results.push(result);
        }
    }
    
    /// Run exact solver on instance
    pub fn run_exact(&mut self, instance: &PDTSPInstance) -> Option<ExactResult> {
        if !self.config.run_exact {
            return None;
        }
        
        
        let initial = self.get_initial_solution(instance);
        let vnd = VND::with_standard_operators();
        let mut warm_solution = initial.clone();
        vnd.improve(instance, &mut warm_solution);
        
        let gurobi_config = GurobiConfig {
            time_limit: self.config.exact_time_limit,
            verbose: false,
            warm_start: Some(warm_solution.tour.clone()),
            ..Default::default()
        };
        
        let solver = GurobiSolver::new(gurobi_config);
        
        match solver.solve(instance) {
            Ok(result) => {
                
                if result.solution.feasible {
                    self.best_known.insert(instance.name.clone(), result.upper_bound);
                }
                
                let alg_result = AlgorithmResult {
                    algorithm: "Gurobi-Exact".to_string(),
                    instance: instance.name.clone(),
                    dimension: instance.dimension,
                    capacity: instance.capacity,
                    cost: result.upper_bound,
                    feasible: result.solution.feasible,
                    time: result.solution.computation_time,
                    iterations: None,
                    gap_to_best: Some(result.gap * 100.0),
                    lower_bound: Some(result.lower_bound),
                };
                
                self.results.push(alg_result);
                Some(result)
            }
            Err(e) => {
                log::error!("Gurobi solver failed: {}", e);
                None
            }
        }
    }
    
    /// Run full benchmark on an instance
    pub fn run_full_benchmark(&mut self, instance: &PDTSPInstance) {
        log::info!("Running benchmark on instance: {}", instance.name);
        
        
        self.run_construction_heuristics(instance);
        
        
        let best_construction = self.get_initial_solution(instance);
        self.run_local_search(instance, best_construction);
        
        
        self.run_metaheuristics(instance);
        
        
        self.run_exact(instance);
    }
    
    /// Run benchmark on multiple instances
    pub fn run_on_instances(&mut self, instances: &[PDTSPInstance]) {
        if self.config.parallel {
            
            
            for instance in instances {
                self.run_full_benchmark(instance);
            }
        } else {
            for instance in instances {
                self.run_full_benchmark(instance);
            }
        }
    }
    
    /// Get initial solution using multi-start construction
    fn get_initial_solution(&self, instance: &PDTSPInstance) -> Solution {
        let multi = MultiStartConstruction::with_all_heuristics();
        multi.construct(instance)
    }
    
    /// Record a result
    fn record_result(&mut self, instance: &PDTSPInstance, solution: &Solution) {
        let mut result = AlgorithmResult {
            algorithm: solution.algorithm.clone(),
            instance: instance.name.clone(),
            dimension: instance.dimension,
            capacity: instance.capacity,
            cost: solution.cost,
            feasible: solution.feasible,
            time: solution.computation_time,
            iterations: solution.iterations,
            gap_to_best: None,
            lower_bound: None,
        };
        
        if let Some(&best) = self.best_known.get(&instance.name) {
            result.gap_to_best = Some((result.cost - best) / best * 100.0);
        }
        
        self.results.push(result);
    }
    
    /// Compute statistics for each algorithm
    pub fn compute_statistics(&self) -> Vec<AlgorithmStatistics> {
        let mut stats_map: HashMap<String, Vec<&AlgorithmResult>> = HashMap::new();
        
        
        for result in &self.results {
            stats_map.entry(result.algorithm.clone())
                .or_insert_with(Vec::new)
                .push(result);
        }
        
        let mut statistics = Vec::new();
        
        for (algo, results) in stats_map {
            let feasible_results: Vec<_> = results.iter()
                .filter(|r| r.feasible)
                .collect();
            
            if feasible_results.is_empty() {
                continue;
            }
            
            let costs: Vec<f64> = feasible_results.iter().map(|r| r.cost).collect();
            let times: Vec<f64> = feasible_results.iter().map(|r| r.time).collect();
            let gaps: Vec<f64> = feasible_results.iter()
                .filter_map(|r| r.gap_to_best)
                .collect();
            
            let avg_cost = costs.iter().sum::<f64>() / costs.len() as f64;
            let best_cost = costs.iter().cloned().fold(f64::INFINITY, f64::min);
            let worst_cost = costs.iter().cloned().fold(0.0, f64::max);
            
            let variance = costs.iter()
                .map(|c| (c - avg_cost).powi(2))
                .sum::<f64>() / costs.len() as f64;
            let std_cost = variance.sqrt();
            
            let avg_time = times.iter().sum::<f64>() / times.len() as f64;
            let total_time = times.iter().sum::<f64>();
            
            let avg_gap = if !gaps.is_empty() {
                Some(gaps.iter().sum::<f64>() / gaps.len() as f64)
            } else {
                None
            };
            
            statistics.push(AlgorithmStatistics {
                algorithm: algo,
                num_instances: results.len(),
                num_feasible: feasible_results.len(),
                avg_cost,
                best_cost,
                worst_cost,
                std_cost,
                avg_time,
                total_time,
                avg_gap,
            });
        }
        
        
        statistics.sort_by(|a, b| a.avg_cost.partial_cmp(&b.avg_cost).unwrap());
        
        statistics
    }
    
    /// Export results to CSV
    pub fn export_to_csv<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let file = File::create(path)?;
        let mut writer = csv::Writer::from_writer(file);
        
        for result in &self.results {
            writer.serialize(result)?;
        }
        
        writer.flush()?;
        Ok(())
    }
    
    /// Export statistics to CSV
    pub fn export_statistics_csv<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let file = File::create(path)?;
        let mut writer = csv::Writer::from_writer(file);
        
        let stats = self.compute_statistics();
        for stat in stats {
            writer.serialize(stat)?;
        }
        
        writer.flush()?;
        Ok(())
    }
    
    /// Generate summary report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("========================================\n");
        report.push_str("       PD-TSP Benchmark Report\n");
        report.push_str("========================================\n\n");
        
        let stats = self.compute_statistics();
        
        report.push_str("Algorithm Performance Summary:\n");
        report.push_str("-".repeat(80).as_str());
        report.push('\n');
        report.push_str(&format!("{:<25} {:>10} {:>12} {:>12} {:>12} {:>10}\n",
            "Algorithm", "Feasible", "Avg Cost", "Best Cost", "Avg Gap%", "Avg Time"));
        report.push_str("-".repeat(80).as_str());
        report.push('\n');
        
        for stat in &stats {
            let gap_str = stat.avg_gap
                .map(|g| format!("{:.2}%", g))
                .unwrap_or_else(|| "-".to_string());
            
            report.push_str(&format!("{:<25} {:>10} {:>12.2} {:>12.2} {:>12} {:>10.4}\n",
                stat.algorithm,
                format!("{}/{}", stat.num_feasible, stat.num_instances),
                stat.avg_cost,
                stat.best_cost,
                gap_str,
                stat.avg_time));
        }
        
        report.push_str("-".repeat(80).as_str());
        report.push('\n');
        
        
        report.push_str("\nBest Solutions per Instance:\n");
        
        let mut instance_best: HashMap<String, (&AlgorithmResult, f64)> = HashMap::new();
        
        for result in &self.results {
            if !result.feasible {
                continue;
            }
            
            let entry = instance_best.entry(result.instance.clone())
                .or_insert((result, result.cost));
            
            if result.cost < entry.1 {
                *entry = (result, result.cost);
            }
        }
        
        for (instance, (best_result, _)) in &instance_best {
            report.push_str(&format!("  {}: {:.2} ({})\n",
                instance, best_result.cost, best_result.algorithm));
        }
        
        report
    }
    
    /// Get all results
    pub fn results(&self) -> &[AlgorithmResult] {
        &self.results
    }
    
    /// Get best known values
    pub fn best_known(&self) -> &HashMap<String, f64> {
        &self.best_known
    }
}

/// Helper function to load instances from a directory
pub fn load_instances_from_dir<P: AsRef<Path>>(dir: P) -> Vec<PDTSPInstance> {
    let mut instances = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "tsp").unwrap_or(false) {
                if let Ok(instance) = PDTSPInstance::from_file(&path) {
                    instances.push(instance);
                }
            }
        }
    }
    
    // Sort by dimension
    instances.sort_by_key(|i| i.dimension);
    
    instances
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_benchmark_config() {
        let config = BenchmarkConfig::default();
        assert_eq!(config.num_runs, 5);
    }
}
