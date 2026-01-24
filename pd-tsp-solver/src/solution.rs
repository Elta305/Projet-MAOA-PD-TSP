//! Solution representation and manipulation for PD-TSP.
//! 
//! This module provides data structures and methods for representing,
//! manipulating, and evaluating solutions to the PD-TSP.

use crate::instance::PDTSPInstance;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Represents a solution to the PD-TSP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solution {
    /// The tour as a sequence of node indices (starting and ending at depot 0)
    pub tour: Vec<usize>,
    /// Total tour length/cost
    pub cost: f64,
    /// Total profit collected along the tour
    pub total_profit: i32,
    /// Objective value Z = total_profit - travel_cost
    pub objective: f64,
    /// Whether the solution is feasible
    pub feasible: bool,
    /// Algorithm that generated this solution
    pub algorithm: String,
    /// Computation time in seconds
    pub computation_time: f64,
    /// Number of iterations (if applicable)
    pub iterations: Option<usize>,
}

impl Solution {
    /// Create a new empty solution
    pub fn new() -> Self {
        Solution {
            tour: Vec::new(),
            cost: f64::INFINITY,
            feasible: false,
            algorithm: String::new(),
            computation_time: 0.0,
            iterations: None,
            total_profit: 0,
            objective: f64::NEG_INFINITY,
        }
    }
    
    /// Create a solution from a tour
    pub fn from_tour(instance: &PDTSPInstance, tour: Vec<usize>, algorithm: &str) -> Self {
        let travel_cost = instance.tour_cost(&tour);
        let feasible = instance.is_feasible(&tour);
        let total_profit = instance.tour_profit(&tour);
        let objective = total_profit as f64 - travel_cost;

        Solution {
            tour,
            cost: travel_cost,
            feasible,
            algorithm: algorithm.to_string(),
            computation_time: 0.0,
            iterations: None,
            total_profit,
            objective,
        }
    }
    
    /// Validate and update solution properties
    pub fn validate(&mut self, instance: &PDTSPInstance) {
        let travel_cost = instance.tour_cost(&self.tour);
        self.cost = travel_cost;
        self.feasible = instance.is_feasible(&self.tour);
        self.total_profit = instance.tour_profit(&self.tour);
        self.objective = self.total_profit as f64 - travel_cost;
    }
    
    /// Check if all nodes are visited exactly once
    pub fn is_complete(&self, instance: &PDTSPInstance) -> bool {
        if self.tour.len() != instance.dimension {
            return false;
        }
        
        let unique: HashSet<usize> = self.tour.iter().cloned().collect();
        unique.len() == instance.dimension && self.tour[0] == 0
    }
    
    /// Get the position of a node in the tour
    pub fn position(&self, node: usize) -> Option<usize> {
        self.tour.iter().position(|&n| n == node)
    }
    
    /// Get the node at a given position (circular)
    pub fn node_at(&self, pos: usize) -> usize {
        self.tour[pos % self.tour.len()]
    }
    
    /// Get the successor of a node in the tour
    pub fn successor(&self, node: usize) -> Option<usize> {
        self.position(node).map(|pos| self.node_at(pos + 1))
    }
    
    /// Get the predecessor of a node in the tour
    pub fn predecessor(&self, node: usize) -> Option<usize> {
        self.position(node).map(|pos| {
            if pos == 0 {
                self.tour[self.tour.len() - 1]
            } else {
                self.tour[pos - 1]
            }
        })
    }
    
    /// Calculate the delta cost of swapping two nodes
    pub fn swap_delta(&self, instance: &PDTSPInstance, i: usize, j: usize) -> f64 {
        if i == j || self.tour.len() < 4 {
            return 0.0;
        }

        
        let mut new_tour = self.tour.clone();
        new_tour.swap(i, j);
        let old_cost = instance.tour_cost(&self.tour);
        let new_cost = instance.tour_cost(&new_tour);
        new_cost - old_cost
    }
    
    /// Calculate the delta cost of a 2-opt move
    pub fn two_opt_delta(&self, instance: &PDTSPInstance, i: usize, j: usize) -> f64 {
        let n = self.tour.len();
        if i >= j || j >= n {
            return 0.0;
        }

        
        let mut new_tour = self.tour.clone();
        new_tour[i + 1..=j].reverse();
        let old_cost = instance.tour_cost(&self.tour);
        let new_cost = instance.tour_cost(&new_tour);
        new_cost - old_cost
    }
    
    /// Apply a 2-opt move (reverse segment between i+1 and j)
    pub fn apply_two_opt(&mut self, i: usize, j: usize) {
        self.tour[i + 1..=j].reverse();
    }
    
    /// Apply a swap move
    pub fn apply_swap(&mut self, i: usize, j: usize) {
        self.tour.swap(i, j);
    }
    
    /// Apply an insertion move (remove node at from_pos and insert at to_pos)
    pub fn apply_insertion(&mut self, from_pos: usize, to_pos: usize) {
        let node = self.tour.remove(from_pos);
        let insert_pos = if to_pos > from_pos { to_pos - 1 } else { to_pos };
        self.tour.insert(insert_pos, node);
    }
    
    /// Calculate insertion delta (remove from from_pos, insert at to_pos)
    pub fn insertion_delta(&self, instance: &PDTSPInstance, from_pos: usize, to_pos: usize) -> f64 {
        if from_pos == to_pos || from_pos + 1 == to_pos {
            return 0.0;
        }
        
        let mut new_tour: Vec<usize> = self.tour.clone();
        let node = new_tour.remove(from_pos);
        let insert_pos = if to_pos > from_pos { to_pos - 1 } else { to_pos };
        new_tour.insert(insert_pos, node);

        let old_cost = instance.tour_cost(&self.tour);
        let new_cost = instance.tour_cost(&new_tour);
        new_cost - old_cost
    }
    
    /// Get load profile along the tour (including return to depot)
    pub fn load_profile(&self, instance: &PDTSPInstance) -> Vec<i32> {
        if self.tour.is_empty() {
            return Vec::new();
        }

        let mut load = instance.starting_load();
        let mut profile = Vec::with_capacity(self.tour.len() + 2);

        profile.push(load);

        for &node in self.tour.iter().skip(1) {
            if node == 0 {
                // Intermediate depot visit: deliver all current load
                load = 0;
            } else {
                load += instance.nodes[node].demand;
            }
            profile.push(load);
        }

        // Return to depot: deliver all remaining load (should be 0 for feasible tours)
        profile.push(0);

        profile
    }
    
    /// Get maximum load during tour
    pub fn max_load(&self, instance: &PDTSPInstance) -> i32 {
        self.load_profile(instance).into_iter().max().unwrap_or(0)
    }
    
    /// Get minimum load during tour
    pub fn min_load(&self, instance: &PDTSPInstance) -> i32 {
        self.load_profile(instance).into_iter().min().unwrap_or(0)
    }
}

impl Default for Solution {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for Solution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Solution ({})", self.algorithm)?;
        writeln!(f, "  Cost: {:.2}", self.cost)?;
        writeln!(f, "  Feasible: {}", self.feasible)?;
        writeln!(f, "  Time: {:.4}s", self.computation_time)?;
        if let Some(iter) = self.iterations {
            writeln!(f, "  Iterations: {}", iter)?;
        }
        writeln!(f, "  Tour: {:?}", self.tour)
    }
}

/// Represents a move in local search
#[derive(Debug, Clone, Copy)]
pub enum Move {
    Swap(usize, usize),
    TwoOpt(usize, usize),
    Insertion(usize, usize),
    OrOpt(usize, usize, usize), // segment start, length, insertion position
}

impl Move {
    pub fn delta(&self, solution: &Solution, instance: &PDTSPInstance) -> f64 {
        match *self {
            Move::Swap(i, j) => solution.swap_delta(instance, i, j),
            Move::TwoOpt(i, j) => solution.two_opt_delta(instance, i, j),
            Move::Insertion(from, to) => solution.insertion_delta(instance, from, to),
            Move::OrOpt(_, _, _) => 0.0, // Computed separately
        }
    }
    
    pub fn apply(&self, solution: &mut Solution) {
        match *self {
            Move::Swap(i, j) => solution.apply_swap(i, j),
            Move::TwoOpt(i, j) => solution.apply_two_opt(i, j),
            Move::Insertion(from, to) => solution.apply_insertion(from, to),
            Move::OrOpt(start, len, to) => {
                
                let segment: Vec<usize> = solution.tour.drain(start..start + len).collect();
                let insert_pos = if to > start { to - len } else { to };
                for (i, node) in segment.into_iter().enumerate() {
                    solution.tour.insert(insert_pos + i, node);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_solution_creation() {
        let sol = Solution::new();
        assert!(sol.tour.is_empty());
        assert!(!sol.feasible);
        assert_eq!(sol.cost, f64::INFINITY);
    }
}
