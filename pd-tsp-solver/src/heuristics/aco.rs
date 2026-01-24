//! Ant Colony Optimization for PD-TSP.
//! 
//! This module implements the Ant Colony System (ACS) algorithm
//! with capacity-aware path construction.

use crate::instance::PDTSPInstance;
use crate::solution::Solution;
use crate::heuristics::local_search::{LocalSearch, VND};
// (no construction fallback used any more)
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use ordered_float::OrderedFloat;

/// ACO configuration parameters
#[derive(Debug, Clone)]
pub struct ACOConfig {
    /// Number of ants
    pub num_ants: usize,
    /// Number of iterations
    pub max_iterations: usize,
    /// Maximum iterations without improvement
    pub max_no_improve: usize,
    /// Pheromone importance (alpha)
    pub alpha: f64,
    /// Heuristic importance (beta)
    pub beta: f64,
    /// Evaporation rate (rho)
    pub evaporation_rate: f64,
    /// Initial pheromone level
    pub initial_pheromone: f64,
    /// Pheromone deposit factor
    pub q: f64,
    /// Exploitation probability (q0 in ACS)
    pub q0: f64,
    /// Local pheromone decay
    pub local_decay: f64,
    /// Use local search
    pub use_local_search: bool,
    /// Random seed
    pub seed: u64,
    /// Time limit in seconds for the ACO run
    pub time_limit: f64,
}

impl Default for ACOConfig {
    fn default() -> Self {
        ACOConfig {
            num_ants: 20,
            max_iterations: 200,
            max_no_improve: 50,
            alpha: 1.0,
            beta: 2.5,
            evaporation_rate: 0.1,
            initial_pheromone: 1.0,
            q: 100.0,
            q0: 0.9,
            local_decay: 0.1,
            use_local_search: true,
            seed: 42,
            time_limit: 60.0,
        }
    }
}

/// Ant Colony Optimization solver
pub struct AntColonyOptimization {
    config: ACOConfig,
    instance: PDTSPInstance,
    pheromone: Vec<Vec<f64>>,
    heuristic: Vec<Vec<f64>>,
    best_tour: Vec<usize>,
    best_cost: f64,
    rng: ChaCha8Rng,
}

impl AntColonyOptimization {
    pub fn new(instance: PDTSPInstance, config: ACOConfig) -> Self {
        let n = instance.dimension;
        
        // Initialize pheromone matrix
        let pheromone = vec![vec![config.initial_pheromone; n]; n];
        
        // Initialize heuristic information (inverse distance)
        let mut heuristic = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..n {
                if i != j {
                    let dist = instance.distance(i, j);
                    heuristic[i][j] = if dist > 0.0 { 1.0 / dist } else { 1e6 };
                }
            }
        }
        
        let rng = ChaCha8Rng::seed_from_u64(config.seed);
        
        AntColonyOptimization {
            config,
            instance,
            pheromone,
            heuristic,
            best_tour: Vec::new(),
            best_cost: f64::INFINITY,
            rng,
        }
    }
    
    /// Construct a solution for one ant
    fn construct_solution(&mut self) -> Vec<usize> {
        let n = self.instance.dimension;
        let mut tour = vec![0]; // Start at depot
        let mut visited = vec![false; n];
        visited[0] = true;
        
        let mut current = 0;
        // Vehicle starts with initial load (depot demands processed)
        let mut current_load = self.instance.starting_load();
        
        while tour.len() < n {
            if let Some(next) = self.select_next_node(current, &visited, current_load) {
                tour.push(next);
                visited[next] = true;
                current_load += self.instance.nodes[next].demand;
                current = next;
            } else {
                // No feasible node found - terminate construction early
                break;
            }
        }
        
        tour
    }
    
    /// Select next node using ACS rule
    /// Returns None if no feasible unvisited node exists
    fn select_next_node(&mut self, current: usize, visited: &[bool], current_load: i32) -> Option<usize> {
        let n = self.instance.dimension;
        
        // Calculate probabilities for feasible unvisited nodes
        let mut candidates: Vec<(usize, f64)> = Vec::new();
        
        for j in 0..n {
            if visited[j] {
                continue;
            }
            
            // Check capacity feasibility
            let new_load = current_load + self.instance.nodes[j].demand;
            if new_load < 0 || new_load > self.instance.capacity {
                continue;
            }
            
            let tau = self.pheromone[current][j].powf(self.config.alpha);
            let eta = self.heuristic[current][j].powf(self.config.beta);
            candidates.push((j, tau * eta));
        }
        
        if candidates.is_empty() {
            // No feasible node available
            return None;
        }
        
        // ACS decision rule
        if self.rng.gen::<f64>() < self.config.q0 {
            // Exploitation: choose best
            candidates.iter()
                .max_by_key(|&&(_, prob)| OrderedFloat(prob))
                .map(|&(j, _)| j)
        } else {
            // Exploration: roulette wheel
            let total: f64 = candidates.iter().map(|&(_, p)| p).sum();
            let mut pick = self.rng.gen::<f64>() * total;
            
            for &(j, prob) in &candidates {
                pick -= prob;
                if pick <= 0.0 {
                    return Some(j);
                }
            }
            
            candidates.last().map(|&(j, _)| j)
        }
    }
    
    /// Local pheromone update (ACS)
    fn local_pheromone_update(&mut self, tour: &[usize]) {
        let n = tour.len();
        let tau0 = self.config.initial_pheromone;
        
        for i in 0..n {
            let from = tour[i];
            let to = tour[(i + 1) % n];
            
            self.pheromone[from][to] = 
                (1.0 - self.config.local_decay) * self.pheromone[from][to] 
                + self.config.local_decay * tau0;
            self.pheromone[to][from] = self.pheromone[from][to];
        }
    }
    
    /// Global pheromone update
    fn global_pheromone_update(&mut self) {
        let n = self.instance.dimension;
        
        // Evaporation
        for i in 0..n {
            for j in 0..n {
                self.pheromone[i][j] *= 1.0 - self.config.evaporation_rate;
            }
        }
        
        // Deposit by best ant
        if !self.best_tour.is_empty() {
            let delta = self.config.q / self.best_cost;
            
            let m = self.best_tour.len();
            for i in 0..m {
                let from = self.best_tour[i];
                let to = self.best_tour[(i + 1) % m];
                
                self.pheromone[from][to] += delta;
                self.pheromone[to][from] += delta;
            }
        }
    }
    
    /// Run ACO algorithm
    pub fn run(&mut self) -> Solution {
        let start = std::time::Instant::now();
        let vnd = VND::with_standard_operators();
        
        let mut no_improve = 0;
        let mut iteration = 0;
        
        while iteration < self.config.max_iterations && no_improve < self.config.max_no_improve
            && start.elapsed().as_secs_f64() < self.config.time_limit {
            let mut iteration_best_tour = Vec::new();
            let mut iteration_best_cost = f64::INFINITY;
            
            // Each ant constructs a solution
            for _ in 0..self.config.num_ants {
                let tour = self.construct_solution();
                
                if !self.instance.is_feasible(&tour) {
                    continue;
                }
                
                let mut cost = self.instance.tour_length(&tour);
                let mut final_tour = tour.clone();
                
                // Apply local search
                if self.config.use_local_search {
                    let mut solution = Solution::from_tour(&self.instance, tour, "ACO-temp");
                    vnd.improve(&self.instance, &mut solution);
                    
                    if solution.feasible {
                        final_tour = solution.tour;
                        cost = solution.cost;
                    }
                }
                
                // Local pheromone update
                self.local_pheromone_update(&final_tour);
                
                // Track iteration best
                if cost < iteration_best_cost {
                    iteration_best_cost = cost;
                    iteration_best_tour = final_tour;
                }
            }
            
            // Update global best
            if iteration_best_cost < self.best_cost {
                self.best_cost = iteration_best_cost;
                self.best_tour = iteration_best_tour;
                no_improve = 0;
            } else {
                no_improve += 1;
            }
            
            // Global pheromone update
            self.global_pheromone_update();
            
            iteration += 1;
        }
        
        // If no feasible solution found, return an empty/infeasible solution (no fallback)
        if self.best_tour.is_empty() {
            let mut solution = Solution::new();
            solution.algorithm = "ACO".to_string();
            solution.computation_time = start.elapsed().as_secs_f64();
            solution.iterations = Some(iteration);
            return solution;
        }
        
        let mut solution = Solution::from_tour(&self.instance, self.best_tour.clone(), "ACO");
        solution.computation_time = start.elapsed().as_secs_f64();
        solution.iterations = Some(iteration);
        
        solution
    }
    
    /// Get best solution found
    pub fn best_solution(&self) -> Solution {
        Solution::from_tour(&self.instance, self.best_tour.clone(), "ACO")
    }
}

/// Max-Min Ant System variant
pub struct MaxMinAntSystem {
    aco: AntColonyOptimization,
    tau_max: f64,
    tau_min: f64,
}

impl MaxMinAntSystem {
    pub fn new(instance: PDTSPInstance, config: ACOConfig) -> Self {
        let tau_max = 1.0 / (config.evaporation_rate * 1000.0); // Initial estimate
        let tau_min = tau_max / 50.0;
        
        let mut aco = AntColonyOptimization::new(instance, config);
        
        // Initialize pheromone to tau_max
        let n = aco.instance.dimension;
        for i in 0..n {
            for j in 0..n {
                aco.pheromone[i][j] = tau_max;
            }
        }
        
        MaxMinAntSystem {
            aco,
            tau_max,
            tau_min,
        }
    }
    
    /// Run MMAS algorithm
    pub fn run(&mut self) -> Solution {
        let start = std::time::Instant::now();
        let vnd = VND::with_standard_operators();
        
        let mut no_improve = 0;
        let mut iteration = 0;
        
        while iteration < self.aco.config.max_iterations && no_improve < self.aco.config.max_no_improve
            && start.elapsed().as_secs_f64() < self.aco.config.time_limit {
            let mut iteration_best_tour = Vec::new();
            let mut iteration_best_cost = f64::INFINITY;
            
            for _ in 0..self.aco.config.num_ants {
                let tour = self.aco.construct_solution();
                
                if !self.aco.instance.is_feasible(&tour) {
                    continue;
                }
                
                let mut cost = self.aco.instance.tour_length(&tour);
                let mut final_tour = tour.clone();
                
                if self.aco.config.use_local_search {
                    let mut solution = Solution::from_tour(&self.aco.instance, tour, "MMAS-temp");
                    vnd.improve(&self.aco.instance, &mut solution);
                    
                    if solution.feasible {
                        final_tour = solution.tour;
                        cost = solution.cost;
                    }
                }
                
                if cost < iteration_best_cost {
                    iteration_best_cost = cost;
                    iteration_best_tour = final_tour;
                }
            }
            
            // Update best
            if iteration_best_cost < self.aco.best_cost {
                self.aco.best_cost = iteration_best_cost;
                self.aco.best_tour = iteration_best_tour.clone();
                no_improve = 0;
                
                // Update tau bounds
                self.tau_max = 1.0 / (self.aco.config.evaporation_rate * self.aco.best_cost);
                self.tau_min = self.tau_max / 50.0;
            } else {
                no_improve += 1;
            }
            
            // Pheromone update with bounds
            let n = self.aco.instance.dimension;
            
            // Evaporation
            for i in 0..n {
                for j in 0..n {
                    self.aco.pheromone[i][j] *= 1.0 - self.aco.config.evaporation_rate;
                }
            }
            
            // Deposit by best (iteration best or global best)
            let update_tour = if no_improve > 10 {
                &self.aco.best_tour
            } else {
                &iteration_best_tour
            };
            
            if !update_tour.is_empty() {
                let cost = self.aco.instance.tour_length(update_tour);
                let delta = self.aco.config.q / cost;
                
                let m = update_tour.len();
                for i in 0..m {
                    let from = update_tour[i];
                    let to = update_tour[(i + 1) % m];
                    
                    self.aco.pheromone[from][to] += delta;
                    self.aco.pheromone[to][from] += delta;
                }
            }
            
            // Apply bounds
            for i in 0..n {
                for j in 0..n {
                    self.aco.pheromone[i][j] = self.aco.pheromone[i][j]
                        .max(self.tau_min)
                        .min(self.tau_max);
                }
            }
            
            iteration += 1;
        }
        
        // If no feasible solution found, return an empty/infeasible solution (no fallback)
        if self.aco.best_tour.is_empty() {
            let mut solution = Solution::new();
            solution.algorithm = "MMAS".to_string();
            solution.computation_time = start.elapsed().as_secs_f64();
            solution.iterations = Some(iteration);
            return solution;
        }
        
        let mut solution = Solution::from_tour(&self.aco.instance, self.aco.best_tour.clone(), "MMAS");
        solution.computation_time = start.elapsed().as_secs_f64();
        solution.iterations = Some(iteration);
        
        solution
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::Node;
    
    fn create_test_instance() -> PDTSPInstance {
        use crate::instance::CostFunction;
        
        let nodes = vec![
            Node::new(0, 0.0, 0.0, 0, 0),
            Node::new(1, 1.0, 0.0, 5, 0),
            Node::new(2, 2.0, 0.0, -3, 0),
            Node::new(3, 1.0, 1.0, -2, 0),
        ];
        
        let mut instance = PDTSPInstance {
            cost_function: CostFunction::Distance,
            alpha: 0.1,
            beta: 0.5,
            name: "test".to_string(),
            comment: "test".to_string(),
            dimension: 4,
            capacity: 10,
            nodes: nodes.clone(),
            distance_matrix: Vec::new(),
            return_depot_demand: 0,
        };
        
        instance.distance_matrix = vec![vec![0.0; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                let dx = instance.nodes[i].x - instance.nodes[j].x;
                let dy = instance.nodes[i].y - instance.nodes[j].y;
                instance.distance_matrix[i][j] = (dx * dx + dy * dy).sqrt();
            }
        }
        
        instance
    }
    
    #[test]
    fn test_aco() {
        let instance = create_test_instance();
        let config = ACOConfig {
            num_ants: 5,
            max_iterations: 10,
            ..Default::default()
        };
        
        let mut aco = AntColonyOptimization::new(instance, config);
        let solution = aco.run();
        
        assert!(solution.feasible);
    }
}
