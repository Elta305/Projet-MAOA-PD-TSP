//! Module for parsing and representing PD-TSP instances.
//! 
//! This module handles the TSP-LIB format files used for the Pickup and Delivery TSP.
//! It supports Euclidean 2D distances and manages node coordinates, demands, and capacity constraints.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use serde::{Deserialize, Serialize};

/// Represents a node in the PD-TSP instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Node identifier (1-indexed in files, 0-indexed internally)
    pub id: usize,
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
    /// Demand (internal convention): positive = pickup (increases load when visited),
    /// negative = delivery (decreases load when visited), 0 = neutral.
    pub demand: i32,
    /// Profit/value associated with this node (optional)
    pub profit: i32,
}

impl Node {
    pub fn new(id: usize, x: f64, y: f64, demand: i32, profit: i32) -> Self {
        Node { id, x, y, demand, profit }
    }
    
    /// Check if this node is a pickup node (positive demand = load items)
    pub fn is_pickup(&self) -> bool {
        self.demand > 0
    }
    
    /// Check if this node is a delivery node (negative demand = unload items)
    pub fn is_delivery(&self) -> bool {
        self.demand < 0
    }
    
    /// Check if this node is the depot
    pub fn is_depot(&self) -> bool {
        self.id == 0
    }
}

/// Represents a complete PD-TSP instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PDTSPInstance {
    /// Name of the instance
    pub name: String,
    /// Comment/description
    pub comment: String,
    /// Number of nodes (including depot)
    pub dimension: usize,
    /// Vehicle capacity
    pub capacity: i32,
    /// List of all nodes
    pub nodes: Vec<Node>,
    /// Precomputed distance matrix
    #[serde(skip)]
    pub distance_matrix: Vec<Vec<f64>>,
    /// Demand at return depot (node n+1 in original file, applied when returning to depot)
    pub return_depot_demand: i32,
    /// Selected cost function for travel cost evaluation
    pub cost_function: CostFunction,
    /// Alpha parameter for quadratic cost
    pub alpha: f64,
    /// Beta parameter for linear-load cost
    pub beta: f64,
}

/// Cost function choices for travel cost
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum CostFunction {
    Distance,
    Quadratic,
    LinearLoad,
}

impl PDTSPInstance {
    /// Initial load after processing depot demand at departure.
    /// The vehicle starts at the depot with demand from depot node.
    /// For PD-TSP, the depot demand represents the initial load.
    #[inline]
    pub fn starting_load(&self) -> i32 {
        // Simply return the depot demand as the starting load
        // Positive = we start with items to deliver
        // Negative = we need to pick up items first (start at 0)
        // For standard PD-TSP instances, depot demand is typically the initial load
        self.nodes[0].demand.max(0)
    }

    /// Return the load after the initial depot visit.
    /// This is the same as starting_load since we process depot at departure.
    #[inline]
    pub fn load_after_initial_deposit(&self) -> i32 {
        self.starting_load()
    }
    
    /// Return the capacity of the depot to receive deliveries.
    /// This is the absolute value of the depot's negative demand.
    #[inline]
    pub fn depot_receiving_capacity(&self) -> i32 {
        // Depot demand is negative, indicating receiving capacity
        (-self.nodes[0].demand).max(0)
    }

    /// Parse a PD-TSP instance from a TSP-LIB format file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file = File::open(&path)
            .map_err(|e| format!("Cannot open file: {}", e))?;
        let reader = BufReader::new(file);
        
        let mut name = String::new();
        let mut comment = String::new();
        let mut dimension = 0usize;
        let mut capacity = 0i32;
        let mut coords: Vec<(usize, f64, f64)> = Vec::new();
        let mut demands: Vec<(usize, i32)> = Vec::new();
        
        let mut section = String::new();
        
        for line in reader.lines() {
            let line = line.map_err(|e| format!("Read error: {}", e))?;
            let line = line.trim();
            
            if line.is_empty() || line == "EOF" {
                continue;
            }
            
            
            if line.starts_with("NAME:") {
                name = line.replace("NAME:", "").trim().to_string();
                continue;
            }
            if line.starts_with("COMMENT:") {
                comment = line.replace("COMMENT:", "").trim().to_string();
                continue;
            }
            if line.starts_with("DIMENSION:") {
                dimension = line.replace("DIMENSION:", "").trim()
                    .parse().map_err(|_| "Invalid dimension")?;
                continue;
            }
            if line.starts_with("CAPACITY:") {
                capacity = line.replace("CAPACITY:", "").trim()
                    .parse().map_err(|_| "Invalid capacity")?;
                continue;
            }
            if line.starts_with("EDGE_WEIGHT_TYPE:") {
                continue;
            }
            
            
            if line.starts_with("NODE_COORD_SECTION") {
                section = "coords".to_string();
                continue;
            }
            if line.starts_with("DISPLAY_DATA_SECTION") {
                section = "display".to_string();
                continue;
            }
            if line.starts_with("DEMAND_SECTION") {
                section = "demands".to_string();
                continue;
            }
            
            
            match section.as_str() {
                "coords" => {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let id: usize = parts[0].parse().map_err(|_| "Invalid node id")?;
                        let x: f64 = parts[1].parse().map_err(|_| "Invalid x coordinate")?;
                        let y: f64 = parts[2].parse().map_err(|_| "Invalid y coordinate")?;
                        coords.push((id, x, y));
                    }
                }
                "demands" => {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let id: usize = parts[0].parse().map_err(|_| "Invalid node id")?;
                        let demand: i32 = parts[1].parse().map_err(|_| "Invalid demand")?;
                        demands.push((id, demand));
                    }
                }
                _ => {}
            }
        }
        
        
        let has_duplicate_depot = if coords.len() >= 2 {
            let first = &coords[0];
            let last = &coords[coords.len() - 1];
            (first.1 - last.1).abs() < 1e-6 && (first.2 - last.2).abs() < 1e-6
        } else {
            false
        };

        // Determine actual number of nodes to load and the return-depot demand
        let (actual_dimension, return_depot_demand) = if has_duplicate_depot {
            // If the file contains a duplicate depot at the end, the DEMAND_SECTION
            // usually contains two depot entries: the first (id=1) is the initial
            // depot load, and the last (id=dimension) is the return-depot adjustment.
            let return_demand = demands.iter()
                .find(|(id, _)| *id == dimension)
                .map(|(_, d)| *d)
                .unwrap_or(0);
            (dimension - 1, return_demand)
        } else {
            // No explicit return-depot entry: the instance is already balanced
            // Calculate return_depot_demand as the negative of the total customer demand
            // to ensure the vehicle ends with 0 load
            let depot_demand = demands.iter().find(|(id, _)| *id == 1).map(|(_, d)| *d).unwrap_or(0);
            let customer_demands_sum: i32 = demands.iter()
                .filter(|(id, _)| *id > 1)
                .map(|(_, d)| *d)
                .sum();
            let return_demand = -(depot_demand + customer_demands_sum);
            (dimension, return_demand)
        };

        let mut nodes = Vec::with_capacity(actual_dimension);

        for (id, x, y) in coords.iter().take(actual_dimension) {
            let file_demand = demands.iter()
                .find(|(did, _)| *did == *id)
                .map(|(_, d)| *d)
                .unwrap_or(0);

            // Preserve the file demand for the depot (id==1) and customers alike.
            let internal_demand = file_demand;
            nodes.push(Node::new(id - 1, *x, *y, internal_demand, 0));
        }

        let distance_matrix = Self::compute_distance_matrix(&nodes);

        Ok(PDTSPInstance {
            name,
            comment,
            dimension: actual_dimension,
            capacity,
            nodes,
            distance_matrix,
            return_depot_demand,
            cost_function: CostFunction::Distance,
            alpha: 0.1,
            beta: 0.5,
        })
    }

    /// Compute travel cost according to the selected cost function stored in the instance
    pub fn tour_cost(&self, tour: &[usize]) -> f64 {
        match self.cost_function {
            CostFunction::Distance => self.tour_length(tour),
            CostFunction::Quadratic => self.tour_cost_quadratic(tour),
            CostFunction::LinearLoad => self.tour_cost_linear_load(tour, self.alpha),
        }
    }
    
    /// Compute Euclidean distance matrix
    fn compute_distance_matrix(nodes: &[Node]) -> Vec<Vec<f64>> {
        let n = nodes.len();
        let mut matrix = vec![vec![0.0; n]; n];
        
        for i in 0..n {
            for j in 0..n {
                if i != j {
                    let dx = nodes[i].x - nodes[j].x;
                    let dy = nodes[i].y - nodes[j].y;
                    matrix[i][j] = (dx * dx + dy * dy).sqrt();
                }
            }
        }
        
        matrix
    }
    
    /// Get the distance between two nodes
    #[inline]
    pub fn distance(&self, i: usize, j: usize) -> f64 {
        self.distance_matrix[i][j]
    }
    
    /// Get the number of customer nodes (excluding depot)
    pub fn num_customers(&self) -> usize {
        self.dimension - 1
    }
    
    /// Get all pickup nodes
    pub fn pickup_nodes(&self) -> Vec<usize> {
        self.nodes.iter()
            .filter(|n| n.is_pickup())
            .map(|n| n.id)
            .collect()
    }
    
    /// Get all delivery nodes
    pub fn delivery_nodes(&self) -> Vec<usize> {
        self.nodes.iter()
            .filter(|n| n.is_delivery())
            .map(|n| n.id)
            .collect()
    }
    
    /// Verify if a tour is feasible (respects capacity constraints)
    /// For PD-TSP: tour is [0, 1, 2, ..., n-1] and implicitly returns to 0
    /// Convention: positive demand = pickup (we load), negative demand = delivery (we unload)
    /// Vehicle starts EMPTY at the depot.
    pub fn is_feasible(&self, tour: &[usize]) -> bool {
        if tour.is_empty() || tour[0] != 0 {
            return false;
        }
        // Vehicle loads initial cargo and processes depot demand
        let mut load = self.starting_load();

        // Traverse all visited nodes after the initial depot
        for &node_id in tour.iter().skip(1) {
            if node_id == 0 {
                // Intermediate depot visit: deliver all current load to depot
                load = 0;
            } else {
                // Positive demand = pickup (increase load), negative = delivery (decrease load)
                load += self.nodes[node_id].demand;
            }

            if load < 0 || load > self.capacity {
                return false;
            }
        }

        // Implicit return to depot: we can deliver the remaining load at depot
        // The depot can receive up to its capacity (absolute value of its negative demand)
        // For Mosheiov instances, the final load should be depositable at depot
        // Since all load can be deposited at depot at the end, we just need load >= 0
        load >= 0
    }
    
    /// Check tour feasibility with detailed information
    /// Tour can be either:
    /// - [0, customers...] (implicit return to depot)
    /// - [0, customers..., 0] (explicit return to depot)
    /// Vehicle loads initial cargo and processes depot demand at start.
    pub fn check_feasibility_detailed(&self, tour: &[usize]) -> (bool, i32, i32, Vec<i32>) {
        // Vehicle loads initial cargo and processes depot demand
        let mut load = self.starting_load();
        let mut max_load = 0i32;
        let mut min_load = 0i32;
        let mut load_profile = Vec::with_capacity(tour.len() + 1);

        // record initial load at depot (0)
        load_profile.push(load);

        for &node_id in tour.iter().skip(1) {
            if node_id == 0 {
                // Intermediate depot visit: deliver all current load
                load = 0;
            } else {
                load += self.nodes[node_id].demand;
            }
            max_load = max_load.max(load);
            min_load = min_load.min(load);
            load_profile.push(load);
        }

        // Implicit return to depot: final load can be deposited at depot
        // so we just need it to be non-negative
        let feasible = max_load <= self.capacity && min_load >= 0 && load >= 0;
        (feasible, max_load, min_load, load_profile)
    }

    /// Check partial tour feasibility: ensure that during the partial tour the load
    /// never goes below 0 or above capacity. Unlike `is_feasible`, this does NOT
    /// require the final load to be zero (useful for construction heuristics testing
    /// intermediate insertions).
    /// Vehicle loads initial cargo and processes depot demand at start.
    pub fn is_partial_feasible(&self, tour: &[usize]) -> bool {
        if tour.is_empty() || tour[0] != 0 {
            return false;
        }
        // Vehicle loads initial cargo and processes depot demand
        let mut load = self.starting_load();

        for &node_id in tour.iter().skip(1) {
            if node_id == 0 {
                // Intermediate depot visit: deliver all current load
                load = 0;
            } else {
                load += self.nodes[node_id].demand;
            }

            if load < 0 || load > self.capacity {
                return false;
            }
        }

        true
    }
    
    /// Calculate total tour length (linear distance)
    pub fn tour_length(&self, tour: &[usize]) -> f64 {
        if tour.len() < 2 {
            return 0.0;
        }
        
        let mut length = 0.0;
        for i in 0..tour.len() - 1 {
            length += self.distance(tour[i], tour[i + 1]);
        }
        
        length += self.distance(tour[tour.len() - 1], tour[0]);
        
        length
    }

    /// Sum of profits collected along a tour (excluding depot)
    pub fn tour_profit(&self, tour: &[usize]) -> i32 {
        tour.iter().filter(|&&n| n != 0).map(|&n| self.nodes[n].profit).sum()
    }

    /// Assign random profits to customer nodes if none are present.
    /// Profits are integers in [10, max_profit] (clamped to 100). Deterministic via seed.
    pub fn assign_random_profits(&mut self, seed: u64, max_profit: i32) {
        
        let any_profit = self.nodes.iter().any(|n| n.profit != 0);
        if any_profit {
            return;
        }

        use rand::prelude::*;
        use rand_chacha::ChaCha8Rng;

        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        
        let upper = max_profit.clamp(10, 100);
        for (i, node) in self.nodes.iter_mut().enumerate() {
            if i == 0 {
                node.profit = 0; // depot has no profit
            } else {
                node.profit = rng.gen_range(10..=upper);
            }
        }
    }
    
    /// Calculate tour cost with an additive load-dependent quadratic surcharge
    /// Arc cost c(i->j) = distance(i,j) + (alpha * Wi + beta * Wi^2)
    /// where Wi is the load carried when leaving node i. Uses instance `alpha` and `beta`.
    pub fn tour_cost_quadratic(&self, tour: &[usize]) -> f64 {
        if tour.len() < 2 {
            return 0.0;
        }

        let mut cost = 0.0;
        
        // Vehicle starts with initial load (depot demands processed)
        let mut load = self.starting_load() as f64;

        for i in 0..tour.len() - 1 {
            let dist = self.distance(tour[i], tour[i + 1]);
            let surcharge = self.alpha * load + self.beta * load * load;
            cost += dist + surcharge;
            // Update load after visiting next node
            if tour[i + 1] == 0 {
                load = 0.0; // Intermediate depot visit: reset load
            } else {
                load += self.nodes[tour[i + 1]].demand as f64;
            }
        }

        // Return arc to depot
        let dist = self.distance(tour[tour.len() - 1], tour[0]);
        let surcharge = self.alpha * load + self.beta * load * load;
        cost += dist + surcharge;

        cost
    }
    
    /// Calculate tour cost with an additive load-dependent linear surcharge
    /// Arc cost c(i->j) = distance(i,j) + (alpha * |Wi|)
    /// where Wi is the load carried when leaving node i. The parameter
    /// `alpha` is the linear weight applied to the absolute load.
    pub fn tour_cost_linear_load(&self, tour: &[usize], alpha: f64) -> f64 {
        if tour.len() < 2 {
            return 0.0;
        }

        let mut cost = 0.0;
        
        // Vehicle starts with initial load (depot demands processed)
        let mut load = self.starting_load() as f64;

        for i in 0..tour.len() - 1 {
            let dist = self.distance(tour[i], tour[i + 1]);
            let surcharge = alpha * load.abs();
            cost += dist + surcharge;
            // Update load after visiting next node
            if tour[i + 1] == 0 {
                load = 0.0; // Intermediate depot visit: reset load
            } else {
                load += self.nodes[tour[i + 1]].demand as f64;
            }
        }

        // Return arc to depot
        let dist = self.distance(tour[tour.len() - 1], tour[0]);
        let surcharge = alpha * load.abs();
        cost += dist + surcharge;

        cost
    }
    
    /// Get statistics about the instance
    pub fn statistics(&self) -> InstanceStatistics {
        let num_pickups = self.pickup_nodes().iter().filter(|&&i| i != 0).count();
        let num_deliveries = self.delivery_nodes().iter().filter(|&&i| i != 0).count();
        let total_pickup: i32 = self.nodes.iter()
            .filter(|n| !n.is_depot() && n.is_pickup())
            .map(|n| n.demand)
            .sum();
        let total_delivery: i32 = self.nodes.iter()
            .filter(|n| !n.is_depot() && n.is_delivery())
            .map(|n| n.demand)
            .sum();
        
        
        let mut distances: Vec<f64> = Vec::new();
        for i in 0..self.dimension {
            for j in i+1..self.dimension {
                distances.push(self.distance(i, j));
            }
        }
        let avg_distance = distances.iter().sum::<f64>() / distances.len() as f64;
        let max_distance = distances.iter().cloned().fold(0.0, f64::max);
        
        let total_profit: i32 = self.nodes.iter().map(|n| n.profit).sum();

        InstanceStatistics {
            name: self.name.clone(),
            dimension: self.dimension,
            capacity: self.capacity,
            num_pickups,
            num_deliveries,
            total_pickup,
            total_delivery,
            total_profit,
            avg_distance,
            max_distance,
        }
    }
}

/// Statistics about a PD-TSP instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceStatistics {
    pub name: String,
    pub dimension: usize,
    pub capacity: i32,
    pub num_pickups: usize,
    pub num_deliveries: usize,
    pub total_pickup: i32,
    pub total_delivery: i32,
    pub total_profit: i32,
    pub avg_distance: f64,
    pub max_distance: f64,
}

impl std::fmt::Display for InstanceStatistics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Instance: {}", self.name)?;
        writeln!(f, "  Nodes: {} (1 depot + {} customers)", self.dimension, self.dimension - 1)?;
        writeln!(f, "  Capacity: {}", self.capacity)?;
        writeln!(f, "  Pickup nodes: {}", self.num_pickups)?;
        writeln!(f, "  Delivery nodes: {}", self.num_deliveries)?;
        writeln!(f, "  Total pickup load: {}", self.total_pickup)?;
        writeln!(f, "  Total delivery load: {}", self.total_delivery)?;
        writeln!(f, "  Total profit (nodes): {}", self.total_profit)?;
        writeln!(f, "  Avg distance: {:.2}", self.avg_distance)?;
        writeln!(f, "  Max distance: {:.2}", self.max_distance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_node_types() {
        let pickup = Node::new(1, 0.0, 0.0, 5, 0);  // positive = pickup
        let delivery = Node::new(2, 0.0, 0.0, -5, 0);  // negative = delivery
        let neutral = Node::new(3, 0.0, 0.0, 0, 0);
        
        assert!(pickup.is_pickup());
        assert!(!pickup.is_delivery());
        
        assert!(delivery.is_delivery());
        assert!(!delivery.is_pickup());
        
        assert!(!neutral.is_pickup());
        assert!(!neutral.is_delivery());
    }
    
    #[test]
    fn test_distance_calculation() {
        let nodes = vec![
            Node::new(0, 0.0, 0.0, 0, 0),
            Node::new(1, 3.0, 4.0, 0, 0),
        ];
        let matrix = PDTSPInstance::compute_distance_matrix(&nodes);
        
        assert!((matrix[0][1] - 5.0).abs() < 1e-10);
        assert!((matrix[1][0] - 5.0).abs() < 1e-10);
    }
}
