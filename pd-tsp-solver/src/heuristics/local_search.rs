//! Local search improvement heuristics for PD-TSP.
//! 
//! This module implements various local search algorithms:
//! - 2-opt with feasibility checks
//! - Or-opt (segment relocation)
//! - Node swap
//! - Node insertion/relocation
//! - Lin-Kernighan style moves

use crate::instance::PDTSPInstance;
use crate::solution::Solution;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

/// Trait for local search improvement methods
pub trait LocalSearch {
    fn improve(&self, instance: &PDTSPInstance, solution: &mut Solution) -> bool;
    fn name(&self) -> &str;
}

 

/// 2-Opt Local Search with capacity feasibility
/// 
/// Reverses segments of the tour to reduce total distance
/// while maintaining capacity constraints.
pub struct TwoOptSearch {
    /// Use first improvement instead of best improvement
    pub first_improvement: bool,
    /// Maximum iterations without improvement
    pub max_no_improve: usize,
}

impl TwoOptSearch {
    pub fn new() -> Self {
        TwoOptSearch {
            first_improvement: false,
            max_no_improve: 10,
        }
    }
    
    pub fn first_improvement() -> Self {
        TwoOptSearch {
            first_improvement: true,
            max_no_improve: 10,
        }
    }
    
    /// Check if 2-opt move maintains feasibility
    fn is_feasible_move(&self, instance: &PDTSPInstance, tour: &[usize], i: usize, j: usize) -> bool {
        
        let mut new_tour = tour.to_vec();
        new_tour[i + 1..=j].reverse();
        instance.is_feasible(&new_tour)
    }
}

impl Default for TwoOptSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalSearch for TwoOptSearch {
    fn improve(&self, instance: &PDTSPInstance, solution: &mut Solution) -> bool {
        let n = solution.tour.len();
        if n < 3 { return false; }
        
        let mut improved = true;
        let mut total_improved = false;
        let mut no_improve_count = 0;
        let mut total_iterations = 0;
        let max_total_iterations = 50; // Limit total iterations
        
        while improved && no_improve_count < self.max_no_improve && total_iterations < max_total_iterations {
            improved = false;
            let mut best_delta = 0.0;
            let mut best_i = 0;
            let mut best_j = 0;
            total_iterations += 1;
            
            for i in 0..n - 2 {
                for j in i + 2..n {
                    if i == 0 && j == n - 1 {
                        continue; // Skip if it would just reverse entire tour
                    }
                    
                    let delta = solution.two_opt_delta(instance, i, j);
                    
                    if delta < -1e-9 {
                        if self.is_feasible_move(instance, &solution.tour, i, j) {
                            if self.first_improvement {
                                solution.apply_two_opt(i, j);
                                solution.cost += delta;
                                improved = true;
                                total_improved = true;
                                no_improve_count = 0;
                                break;
                            } else if delta < best_delta {
                                best_delta = delta;
                                best_i = i;
                                best_j = j;
                            }
                        }
                    }
                }
                if improved && self.first_improvement {
                    break;
                }
            }
            
            if !self.first_improvement && best_delta < -1e-9 {
                solution.apply_two_opt(best_i, best_j);
                solution.cost += best_delta;
                improved = true;
                total_improved = true;
                no_improve_count = 0;
            } else if !improved {
                no_improve_count += 1;
            }
        }
        
        solution.validate(instance);
        total_improved
    }
    
    fn name(&self) -> &str {
        if self.first_improvement {
            "2-Opt-FI"
        } else {
            "2-Opt-BI"
        }
    }
}

 

/// Or-Opt Local Search
/// 
/// Relocates segments of 1, 2, or 3 consecutive nodes to other positions.
pub struct OrOptSearch {
    /// Maximum segment length to consider
    pub max_segment_length: usize,
    /// Use first improvement
    pub first_improvement: bool,
}

impl OrOptSearch {
    pub fn new() -> Self {
        OrOptSearch {
            max_segment_length: 3,
            first_improvement: false,
        }
    }
    
    pub fn first_improvement() -> Self {
        OrOptSearch {
            max_segment_length: 3,
            first_improvement: true,
        }
    }
    
    /// Calculate delta for relocating a segment
    fn segment_relocation_delta(
        &self,
        instance: &PDTSPInstance,
        tour: &[usize],
        seg_start: usize,
        seg_len: usize,
        insert_pos: usize
    ) -> f64 {
        let n = tour.len();
        let seg_end = seg_start + seg_len - 1;
        
        
        if insert_pos >= seg_start && insert_pos <= seg_end + 1 {
            return 0.0;
        }
        
        let prev_seg = if seg_start == 0 { n - 1 } else { seg_start - 1 };
        let next_seg = (seg_end + 1) % n;
        
        
        let removal_cost = -instance.distance(tour[prev_seg], tour[seg_start])
            - instance.distance(tour[seg_end], tour[next_seg])
            + instance.distance(tour[prev_seg], tour[next_seg]);
        
        
        let prev_insert = if insert_pos == 0 { n - 1 } else { insert_pos - 1 };
        
        
        let actual_prev = if prev_insert >= seg_start && prev_insert <= seg_end {
            prev_seg
        } else if prev_insert > seg_end {
            tour[(prev_insert - seg_len + n) % n]
        } else {
            tour[prev_insert]
        };
        
        let actual_next = if insert_pos >= seg_start && insert_pos <= seg_end {
            tour[next_seg]
        } else if insert_pos > seg_end {
            tour[(insert_pos - seg_len + n) % n]
        } else {
            tour[insert_pos % n]
        };
        
        let insertion_cost = instance.distance(actual_prev, tour[seg_start])
            + instance.distance(tour[seg_end], actual_next)
            - instance.distance(actual_prev, actual_next);
        
        removal_cost + insertion_cost
    }
    
    /// Check if segment relocation maintains feasibility
    fn is_feasible_relocation(
        &self,
        instance: &PDTSPInstance,
        tour: &[usize],
        seg_start: usize,
        seg_len: usize,
        insert_pos: usize
    ) -> bool {
        let mut new_tour = Vec::with_capacity(tour.len());
        
        
        let segment: Vec<usize> = tour[seg_start..seg_start + seg_len].to_vec();
        
        for (i, &node) in tour.iter().enumerate() {
            if i == insert_pos && insert_pos < seg_start {
                new_tour.extend(&segment);
            }
            if i < seg_start || i >= seg_start + seg_len {
                new_tour.push(node);
            }
            if i == insert_pos && insert_pos > seg_start + seg_len {
                new_tour.extend(&segment);
            }
        }
        
        if insert_pos == tour.len() {
            new_tour.extend(&segment);
        }
        
        instance.is_feasible(&new_tour)
    }
    
    /// Apply segment relocation
    fn apply_relocation(&self, tour: &mut Vec<usize>, seg_start: usize, seg_len: usize, insert_pos: usize) {
        let segment: Vec<usize> = tour.drain(seg_start..seg_start + seg_len).collect();
        let adj_pos = if insert_pos > seg_start { insert_pos - seg_len } else { insert_pos };
        
        for (i, node) in segment.into_iter().enumerate() {
            tour.insert(adj_pos + i, node);
        }
    }
}

impl Default for OrOptSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalSearch for OrOptSearch {
    fn improve(&self, instance: &PDTSPInstance, solution: &mut Solution) -> bool {
        let n = solution.tour.len();
        if n < 3 { return false; }
        
        let mut improved = true;
        let mut total_improved = false;
        let mut iterations = 0;
        let max_iterations = 20;
        
        while improved && iterations < max_iterations {
            improved = false;
            let mut best_delta = 0.0;
            let mut best_seg_start = 0;
            let mut best_seg_len = 1;
            let mut best_insert_pos = 0;
            iterations += 1;
            
            for seg_len in 1..=self.max_segment_length.min(n - 1) {
                for seg_start in 0..n - seg_len + 1 {
                    
                    if solution.tour[seg_start] == 0 {
                        continue;
                    }
                    
                    for insert_pos in 0..=n - seg_len {
                        if insert_pos >= seg_start && insert_pos <= seg_start + seg_len {
                            continue;
                        }
                        
                        let delta = self.segment_relocation_delta(
                            instance, &solution.tour, seg_start, seg_len, insert_pos
                        );
                        
                        if delta < -1e-9 {
                            if self.is_feasible_relocation(instance, &solution.tour, seg_start, seg_len, insert_pos) {
                                if self.first_improvement {
                                    self.apply_relocation(&mut solution.tour, seg_start, seg_len, insert_pos);
                                    solution.cost += delta;
                                    improved = true;
                                    total_improved = true;
                                    break;
                                } else if delta < best_delta {
                                    best_delta = delta;
                                    best_seg_start = seg_start;
                                    best_seg_len = seg_len;
                                    best_insert_pos = insert_pos;
                                }
                            }
                        }
                    }
                    if improved && self.first_improvement {
                        break;
                    }
                }
                if improved && self.first_improvement {
                    break;
                }
            }
            
            if !self.first_improvement && best_delta < -1e-9 {
                self.apply_relocation(&mut solution.tour, best_seg_start, best_seg_len, best_insert_pos);
                solution.cost += best_delta;
                improved = true;
                total_improved = true;
            }
        }
        
        solution.validate(instance);
        total_improved
    }
    
    fn name(&self) -> &str {
        "Or-Opt"
    }
}

 

/// Node Swap Local Search
/// 
/// Swaps pairs of nodes to improve tour quality.
pub struct SwapSearch {
    /// Use first improvement
    pub first_improvement: bool,
}

impl SwapSearch {
    pub fn new() -> Self {
        SwapSearch {
            first_improvement: false,
        }
    }
    
    pub fn first_improvement() -> Self {
        SwapSearch {
            first_improvement: true,
        }
    }
    
    /// Check if swap maintains feasibility
    fn is_feasible_swap(&self, instance: &PDTSPInstance, tour: &[usize], i: usize, j: usize) -> bool {
        let mut new_tour = tour.to_vec();
        new_tour.swap(i, j);
        instance.is_feasible(&new_tour)
    }
}

impl Default for SwapSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalSearch for SwapSearch {
    fn improve(&self, instance: &PDTSPInstance, solution: &mut Solution) -> bool {
        let n = solution.tour.len();
        if n < 3 { return false; }
        
        let mut improved = true;
        let mut total_improved = false;
        let mut iterations = 0;
        let max_iterations = 20;
        
        while improved && iterations < max_iterations {
            improved = false;
            let mut best_delta = 0.0;
            let mut best_i = 0;
            let mut best_j = 0;
            iterations += 1;
            
            for i in 1..n - 1 {
                for j in i + 1..n {
                    // Don't swap depot
                    if solution.tour[i] == 0 || solution.tour[j] == 0 {
                        continue;
                    }
                    
                    let delta = solution.swap_delta(instance, i, j);
                    
                    if delta < -1e-9 {
                        if self.is_feasible_swap(instance, &solution.tour, i, j) {
                            if self.first_improvement {
                                solution.apply_swap(i, j);
                                solution.cost += delta;
                                improved = true;
                                total_improved = true;
                                break;
                            } else if delta < best_delta {
                                best_delta = delta;
                                best_i = i;
                                best_j = j;
                            }
                        }
                    }
                }
                if improved && self.first_improvement {
                    break;
                }
            }
            
            if !self.first_improvement && best_delta < -1e-9 {
                solution.apply_swap(best_i, best_j);
                solution.cost += best_delta;
                improved = true;
                total_improved = true;
            }
        }
        
        solution.validate(instance);
        total_improved
    }
    
    fn name(&self) -> &str {
        "Swap"
    }
}

 

/// Node Relocation Local Search
/// 
/// Removes a node and reinserts it at a better position.
pub struct RelocationSearch {
    /// Use first improvement
    pub first_improvement: bool,
}

impl RelocationSearch {
    pub fn new() -> Self {
        RelocationSearch {
            first_improvement: false,
        }
    }
    
    pub fn first_improvement() -> Self {
        RelocationSearch {
            first_improvement: true,
        }
    }
    
    /// Calculate relocation delta
    fn relocation_delta(&self, instance: &PDTSPInstance, tour: &[usize], from: usize, to: usize) -> f64 {
        if from == to || from + 1 == to {
            return 0.0;
        }
        
        let n = tour.len();
        let node = tour[from];
        let prev_from = if from == 0 { n - 1 } else { from - 1 };
        let next_from = (from + 1) % n;
        
        
        let removal = -instance.distance(tour[prev_from], node)
            - instance.distance(node, tour[next_from])
            + instance.distance(tour[prev_from], tour[next_from]);
        
        
        let adj_to = if to > from { to - 1 } else { to };
        let prev_to = if adj_to == 0 { n - 2 } else { adj_to - 1 };
        let next_to = adj_to;
        
        
        let actual_prev = if prev_to == from { tour[prev_from] }
            else if prev_to > from { tour[prev_to + 1] }
            else { tour[prev_to] };
        
        let actual_next = if next_to == from { tour[next_from] }
            else if next_to > from { tour[next_to + 1] }
            else { tour[next_to] };
        
        
        let insertion = instance.distance(actual_prev, node)
            + instance.distance(node, actual_next)
            - instance.distance(actual_prev, actual_next);
        
        removal + insertion
    }
    
    /// Check if relocation maintains feasibility
    fn is_feasible_relocation(&self, instance: &PDTSPInstance, tour: &[usize], from: usize, to: usize) -> bool {
        let mut new_tour = tour.to_vec();
        let node = new_tour.remove(from);
        let insert_pos = if to > from { to - 1 } else { to };
        new_tour.insert(insert_pos, node);
        instance.is_feasible(&new_tour)
    }
}

impl Default for RelocationSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalSearch for RelocationSearch {
    fn improve(&self, instance: &PDTSPInstance, solution: &mut Solution) -> bool {
        let n = solution.tour.len();
        if n < 3 { return false; }
        
        let mut improved = true;
        let mut total_improved = false;
        let mut iterations = 0;
        let max_iterations = 20;
        
        while improved && iterations < max_iterations {
            improved = false;
            let mut best_delta = 0.0;
            let mut best_from = 0;
            let mut best_to = 0;
            iterations += 1;
            
            for from in 0..n {
                
                if solution.tour[from] == 0 {
                    continue;
                }
                
                for to in 0..n {
                    if to == from || to == from + 1 {
                        continue;
                    }
                    
                    let delta = self.relocation_delta(instance, &solution.tour, from, to);
                    
                    if delta < -1e-9 {
                        if self.is_feasible_relocation(instance, &solution.tour, from, to) {
                            if self.first_improvement {
                                solution.apply_insertion(from, to);
                                solution.cost += delta;
                                improved = true;
                                total_improved = true;
                                break;
                            } else if delta < best_delta {
                                best_delta = delta;
                                best_from = from;
                                best_to = to;
                            }
                        }
                    }
                }
                if improved && self.first_improvement {
                    break;
                }
            }
            
            if !self.first_improvement && best_delta < -1e-9 {
                solution.apply_insertion(best_from, best_to);
                solution.cost += best_delta;
                improved = true;
                total_improved = true;
            }
        }
        
        solution.validate(instance);
        total_improved
    }
    
    fn name(&self) -> &str {
        "Relocation"
    }
}

 

/// Variable Neighborhood Descent (VND)
/// 
/// Applies multiple local search operators in a systematic way.
pub struct VND {
    /// List of local search operators
    operators: Vec<Box<dyn LocalSearch + Send + Sync>>,
}

impl VND {
    pub fn new() -> Self {
        VND {
            operators: Vec::new(),
        }
    }
    
    pub fn with_standard_operators() -> Self {
        let operators: Vec<Box<dyn LocalSearch + Send + Sync>> = vec![
            Box::new(TwoOptSearch::first_improvement()),
            Box::new(SwapSearch::first_improvement()),
            Box::new(RelocationSearch::first_improvement()),
            Box::new(OrOptSearch::first_improvement()),
        ];
        
        VND { operators }
    }
    
    pub fn add_operator<L: LocalSearch + Send + Sync + 'static>(&mut self, op: L) {
        self.operators.push(Box::new(op));
    }
}

impl Default for VND {
    fn default() -> Self {
        Self::with_standard_operators()
    }
}

impl LocalSearch for VND {
    fn improve(&self, instance: &PDTSPInstance, solution: &mut Solution) -> bool {
        let mut total_improved = false;
        let mut k = 0;
        let mut total_iterations = 0;
        let max_total_iterations = 100; // Prevent infinite loops
        
        while k < self.operators.len() && total_iterations < max_total_iterations {
            if self.operators[k].improve(instance, solution) {
                total_improved = true;
                k = 0; // Restart from first operator
            } else {
                k += 1; // Move to next operator
            }
            total_iterations += 1;
        }
        
        total_improved
    }
    
    fn name(&self) -> &str {
        "VND"
    }
}

 

/// Simulated Annealing
/// 
/// Metaheuristic that accepts worse solutions with decreasing probability.
pub struct SimulatedAnnealing {
    /// Initial temperature
    pub initial_temp: f64,
    /// Final temperature
    pub final_temp: f64,
    /// Cooling rate
    pub cooling_rate: f64,
    /// Iterations per temperature
    pub iterations_per_temp: usize,
    /// Random seed
    pub seed: u64,
}

impl SimulatedAnnealing {
    pub fn new() -> Self {
        SimulatedAnnealing {
            initial_temp: 1000.0,
            final_temp: 0.1,
            cooling_rate: 0.995,
            iterations_per_temp: 100,
            seed: 42,
        }
    }
    
    pub fn with_params(initial_temp: f64, final_temp: f64, cooling_rate: f64, iterations_per_temp: usize) -> Self {
        SimulatedAnnealing {
            initial_temp,
            final_temp,
            cooling_rate,
            iterations_per_temp,
            seed: 42,
        }
    }
    
    /// Generate a random neighbor solution
    fn generate_neighbor(&self, instance: &PDTSPInstance, solution: &Solution, rng: &mut ChaCha8Rng) -> Option<(Vec<usize>, f64)> {
        let n = solution.tour.len();
        
        
        let move_type = rng.gen_range(0..4);
        
        match move_type {
            0 => {
                
                let i = rng.gen_range(0..n - 2);
                let j = rng.gen_range(i + 2..n);
                
                let mut new_tour = solution.tour.clone();
                new_tour[i + 1..=j].reverse();
                
                if instance.is_feasible(&new_tour) {
                    let delta = solution.two_opt_delta(instance, i, j);
                    Some((new_tour, delta))
                } else {
                    None
                }
            }
            1 => {
                // Swap
                let i = rng.gen_range(1..n);
                let j = rng.gen_range(1..n);
                if i == j || solution.tour[i] == 0 || solution.tour[j] == 0 {
                    return None;
                }
                
                let mut new_tour = solution.tour.clone();
                new_tour.swap(i, j);
                
                if instance.is_feasible(&new_tour) {
                    let delta = solution.swap_delta(instance, i, j);
                    Some((new_tour, delta))
                } else {
                    None
                }
            }
            2 => {
                // Relocation
                let from = rng.gen_range(1..n);
                if solution.tour[from] == 0 {
                    return None;
                }
                let to = rng.gen_range(0..n);
                if to == from || to == from + 1 {
                    return None;
                }
                
                let mut new_tour = solution.tour.clone();
                let node = new_tour.remove(from);
                let insert_pos = if to > from { to - 1 } else { to };
                new_tour.insert(insert_pos, node);
                
                if instance.is_feasible(&new_tour) {
                    let new_cost = instance.tour_length(&new_tour);
                    let delta = new_cost - solution.cost;
                    Some((new_tour, delta))
                } else {
                    None
                }
            }
            _ => {
                // Or-opt (segment of length 2)
                if n < 4 {
                    return None;
                }
                let seg_start = rng.gen_range(1..n - 1);
                if solution.tour[seg_start] == 0 {
                    return None;
                }
                let insert_pos = rng.gen_range(0..n - 1);
                if insert_pos >= seg_start && insert_pos <= seg_start + 2 {
                    return None;
                }
                
                let mut new_tour = Vec::new();
                let segment: Vec<usize> = solution.tour[seg_start..seg_start + 2.min(n - seg_start)].to_vec();
                
                for (i, &node) in solution.tour.iter().enumerate() {
                    if i == insert_pos && insert_pos < seg_start {
                        new_tour.extend(&segment);
                    }
                    if i < seg_start || i >= seg_start + segment.len() {
                        new_tour.push(node);
                    }
                    if i == insert_pos && insert_pos > seg_start + segment.len() {
                        new_tour.extend(&segment);
                    }
                }
                
                if insert_pos >= solution.tour.len() - segment.len() {
                    new_tour.extend(&segment);
                }
                
                if new_tour.len() == solution.tour.len() && instance.is_feasible(&new_tour) {
                    let new_cost = instance.tour_length(&new_tour);
                    let delta = new_cost - solution.cost;
                    Some((new_tour, delta))
                } else {
                    None
                }
            }
        }
    }
}

impl Default for SimulatedAnnealing {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalSearch for SimulatedAnnealing {
    fn improve(&self, instance: &PDTSPInstance, solution: &mut Solution) -> bool {
        let n = solution.tour.len();
        if n < 3 { return false; }
        let mut rng = ChaCha8Rng::seed_from_u64(self.seed);
        
        let mut current_tour = solution.tour.clone();
        let mut current_cost = solution.cost;
        let mut best_tour = current_tour.clone();
        let mut best_cost = current_cost;
        
        let mut temp = self.initial_temp;
        let mut iterations = 0;
        
        while temp > self.final_temp {
            for _ in 0..self.iterations_per_temp {
                let total_profit = instance.tour_profit(&current_tour);
                let temp_solution = Solution {
                    tour: current_tour.clone(),
                    cost: current_cost,
                    feasible: true,
                    algorithm: String::new(),
                    computation_time: 0.0,
                    iterations: None,
                    total_profit,
                    objective: total_profit as f64 - current_cost,
                };
                
                if let Some((new_tour, delta)) = self.generate_neighbor(instance, &temp_solution, &mut rng) {
                    let new_cost = current_cost + delta;
                    
                    // Accept if better or with probability
                    let accept = if delta < 0.0 {
                        true
                    } else {
                        let prob = (-delta / temp).exp();
                        rng.gen::<f64>() < prob
                    };
                    
                    if accept {
                        current_tour = new_tour;
                        current_cost = new_cost;
                        
                        if current_cost < best_cost {
                            best_tour = current_tour.clone();
                            best_cost = current_cost;
                        }
                    }
                }
                
                iterations += 1;
            }
            
            temp *= self.cooling_rate;
        }
        
        let improved = best_cost < solution.cost - 1e-9;
        
        solution.tour = best_tour;
        solution.cost = best_cost;
        solution.iterations = Some(iterations);
        solution.validate(instance);
        
        improved
    }
    
    fn name(&self) -> &str {
        "SimulatedAnnealing"
    }
}

// ==================== Tabu Search ====================

/// Tabu Search
/// 
/// Local search with memory to avoid cycling.
pub struct TabuSearch {
    /// Tabu tenure (how long a move stays tabu)
    pub tenure: usize,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Maximum iterations without improvement
    pub max_no_improve: usize,
}

impl TabuSearch {
    pub fn new() -> Self {
        TabuSearch {
            tenure: 10,
            max_iterations: 1000,
            max_no_improve: 100,
        }
    }
    
    pub fn with_params(tenure: usize, max_iterations: usize, max_no_improve: usize) -> Self {
        TabuSearch {
            tenure,
            max_iterations,
            max_no_improve,
        }
    }
}

impl Default for TabuSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalSearch for TabuSearch {
    fn improve(&self, instance: &PDTSPInstance, solution: &mut Solution) -> bool {
        let n = solution.tour.len();
        if n < 3 { return false; }
        let n = solution.tour.len();
        
        // Tabu list: (node1, node2) -> expiry iteration
        let mut tabu_list: std::collections::HashMap<(usize, usize), usize> = std::collections::HashMap::new();
        
        let mut current_tour = solution.tour.clone();
        let mut current_cost = solution.cost;
        let mut best_tour = current_tour.clone();
        let mut best_cost = current_cost;
        
        let mut iteration = 0;
        let mut no_improve = 0;
        
        while iteration < self.max_iterations && no_improve < self.max_no_improve {
            let mut best_move_delta = f64::INFINITY;
            let mut best_move_i = 0;
            let mut best_move_j = 0;
            let mut best_move_type = 0; // 0 = swap, 1 = 2-opt
            
            // Evaluate all possible moves
            for i in 1..n - 1 {
                for j in i + 1..n {
                    if current_tour[i] == 0 || current_tour[j] == 0 {
                        continue;
                    }
                    
                    // Check swap
                    let mut test_tour = current_tour.clone();
                    test_tour.swap(i, j);
                    
                    if instance.is_feasible(&test_tour) {
                        let new_cost = instance.tour_length(&test_tour);
                        let delta = new_cost - current_cost;
                        
                        let tabu_key = (current_tour[i].min(current_tour[j]), 
                                       current_tour[i].max(current_tour[j]));
                        let is_tabu = tabu_list.get(&tabu_key)
                            .map(|&exp| exp > iteration)
                            .unwrap_or(false);
                        
                        // Aspiration: accept if better than best known
                        let accept = !is_tabu || new_cost < best_cost;
                        
                        if accept && delta < best_move_delta {
                            best_move_delta = delta;
                            best_move_i = i;
                            best_move_j = j;
                            best_move_type = 0;
                        }
                    }
                    
                    // Check 2-opt
                    if j > i + 1 {
                        let mut test_tour = current_tour.clone();
                        test_tour[i + 1..=j].reverse();
                        
                        if instance.is_feasible(&test_tour) {
                            let new_cost = instance.tour_length(&test_tour);
                            let delta = new_cost - current_cost;
                            
                            let tabu_key = (current_tour[i].min(current_tour[j]), 
                                           current_tour[i].max(current_tour[j]));
                            let is_tabu = tabu_list.get(&tabu_key)
                                .map(|&exp| exp > iteration)
                                .unwrap_or(false);
                            
                            let accept = !is_tabu || new_cost < best_cost;
                            
                            if accept && delta < best_move_delta {
                                best_move_delta = delta;
                                best_move_i = i;
                                best_move_j = j;
                                best_move_type = 1;
                            }
                        }
                    }
                }
            }
            
            // Apply best move
            if best_move_delta < f64::INFINITY {
                if best_move_type == 0 {
                    let tabu_key = (current_tour[best_move_i].min(current_tour[best_move_j]),
                                   current_tour[best_move_i].max(current_tour[best_move_j]));
                    current_tour.swap(best_move_i, best_move_j);
                    tabu_list.insert(tabu_key, iteration + self.tenure);
                } else {
                    let tabu_key = (current_tour[best_move_i].min(current_tour[best_move_j]),
                                   current_tour[best_move_i].max(current_tour[best_move_j]));
                    current_tour[best_move_i + 1..=best_move_j].reverse();
                    tabu_list.insert(tabu_key, iteration + self.tenure);
                }
                
                current_cost += best_move_delta;
                
                if current_cost < best_cost - 1e-9 {
                    best_tour = current_tour.clone();
                    best_cost = current_cost;
                    no_improve = 0;
                } else {
                    no_improve += 1;
                }
            } else {
                no_improve += 1;
            }
            
            iteration += 1;
        }
        
        let improved = best_cost < solution.cost - 1e-9;
        
        solution.tour = best_tour;
        solution.cost = best_cost;
        solution.iterations = Some(iteration);
        solution.validate(instance);
        
        improved
    }
    
    fn name(&self) -> &str {
        "TabuSearch"
    }
}

// ==================== Iterated Local Search ====================

/// Iterated Local Search
/// 
/// Applies local search, then perturbation, then local search again.
pub struct IteratedLocalSearch {
    /// Number of perturbation moves
    pub perturbation_strength: usize,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Maximum iterations without improvement
    pub max_no_improve: usize,
    /// Random seed
    pub seed: u64,
}

impl IteratedLocalSearch {
    pub fn new() -> Self {
        IteratedLocalSearch {
            perturbation_strength: 3,
            max_iterations: 100,
            max_no_improve: 20,
            seed: 42,
        }
    }
    
    pub fn with_params(perturbation_strength: usize, max_iterations: usize, max_no_improve: usize) -> Self {
        IteratedLocalSearch {
            perturbation_strength,
            max_iterations,
            max_no_improve,
            seed: 42,
        }
    }
    
    /// Perturb solution by applying random moves
    fn perturb(&self, instance: &PDTSPInstance, tour: &mut Vec<usize>, rng: &mut ChaCha8Rng) {
        let n = tour.len();
        
        for _ in 0..self.perturbation_strength {
            // Try random 2-opt or swap
            if rng.gen_bool(0.5) {
                // Random 2-opt
                let i = rng.gen_range(0..n - 2);
                let j = rng.gen_range(i + 2..n);
                
                let mut new_tour = tour.clone();
                new_tour[i + 1..=j].reverse();
                
                if instance.is_feasible(&new_tour) {
                    *tour = new_tour;
                }
            } else {
                // Random swap
                let i = rng.gen_range(1..n);
                let j = rng.gen_range(1..n);
                
                if i != j && tour[i] != 0 && tour[j] != 0 {
                    let mut new_tour = tour.clone();
                    new_tour.swap(i, j);
                    
                    if instance.is_feasible(&new_tour) {
                        *tour = new_tour;
                    }
                }
            }
        }
    }
}

impl Default for IteratedLocalSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalSearch for IteratedLocalSearch {
    fn improve(&self, instance: &PDTSPInstance, solution: &mut Solution) -> bool {
        let n = solution.tour.len();
        if n < 3 { return false; }
        let mut rng = ChaCha8Rng::seed_from_u64(self.seed);
        let vnd = VND::with_standard_operators();
        
        // Apply initial local search
        vnd.improve(instance, solution);
        
        let mut best_tour = solution.tour.clone();
        let mut best_cost = solution.cost;
        
        let mut current_tour = solution.tour.clone();
        let mut current_cost = solution.cost;
        
        let mut no_improve = 0;
        let mut iteration = 0;
        
        while iteration < self.max_iterations && no_improve < self.max_no_improve {
            // Perturb current solution
            let mut perturbed = current_tour.clone();
            self.perturb(instance, &mut perturbed, &mut rng);
            
            // Apply local search to perturbed solution
            let mut perturbed_solution = Solution::from_tour(instance, perturbed, "ILS-temp");
            vnd.improve(instance, &mut perturbed_solution);
            
            // Acceptance criterion (accept if better than current)
            if perturbed_solution.cost < current_cost {
                current_tour = perturbed_solution.tour;
                current_cost = perturbed_solution.cost;
                
                if current_cost < best_cost - 1e-9 {
                    best_tour = current_tour.clone();
                    best_cost = current_cost;
                    no_improve = 0;
                } else {
                    no_improve += 1;
                }
            } else {
                no_improve += 1;
            }
            
            iteration += 1;
        }
        
        let improved = best_cost < solution.cost - 1e-9;
        
        solution.tour = best_tour;
        solution.cost = best_cost;
        solution.iterations = Some(iteration);
        solution.validate(instance);
        
        improved
    }
    
    fn name(&self) -> &str {
        "ILS"
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
    fn test_two_opt() {
        let instance = create_test_instance();
        let mut solution = Solution::from_tour(&instance, vec![0, 1, 2, 3], "test");
        
        let two_opt = TwoOptSearch::new();
        two_opt.improve(&instance, &mut solution);
        
        assert!(solution.feasible);
    }
}
