//! PD-TSP Solver Library
//! 
//! A comprehensive solver for the Pickup and Delivery Traveling Salesman Problem (PD-TSP).
//! 
//! # Features
//! 
//! - Multiple construction heuristics (Nearest Neighbor, Greedy Insertion, Savings, etc.)
//! - Local search methods (2-opt, Or-opt, Swap, VND)
//! - Metaheuristics (Simulated Annealing, Tabu Search, ILS)
//! - Population-based methods (Genetic Algorithm, Ant Colony Optimization)
//! - Exact solver using Gurobi MIP
//! - Benchmarking and visualization tools
//! 
//! # Example
//! 
//! ```no_run
//! use pd_tsp_solver::instance::PDTSPInstance;
//! use pd_tsp_solver::heuristics::construction::MultiStartConstruction;
//! use pd_tsp_solver::heuristics::local_search::VND;
//! use pd_tsp_solver::heuristics::construction::ConstructionHeuristic;
//! use pd_tsp_solver::heuristics::local_search::LocalSearch;
//! 
//! // Load instance
//! let instance = PDTSPInstance::from_file("instance.tsp").unwrap();
//! 
//! // Construct initial solution
//! let multi_start = MultiStartConstruction::with_all_heuristics();
//! let mut solution = multi_start.construct(&instance);
//! 
//! // Improve with VND
//! let vnd = VND::with_standard_operators();
//! vnd.improve(&instance, &mut solution);
//! 
//! println!("Solution cost: {:.2}", solution.cost);
//! ```

pub mod instance;
pub mod solution;
pub mod heuristics;
pub mod exact;
pub mod benchmark;
pub mod visualization;

pub use instance::PDTSPInstance;
pub use solution::Solution;
