//! PD-TSP Solver - Command Line Interface
//! 
//! A comprehensive solver for the Pickup and Delivery Traveling Salesman Problem.

use clap::{Parser, Subcommand, ValueEnum};
use pd_tsp_solver::instance::PDTSPInstance;
use pd_tsp_solver::solution::Solution;
use pd_tsp_solver::heuristics::construction::*;
use pd_tsp_solver::heuristics::local_search::*;
use pd_tsp_solver::heuristics::genetic::{GeneticAlgorithm, GAConfig, MemeticAlgorithm};
use pd_tsp_solver::heuristics::aco::{AntColonyOptimization, ACOConfig, MaxMinAntSystem};
use pd_tsp_solver::heuristics::profit_density::ProfitDensityHeuristic;
use pd_tsp_solver::exact::{GurobiSolver, GurobiConfig};
use pd_tsp_solver::benchmark::{Benchmark, BenchmarkConfig, load_instances_from_dir};
use pd_tsp_solver::visualization::Visualizer;

use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "pd-tsp-solver")]
#[command(author = "M2 AI2D Student")]
#[command(version = "1.0")]
#[command(about = "A comprehensive solver for the Pickup and Delivery TSP")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Solve {
        #[arg(short, long)]
        instance: PathBuf,
        
        /// Algorithm to use
        #[arg(short, long, value_enum, default_value = "hybrid")]
        algorithm: Algorithm,
        
        /// Cost function: distance, quadratic, or linear-load
        #[arg(long, value_enum, default_value = "distance")]
        cost_function: CostFunction,
        
        /// Alpha parameter: linear weight applied to absolute load (used by linear-load
        /// and as the linear term in quadratic cost)
        #[arg(long, default_value = "0.1")]
        alpha: f64,

        /// Beta parameter: quadratic weight applied to load^2 (used by quadratic cost)
        #[arg(long, default_value = "0.0")]
        beta: f64,
        
        /// Time limit in seconds
        #[arg(short, long, default_value = "60")]
        time_limit: f64,
        
        /// Random seed
        #[arg(short, long, default_value = "42")]
        seed: u64,
        
        /// Output solution to file
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Generate SVG visualization
        #[arg(long)]
        visualize: bool,
        
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
        /// Maximum random profit to assign (10..=max). 0 means keep existing profits.
        #[arg(long, default_value = "200")]
        max_profit: i32,
    },
    
    /// Run benchmarks on a directory of instances
    Benchmark {
        /// Directory containing instance files
        #[arg(short, long)]
        dir: PathBuf,
        
        /// Output directory for results
        #[arg(short, long, default_value = "results")]
        output: PathBuf,
        
        /// Number of runs per algorithm
        #[arg(short, long, default_value = "5")]
        runs: usize,
        
        /// Time limit per run
        #[arg(short, long, default_value = "60")]
        time_limit: f64,
        
        /// Run exact solver (requires Gurobi)
        #[arg(long)]
        exact: bool,
        
        /// Exact solver time limit
        #[arg(long, default_value = "300")]
        exact_time_limit: f64,
        
        /// Maximum instance size
        #[arg(long)]
        max_size: Option<usize>,
    },
    
    /// Analyze an instance
    Analyze {
        /// Path to the instance file
        #[arg(short, long)]
        instance: PathBuf,
    },
    
    /// Compare algorithms on an instance
    Compare {
        /// Path to the instance file
        #[arg(short, long)]
        instance: PathBuf,
        
        /// Number of runs
        #[arg(short, long, default_value = "10")]
        runs: usize,
        
        /// Output CSV file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum, Debug)]
enum Algorithm {
    /// Nearest Neighbor construction
    Nn,
    /// Greedy Insertion
    Greedy,
    /// Savings algorithm (Clarke-Wright)
    Savings,
    /// Sweep algorithm
    Sweep,
    /// Regret Insertion
    Regret,
    /// Cluster-First algorithm
    ClusterFirst,
    /// Multi-start construction
    MultiStart,
    /// 2-Opt local search
    TwoOpt,
    /// Variable Neighborhood Descent
    Vnd,
    /// Simulated Annealing
    Sa,
    /// Tabu Search
    Tabu,
    /// Iterated Local Search
    Ils,
    /// Genetic Algorithm
    Ga,
    /// Memetic Algorithm
    Memetic,
    /// Ant Colony Optimization
    Aco,
    /// Max-Min Ant System
    Mmas,
    /// Hybrid (best combination)
    Hybrid,
    /// Profit-density construction heuristic
    ProfitDensity,
    /// Exact solver (Gurobi)
    Exact,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum, Debug)]
enum CostFunction {
    /// Euclidean distance only
    Distance,
    /// Quadratic load-dependent: distance + alpha * W + beta * W^2 (additive surcharge)
    Quadratic,
    /// Linear load-dependent: distance + alpha * |W| (additive surcharge)
    LinearLoad,
}

fn main() {
    env_logger::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Solve { instance, algorithm, cost_function, alpha, beta, time_limit, seed, output, visualize, verbose, max_profit } => {
            solve_instance(&instance, algorithm, cost_function, alpha, beta, time_limit, seed, output, visualize, verbose, max_profit);
        }
        
        Commands::Benchmark { dir, output, runs, time_limit, exact, exact_time_limit, max_size } => {
            run_benchmark(&dir, &output, runs, time_limit, exact, exact_time_limit, max_size);
        }
        
        Commands::Analyze { instance } => {
            analyze_instance(&instance);
        }
        
        Commands::Compare { instance, runs, output } => {
            compare_algorithms(&instance, runs, output);
        }
    }
}

fn solve_instance(
    path: &PathBuf,
    algorithm: Algorithm,
    cost_function: CostFunction,
    alpha: f64,
    beta: f64,
    time_limit: f64,
    seed: u64,
    output: Option<PathBuf>,
    visualize: bool,
    verbose: bool,
    max_profit: i32,
) {
    println!("Loading instance from {:?}...", path);
    
    let mut instance = match PDTSPInstance::from_file(path) {
        Ok(inst) => inst,
        Err(e) => {
            eprintln!("Error loading instance: {}", e);
            std::process::exit(1);
        }
    };

    
    if max_profit > 0 {
        instance.assign_random_profits(seed, max_profit);
    }
    
    if verbose {
        println!("{}", instance.statistics());
        println!("Cost function: {:?}", cost_function);
        match cost_function {
            CostFunction::Quadratic => println!("Alpha (linear weight): {}, Beta (quadratic weight): {}", alpha, beta),
            CostFunction::LinearLoad => println!("Alpha (linear load weight): {}", alpha),
            _ => {}
        }
    }
    
    
    instance.cost_function = match cost_function {
        CostFunction::Distance => pd_tsp_solver::instance::CostFunction::Distance,
        CostFunction::Quadratic => pd_tsp_solver::instance::CostFunction::Quadratic,
        CostFunction::LinearLoad => pd_tsp_solver::instance::CostFunction::LinearLoad,
    };
    instance.alpha = alpha;
    instance.beta = beta;

    println!("Solving with {:?} algorithm...", algorithm);
    let start = Instant::now();
    
    let solution = match algorithm {
        Algorithm::Nn => {
            let nn = NearestNeighborHeuristic::new();
            nn.construct(&instance)
        }
        
        Algorithm::Greedy => {
            let greedy = GreedyInsertionHeuristic::new();
            greedy.construct(&instance)
        }
        
        Algorithm::Savings => {
            let savings = SavingsHeuristic::new();
            savings.construct(&instance)
        }
        
        Algorithm::Sweep => {
            let sweep = SweepHeuristic::new();
            sweep.construct(&instance)
        }
        
        Algorithm::Regret => {
            let regret = RegretInsertionHeuristic::new(3);
            regret.construct(&instance)
        }
        
        Algorithm::ClusterFirst => {
            let cluster = ClusterFirstHeuristic::new();
            cluster.construct(&instance)
        }
        
        Algorithm::MultiStart => {
            let multi = MultiStartConstruction::with_all_heuristics();
            multi.construct(&instance)
        }
        
        Algorithm::ProfitDensity => {
            let pd = ProfitDensityHeuristic::new();
            pd.construct(&instance)
        }
        
        Algorithm::TwoOpt => {
            let multi = MultiStartConstruction::with_all_heuristics();
            let mut sol = multi.construct(&instance);
            let two_opt = TwoOptSearch::new();
            two_opt.improve(&instance, &mut sol);
            sol
        }
        
        Algorithm::Vnd => {
            let multi = MultiStartConstruction::with_all_heuristics();
            let mut sol = multi.construct(&instance);
            let vnd = VND::with_standard_operators();
            vnd.improve(&instance, &mut sol);
            sol.algorithm = "VND".to_string();
            sol
        }
        
        Algorithm::Sa => {
            let multi = MultiStartConstruction::with_all_heuristics();
            let mut sol = multi.construct(&instance);
            let mut sa = SimulatedAnnealing::new();
            sa.seed = seed;
            sa.improve(&instance, &mut sol);
            sol.algorithm = "SimulatedAnnealing".to_string();
            sol
        }
        
        Algorithm::Tabu => {
            let multi = MultiStartConstruction::with_all_heuristics();
            let mut sol = multi.construct(&instance);
            let ts = TabuSearch::new();
            ts.improve(&instance, &mut sol);
            sol.algorithm = "TabuSearch".to_string();
            sol
        }
        
        Algorithm::Ils => {
            let multi = MultiStartConstruction::with_all_heuristics();
            let mut sol = multi.construct(&instance);
            let mut ils = IteratedLocalSearch::new();
            ils.seed = seed;
            ils.improve(&instance, &mut sol);
            sol.algorithm = "ILS".to_string();
            sol
        }
        
        Algorithm::Ga => {
            let config = GAConfig {
                seed,
                population_size: 50,
                max_generations: 200,
                time_limit: time_limit,
                ..Default::default()
            };
            let mut ga = GeneticAlgorithm::new(instance.clone(), config);
            ga.run()
        }
        
        Algorithm::Memetic => {
            let config = GAConfig {
                seed,
                time_limit: time_limit,
                ..Default::default()
            };
            let mut ma = MemeticAlgorithm::with_config(instance.clone(), config);
            ma.run()
        }
        
        Algorithm::Aco => {
            let config = ACOConfig {
                seed,
                max_iterations: 200,
                ..Default::default()
            };
            let mut aco = AntColonyOptimization::new(instance.clone(), config);
            aco.run()
        }
        
        Algorithm::Mmas => {
            let config = ACOConfig {
                seed,
                max_iterations: 200,
                ..Default::default()
            };
            let mut mmas = MaxMinAntSystem::new(instance.clone(), config);
            mmas.run()
        }
        
        Algorithm::Hybrid => {
            
            let multi = MultiStartConstruction::with_all_heuristics();
            let mut sol = multi.construct(&instance);
            
            
            let vnd = VND::with_standard_operators();
            vnd.improve(&instance, &mut sol);
            
            
            let mut ils = IteratedLocalSearch::with_params(4, 50, 15);
            ils.seed = seed;
            ils.improve(&instance, &mut sol);
            
            sol.algorithm = "Hybrid".to_string();
            sol
        }
        
        Algorithm::Exact => {
            let warm_start = {
                let multi = MultiStartConstruction::with_all_heuristics();
                let mut sol = multi.construct(&instance);
                let vnd = VND::with_standard_operators();
                vnd.improve(&instance, &mut sol);
                sol.tour
            };
            
            let config = GurobiConfig {
                time_limit,
                verbose,
                warm_start: Some(warm_start),
                ..Default::default()
            };
            
            let solver = GurobiSolver::new(config);
            match solver.solve(&instance) {
                Ok(result) => {
                    println!("Status: {}", result.status);
                    println!("Lower bound: {:.2}", result.lower_bound);
                    println!("Gap: {:.4}%", result.gap * 100.0);
                    println!("Nodes explored: {}", result.nodes_explored);
                    result.solution
                }
                Err(e) => {
                    eprintln!("Gurobi solver error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    };
    
    let elapsed = start.elapsed();
    
    
    let final_solution = solution;
    
    
    println!("\n========== Results ==========");
    println!("Algorithm: {}", final_solution.algorithm);
    println!("Cost function: {:?}", cost_function);
    println!("Cost (travel): {:.2}", final_solution.cost);
    println!("Total profit: {}", final_solution.total_profit);
    println!("Objective (profit - travel_cost): {:.2}", final_solution.objective);
    println!("Feasible: {}", final_solution.feasible);
    println!("Time: {:.4}s", elapsed.as_secs_f64());
    if let Some(iter) = final_solution.iterations {
        println!("Iterations: {}", iter);
    }
    
    if verbose {
        println!("\nTour: {:?}", final_solution.tour);
        let profile = final_solution.load_profile(&instance);
        println!("Load profile: {:?}", profile);
        println!("Max load: {}", final_solution.max_load(&instance));
        println!("Min load: {}", final_solution.min_load(&instance));
    }
    
    
    if let Some(out_path) = output {
        let json = serde_json::to_string_pretty(&final_solution).unwrap();
        std::fs::write(&out_path, json).expect("Failed to write output");
        println!("\nSolution saved to {:?}", out_path);
    }
    
    
    if visualize {
        let viz = Visualizer::new();
        let svg = viz.generate_svg(&instance, &final_solution);
        let png_path = path.with_extension("png");
        match viz.save_png(&svg, &png_path) {
            Ok(()) => println!("Visualization saved to {:?}", png_path),
            Err(e) => {
                // fallback: write SVG if PNG conversion failed
                let svg_path = path.with_extension("svg");
                viz.save_svg(&svg, &svg_path).expect("Failed to save SVG");
                println!("PNG conversion failed ({}). Saved SVG to {:?}", e, svg_path);
            }
        }

        let load_svg = viz.generate_load_profile_svg(&instance, &final_solution);
        let load_png_path = path.with_extension("load.png");
        match viz.save_png(&load_svg, &load_png_path) {
            Ok(()) => println!("Load profile saved to {:?}", load_png_path),
            Err(e) => {
                let load_svg_path = path.with_extension("load.svg");
                viz.save_svg(&load_svg, &load_svg_path).expect("Failed to save load SVG");
                println!("PNG conversion failed ({}). Saved load SVG to {:?}", e, load_svg_path);
            }
        }
    }
}

fn run_benchmark(
    dir: &PathBuf,
    output: &PathBuf,
    runs: usize,
    time_limit: f64,
    exact: bool,
    exact_time_limit: f64,
    max_size: Option<usize>,
) {
    println!("Loading instances from {:?}...", dir);
    
    let mut instances = load_instances_from_dir(dir);
    
    if let Some(max) = max_size {
        instances.retain(|i| i.dimension <= max);
    }
    
    println!("Found {} instances", instances.len());
    
    if instances.is_empty() {
        eprintln!("No instances found!");
        return;
    }
    
    
    std::fs::create_dir_all(output).expect("Failed to create output directory");
    
    let config = BenchmarkConfig {
        num_runs: runs,
        time_limit,
        run_exact: exact,
        exact_time_limit,
        output_dir: output.to_string_lossy().to_string(),
        ..Default::default()
    };
    
    let mut benchmark = Benchmark::new(config);
    
    for (i, instance) in instances.iter().enumerate() {
        println!("\n[{}/{}] Processing {} (n={})...", 
            i + 1, instances.len(), instance.name, instance.dimension);
        
        benchmark.run_full_benchmark(instance);
    }
    
    
    let results_path = output.join("results.csv");
    benchmark.export_to_csv(&results_path).expect("Failed to export results");
    println!("\nResults exported to {:?}", results_path);
    
    let stats_path = output.join("statistics.csv");
    benchmark.export_statistics_csv(&stats_path).expect("Failed to export statistics");
    println!("Statistics exported to {:?}", stats_path);
    
    
    let report = benchmark.generate_report();
    println!("\n{}", report);
    
    let report_path = output.join("report.txt");
    std::fs::write(&report_path, &report).expect("Failed to save report");
    println!("Report saved to {:?}", report_path);
}

fn analyze_instance(path: &PathBuf) {
    let instance = match PDTSPInstance::from_file(path) {
        Ok(inst) => inst,
        Err(e) => {
            eprintln!("Error loading instance: {}", e);
            std::process::exit(1);
        }
    };
    
    println!("========== Instance Analysis ==========\n");
    println!("{}", instance.statistics());
    
    
    let pickups: Vec<_> = instance.nodes.iter().filter(|n| n.demand < 0).collect();
    let deliveries: Vec<_> = instance.nodes.iter().filter(|n| n.demand > 0).collect();
    let neutrals: Vec<_> = instance.nodes.iter().filter(|n| n.demand == 0 && n.id != 0).collect();
    
    println!("\nDemand Distribution:");
    println!("  Pickup nodes: {} (total: {})", 
        pickups.len(),
        pickups.iter().map(|n| -n.demand).sum::<i32>());
    println!("  Delivery nodes: {} (total: {})",
        deliveries.len(),
        deliveries.iter().map(|n| n.demand).sum::<i32>());
    println!("  Neutral nodes: {}", neutrals.len());
    
    
    let all_demands: Vec<i32> = instance.nodes.iter()
        .filter(|n| n.demand != 0 && n.id != 0)
        .map(|n| n.demand.abs())
        .collect();
    
    if !all_demands.is_empty() {
        let avg_demand = all_demands.iter().sum::<i32>() as f64 / all_demands.len() as f64;
        let max_demand = *all_demands.iter().max().unwrap();
        let min_demand = *all_demands.iter().min().unwrap();
        
        println!("\nDemand Statistics (absolute):");
        println!("  Average: {:.2}", avg_demand);
        println!("  Min: {}", min_demand);
        println!("  Max: {}", max_demand);
        println!("  Capacity utilization ratio: {:.2}%", 
            avg_demand / instance.capacity as f64 * 100.0);
    }
    
    
    let mut all_distances: Vec<f64> = Vec::new();
    for i in 0..instance.dimension {
        for j in i + 1..instance.dimension {
            all_distances.push(instance.distance(i, j));
        }
    }
    
    let avg_dist = all_distances.iter().sum::<f64>() / all_distances.len() as f64;
    let min_dist = all_distances.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_dist = all_distances.iter().cloned().fold(0.0, f64::max);
    
    println!("\nDistance Statistics:");
    println!("  Average: {:.2}", avg_dist);
    println!("  Min: {:.2}", min_dist);
    println!("  Max: {:.2}", max_dist);
    
    
    let nn = NearestNeighborHeuristic::new();
    let nn_sol = nn.construct(&instance);
    
    let multi = MultiStartConstruction::with_all_heuristics();
    let mut multi_sol = multi.construct(&instance);
    let vnd = VND::with_standard_operators();
    vnd.improve(&instance, &mut multi_sol);
    
    println!("\nQuick Solution Estimates:");
    println!("  Nearest Neighbor: {:.2} (feasible: {})", nn_sol.cost, nn_sol.feasible);
    println!("  Multi-Start + VND: {:.2} (feasible: {})", multi_sol.cost, multi_sol.feasible);
}

fn compare_algorithms(path: &PathBuf, runs: usize, output: Option<PathBuf>) {
    let instance = match PDTSPInstance::from_file(path) {
        Ok(inst) => inst,
        Err(e) => {
            eprintln!("Error loading instance: {}", e);
            std::process::exit(1);
        }
    };
    
    println!("Comparing algorithms on {} (n={})...\n", instance.name, instance.dimension);
    
    let mut results: Vec<(String, Vec<f64>, Vec<f64>)> = Vec::new();
    
    
    let algorithms: Vec<(&str, Box<dyn Fn(&PDTSPInstance, u64) -> Solution>)> = vec![
        ("MultiStart+VND", Box::new(|inst: &PDTSPInstance, _seed: u64| {
            let multi = MultiStartConstruction::with_all_heuristics();
            let mut sol = multi.construct(inst);
            let vnd = VND::with_standard_operators();
            vnd.improve(inst, &mut sol);
            sol
        })),
        ("SA", Box::new(|inst: &PDTSPInstance, seed: u64| {
            let multi = MultiStartConstruction::with_all_heuristics();
            let mut sol = multi.construct(inst);
            let mut sa = SimulatedAnnealing::new();
            sa.seed = seed;
            sa.improve(inst, &mut sol);
            sol
        })),
        ("Tabu", Box::new(|inst: &PDTSPInstance, _seed: u64| {
            let multi = MultiStartConstruction::with_all_heuristics();
            let mut sol = multi.construct(inst);
            let ts = TabuSearch::new();
            ts.improve(inst, &mut sol);
            sol
        })),
        ("ILS", Box::new(|inst: &PDTSPInstance, seed: u64| {
            let multi = MultiStartConstruction::with_all_heuristics();
            let mut sol = multi.construct(inst);
            let mut ils = IteratedLocalSearch::new();
            ils.seed = seed;
            ils.improve(inst, &mut sol);
            sol
        })),
        ("GA", Box::new(|inst: &PDTSPInstance, seed: u64| {
            let config = GAConfig {
                seed,
                population_size: 50,
                max_generations: 100,
                time_limit: 60.0,
                ..Default::default()
            };
            let mut ga = GeneticAlgorithm::new(inst.clone(), config);
            ga.run()
        })),
        ("MA", Box::new(|inst: &PDTSPInstance, seed: u64| {
            let config = GAConfig {
                seed,
                population_size: 30,
                max_generations: 50,
                time_limit: 60.0,
                ..Default::default()
            };
            let mut ma = MemeticAlgorithm::with_config(inst.clone(), config);
            ma.run()
        })),
        ("ACO", Box::new(|inst: &PDTSPInstance, seed: u64| {
            let config = ACOConfig {
                seed,
                num_ants: 15,
                max_iterations: 50,
                ..Default::default()
            };
            let mut aco = AntColonyOptimization::new(inst.clone(), config);
            aco.run()
        })),
    ];
    
    for (name, solver) in &algorithms {
        let mut costs = Vec::new();
        let mut times = Vec::new();
        
        print!("Testing {}... ", name);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        for seed in 0..runs as u64 {
            let start = Instant::now();
            let sol = solver(&instance, seed);
            let elapsed = start.elapsed().as_secs_f64();
            
            if sol.feasible {
                costs.push(sol.cost);
                times.push(elapsed);
            }
        }
        
        if !costs.is_empty() {
            let avg_cost = costs.iter().sum::<f64>() / costs.len() as f64;
            let avg_time = times.iter().sum::<f64>() / times.len() as f64;
            println!("avg={:.2}, best={:.2}, time={:.4}s", 
                avg_cost, 
                costs.iter().cloned().fold(f64::INFINITY, f64::min),
                avg_time);
        } else {
            println!("no feasible solutions");
        }
        
        results.push((name.to_string(), costs, times));
    }
    
    
    println!("\n========== Summary ==========");
    println!("{:<15} {:>10} {:>10} {:>10} {:>10}", 
        "Algorithm", "Best", "Average", "Worst", "Avg Time");
    println!("{}", "-".repeat(60));
    
    for (name, costs, times) in &results {
        if !costs.is_empty() {
            let best = costs.iter().cloned().fold(f64::INFINITY, f64::min);
            let avg = costs.iter().sum::<f64>() / costs.len() as f64;
            let worst = costs.iter().cloned().fold(0.0, f64::max);
            let avg_time = times.iter().sum::<f64>() / times.len() as f64;
            
            println!("{:<15} {:>10.2} {:>10.2} {:>10.2} {:>10.4}", 
                name, best, avg, worst, avg_time);
        }
    }
    
    
    if let Some(out_path) = output {
        let mut csv = String::new();
        csv.push_str("algorithm,run,cost,time\n");
        
        for (name, costs, times) in &results {
            for (i, (cost, time)) in costs.iter().zip(times.iter()).enumerate() {
                csv.push_str(&format!("{},{},{:.2},{:.4}\n", name, i, cost, time));
            }
        }
        
        std::fs::write(&out_path, csv).expect("Failed to write CSV");
        println!("\nResults exported to {:?}", out_path);
    }
}
