use crate::instance::PDTSPInstance;
use crate::solution::Solution;
use ordered_float::OrderedFloat;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;

pub trait ConstructionHeuristic {
    fn construct(&self, instance: &PDTSPInstance) -> Solution;
    fn name(&self) -> &str;
}

 

/// Capacity-aware Nearest Neighbor Heuristic
/// 
/// Builds a tour by repeatedly visiting the nearest unvisited node
/// that doesn't violate capacity constraints.
pub struct NearestNeighborHeuristic {
    pub randomized: bool,
    pub seed: u64,
}

impl NearestNeighborHeuristic {
    pub fn new() -> Self {
        NearestNeighborHeuristic {
            randomized: false,
            seed: 42,
        }
    }
    
    pub fn randomized(seed: u64) -> Self {
        NearestNeighborHeuristic {
            randomized: true,
            seed,
        }
    }
    
    fn can_add_node(&self, instance: &PDTSPInstance, current_load: i32, node: usize) -> bool {
        let new_load = current_load + instance.nodes[node].demand;
        new_load >= 0 && new_load <= instance.capacity
    }
    
    fn find_nearest(&self, 
        instance: &PDTSPInstance, 
        current: usize, 
        visited: &HashSet<usize>,
        current_load: i32,
        rng: &mut ChaCha8Rng
    ) -> Option<usize> {
        let mut candidates: Vec<(usize, f64)> = (0..instance.dimension)
            .filter(|&n| !visited.contains(&n))
            .filter(|&n| self.can_add_node(instance, current_load, n))
            .map(|n| (n, instance.distance(current, n)))
            .collect();
        
        if candidates.is_empty() {
            return None;
        }
        
        candidates.sort_by_key(|&(_, d)| OrderedFloat(d));
        
        if self.randomized && candidates.len() > 1 {
            
            let top_k = candidates.len().min(3);
            let idx = rng.gen_range(0..top_k);
            Some(candidates[idx].0)
        } else {
            Some(candidates[0].0)
        }
    }
}

impl Default for NearestNeighborHeuristic {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstructionHeuristic for NearestNeighborHeuristic {
    fn construct(&self, instance: &PDTSPInstance) -> Solution {
        let start = std::time::Instant::now();
        let mut rng = ChaCha8Rng::seed_from_u64(self.seed);
        
        let mut tour = vec![0]; // Start at depot
        let mut visited = HashSet::new();
        visited.insert(0);
        
        let mut current = 0;
        // Vehicle loads initial cargo and processes depot demand
        let mut current_load = instance.starting_load();
        
        while visited.len() < instance.dimension {
            if let Some(next) = self.find_nearest(instance, current, &visited, current_load, &mut rng) {
                tour.push(next);
                visited.insert(next);
                current_load += instance.nodes[next].demand;
                current = next;
            } else {
                break;
            }
        }
        
        // println!("[Savings-debug] visited={} tour_len={} dimension={} is_feasible={}",
            // visited.len(), tour.len(), instance.dimension, instance.is_feasible(&tour));

        // If some nodes couldn't be added feasibly, they remain unvisited.
        // We don't force infeasible insertions - the solution may be partial.
        let mut solution = Solution::from_tour(instance, tour, self.name());
        solution.computation_time = start.elapsed().as_secs_f64();
        solution
    }
    
    fn name(&self) -> &str {
        if self.randomized {
            "NearestNeighbor-Randomized"
        } else {
            "NearestNeighbor"
        }
    }
}

 

/// Greedy Insertion Heuristic
/// 
/// Starts with a partial tour and repeatedly inserts the node
/// that causes the minimum increase in tour length.
pub struct GreedyInsertionHeuristic {
    pub farthest_insertion: bool,
}

impl GreedyInsertionHeuristic {
    pub fn new() -> Self {
        GreedyInsertionHeuristic {
            farthest_insertion: false,
        }
    }
    
    pub fn farthest() -> Self {
        GreedyInsertionHeuristic {
            farthest_insertion: true,
        }
    }
    
    /// Calculate insertion cost for a node at a position
    fn insertion_cost(&self, instance: &PDTSPInstance, tour: &[usize], node: usize, pos: usize) -> f64 {
        let prev = tour[pos];
        let next = tour[(pos + 1) % tour.len()];
        
        instance.distance(prev, node) + instance.distance(node, next) - instance.distance(prev, next)
    }
    
    /// Check if inserting node at position pos maintains feasibility
    /// Simulates the tour with the new node inserted and checks capacity constraints
    fn is_feasible_insertion(&self, instance: &PDTSPInstance, tour: &[usize], node: usize, pos: usize) -> bool {
        // Build the tour with the node inserted
        let mut test_tour = tour.to_vec();
        test_tour.insert(pos + 1, node);
        
        // Check partial feasibility (load stays in [0, capacity] throughout)
        instance.is_partial_feasible(&test_tour)
    }
    
    /// Find best insertion for a node
    fn find_best_insertion(&self, instance: &PDTSPInstance, tour: &[usize], node: usize) -> Option<(usize, f64)> {
        let mut best_pos = None;
        let mut best_cost = f64::INFINITY;
        
        for pos in 0..tour.len() {
            if self.is_feasible_insertion(instance, tour, node, pos) {
                let cost = self.insertion_cost(instance, tour, node, pos);
                if cost < best_cost {
                    best_cost = cost;
                    best_pos = Some(pos);
                }
            }
        }
        
        best_pos.map(|p| (p, best_cost))
    }
}

impl Default for GreedyInsertionHeuristic {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstructionHeuristic for GreedyInsertionHeuristic {
    fn construct(&self, instance: &PDTSPInstance) -> Solution {
        let start = std::time::Instant::now();
        
        
        let mut tour = vec![0];
        let mut unvisited: HashSet<usize> = (1..instance.dimension).collect();
        
        
        let initial = if self.farthest_insertion {
            *unvisited.iter()
                .max_by_key(|&&n| OrderedFloat(instance.distance(0, n)))
                .unwrap()
        } else {
            *unvisited.iter()
                .min_by_key(|&&n| OrderedFloat(instance.distance(0, n)))
                .unwrap()
        };
        
        tour.push(initial);
        unvisited.remove(&initial);
        
        while !unvisited.is_empty() {
            let mut best_node = None;
            let mut best_pos = 0;
            let mut best_cost = f64::INFINITY;
            
            for &node in &unvisited {
                if let Some((pos, cost)) = self.find_best_insertion(instance, &tour, node) {
                    let selection_cost = if self.farthest_insertion {
                        -tour.iter().map(|&t| instance.distance(t, node)).fold(f64::INFINITY, f64::min)
                    } else {
                        cost
                    };
                    
                    if selection_cost < best_cost {
                        best_cost = selection_cost;
                        best_node = Some(node);
                        best_pos = pos;
                    }
                }
            }
            
            if let Some(node) = best_node {
                tour.insert(best_pos + 1, node);
                unvisited.remove(&node);
            } else {
                break;
            }
        }

        // If some nodes couldn't be inserted feasibly, they remain unvisited.
        // We don't force infeasible insertions - the solution may be partial.
        let mut solution = Solution::from_tour(instance, tour, self.name());
        solution.computation_time = start.elapsed().as_secs_f64();
        solution
    }
    
    fn name(&self) -> &str {
        if self.farthest_insertion {
            "FarthestInsertion"
        } else {
            "GreedyInsertion"
        }
    }
}

 

/// Clarke-Wright Savings Algorithm adapted for PD-TSP
/// 
/// Computes savings for merging routes and applies them while
/// respecting capacity constraints.
pub struct SavingsHeuristic {
    /// Shape parameter for savings calculation
    pub lambda: f64,
}

impl SavingsHeuristic {
    pub fn new() -> Self {
        SavingsHeuristic { lambda: 1.0 }
    }
    
    pub fn with_lambda(lambda: f64) -> Self {
        SavingsHeuristic { lambda }
    }
    
    /// Calculate savings for merging two nodes
    fn savings(&self, instance: &PDTSPInstance, i: usize, j: usize) -> f64 {
        instance.distance(i, 0) + instance.distance(0, j) 
            - self.lambda * instance.distance(i, j)
    }
}

impl Default for SavingsHeuristic {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstructionHeuristic for SavingsHeuristic {
    fn construct(&self, instance: &PDTSPInstance) -> Solution {
        let start = std::time::Instant::now();
        
        
        let mut savings: Vec<(usize, usize, f64)> = Vec::new();
        for i in 1..instance.dimension {
            for j in i + 1..instance.dimension {
                let s = self.savings(instance, i, j);
                savings.push((i, j, s));
            }
        }
        
        
        savings.sort_by(|a, b| OrderedFloat(b.2).cmp(&OrderedFloat(a.2)));
        
        
        let mut tour = vec![0];
        let mut visited = HashSet::new();
        visited.insert(0);
        
        
        if let Some(&(i, j, _)) = savings.first() {
            tour.push(i);
            tour.push(j);
            visited.insert(i);
            visited.insert(j);
        }
        
        
        for &(i, j, _) in &savings {
            if visited.len() >= instance.dimension {
                break;
            }
            
            let i_in = visited.contains(&i);
            let j_in = visited.contains(&j);
            
            if i_in && !j_in {
                
                if let Some(pos) = tour.iter().position(|&x| x == i) {
                    let test_tour: Vec<usize> = tour[..=pos].iter()
                        .chain(std::iter::once(&j))
                        .chain(tour[pos + 1..].iter())
                        .cloned()
                        .collect();
                    
                    if instance.is_partial_feasible(&test_tour) {
                        tour.insert(pos + 1, j);
                        visited.insert(j);
                    }
                }
            } else if !i_in && j_in {
                
                if let Some(pos) = tour.iter().position(|&x| x == j) {
                    let insert_pos = if pos > 0 { pos } else { 1 };
                    let test_tour: Vec<usize> = tour[..insert_pos].iter()
                        .chain(std::iter::once(&i))
                        .chain(tour[insert_pos..].iter())
                        .cloned()
                        .collect();
                    
                    if instance.is_partial_feasible(&test_tour) {
                        tour.insert(insert_pos, i);
                        visited.insert(i);
                    }
                }
            }
        }
        
        
        let greedy_helper = GreedyInsertionHeuristic::new();
        let mut still_unvisited: Vec<usize> = Vec::new();
        for n in 1..instance.dimension {
            if !visited.contains(&n) {
                if let Some((pos, _cost)) = greedy_helper.find_best_insertion(instance, &tour, n) {
                    tour.insert(pos + 1, n); // find_best_insertion returns `pos` as insertion index before node at pos+1
                    visited.insert(n);
                } else {
                    still_unvisited.push(n);
                }
            }
        }

        
        for n in still_unvisited.iter().cloned() {
            let mut best_pos = None;
            let mut best_cost = f64::INFINITY;
            for pos in 1..=tour.len() {
                let mut test_tour = tour.clone();
                test_tour.insert(pos, n);
                if instance.is_partial_feasible(&test_tour) {
                    let cost = instance.tour_length(&test_tour);
                    if cost < best_cost {
                        best_cost = cost;
                        best_pos = Some(pos);
                    }
                }
            }
            if let Some(pos) = best_pos {
                tour.insert(pos, n);
                visited.insert(n);
            }
        }
        
        let mut solution = Solution::from_tour(instance, tour, self.name());
        solution.computation_time = start.elapsed().as_secs_f64();

        
        if !solution.feasible || solution.tour.len() < instance.dimension {
            // Fallbacks removed: return the constructed solution as-is (may be infeasible)
            solution.computation_time = start.elapsed().as_secs_f64();
            return solution;
        }

        solution
    }
    
    fn name(&self) -> &str {
        "Savings-ClarkeWright"
    }
}

 

/// Sweep Algorithm
/// 
/// Sorts nodes by polar angle from depot and constructs a tour
/// following this order while respecting capacity.
pub struct SweepHeuristic {
    /// Starting angle for the sweep
    pub start_angle: f64,
}

impl SweepHeuristic {
    pub fn new() -> Self {
        SweepHeuristic { start_angle: 0.0 }
    }
    
    pub fn with_start_angle(angle: f64) -> Self {
        SweepHeuristic { start_angle: angle }
    }
    
    /// Calculate polar angle from depot to node
    fn polar_angle(&self, instance: &PDTSPInstance, node: usize) -> f64 {
        let dx = instance.nodes[node].x - instance.nodes[0].x;
        let dy = instance.nodes[node].y - instance.nodes[0].y;
        let angle = dy.atan2(dx);
        
        
        let normalized = angle - self.start_angle;
        if normalized < 0.0 {
            normalized + 2.0 * std::f64::consts::PI
        } else {
            normalized
        }
    }
}

impl Default for SweepHeuristic {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstructionHeuristic for SweepHeuristic {
    fn construct(&self, instance: &PDTSPInstance) -> Solution {
        let start = std::time::Instant::now();
        
        
        let mut nodes: Vec<usize> = (1..instance.dimension).collect();
        nodes.sort_by_key(|&n| OrderedFloat(self.polar_angle(instance, n)));
        
        
        let mut tour = vec![0];
        // Vehicle loads initial cargo and processes depot demand
        let mut current_load = instance.starting_load();
        let mut remaining: Vec<usize> = Vec::new();
        
        for node in nodes {
            let new_load = current_load + instance.nodes[node].demand;
            if new_load >= 0 && new_load <= instance.capacity {
                tour.push(node);
                current_load = new_load;
            } else {
                remaining.push(node);
            }
        }
        
        
        for node in remaining {
            let mut inserted = false;
            for pos in 1..=tour.len() {
                let mut test_tour = tour.clone();
                test_tour.insert(pos, node);
                
                if instance.is_feasible(&test_tour) {
                    tour.insert(pos, node);
                    inserted = true;
                    break;
                }
            }
            
            if !inserted {
                
                tour.push(node);
            }
        }
        
        let mut solution = Solution::from_tour(instance, tour, self.name());
        solution.computation_time = start.elapsed().as_secs_f64();
        solution
    }
    
    fn name(&self) -> &str {
        "Sweep"
    }
}

 

/// Regret-k Insertion Heuristic
/// 
/// Selects the node with maximum regret (difference between best
/// and k-th best insertion cost) for insertion.
pub struct RegretInsertionHeuristic {
    /// Number of positions to consider for regret calculation
    pub k: usize,
}

impl RegretInsertionHeuristic {
    pub fn new(k: usize) -> Self {
        RegretInsertionHeuristic { k: k.max(2) }
    }
    
    /// Calculate regret for inserting a node
    fn calculate_regret(&self, instance: &PDTSPInstance, tour: &[usize], node: usize) -> (f64, usize) {
        let mut costs: Vec<(usize, f64)> = Vec::new();
        
        for pos in 0..tour.len() {
            let prev = tour[pos];
            let next = tour[(pos + 1) % tour.len()];
            let cost = instance.distance(prev, node) + instance.distance(node, next) 
                - instance.distance(prev, next);
            
            // Vehicle loads initial cargo at depot then processes depot demand
            let mut load = instance.starting_load();
            let mut feasible = true;
            for (i, &n) in tour.iter().enumerate().skip(1) {
                if i == pos + 1 {
                    load += instance.nodes[node].demand;
                    if load < 0 || load > instance.capacity {
                        feasible = false;
                        break;
                    }
                }
                load += instance.nodes[n].demand;
                if load < 0 || load > instance.capacity {
                    feasible = false;
                    break;
                }
            }
            
            // Check if inserting at end is feasible
            if feasible && pos + 1 >= tour.len() {
                let test_load = load + instance.nodes[node].demand;
                if test_load < 0 || test_load > instance.capacity {
                    feasible = false;
                }
            }
            
            if feasible {
                costs.push((pos, cost));
            }
        }
        
        if costs.is_empty() {
            return (f64::NEG_INFINITY, 0);
        }
        
        costs.sort_by_key(|&(_, c)| OrderedFloat(c));
        
        let best_cost = costs[0].1;
        let best_pos = costs[0].0;
        
        
        let regret = if costs.len() >= self.k {
            costs[self.k - 1].1 - best_cost
        } else if costs.len() > 1 {
            costs.last().unwrap().1 - best_cost
        } else {
            0.0
        };
        
        (regret, best_pos)
    }
}

impl ConstructionHeuristic for RegretInsertionHeuristic {
    fn construct(&self, instance: &PDTSPInstance) -> Solution {
        let start = std::time::Instant::now();
        
        
        let mut tour = vec![0];
        let mut unvisited: HashSet<usize> = (1..instance.dimension).collect();
        
        let farthest = *unvisited.iter()
            .max_by_key(|&&n| OrderedFloat(instance.distance(0, n)))
            .unwrap();
        tour.push(farthest);
        unvisited.remove(&farthest);
        
        let max_iterations = instance.dimension * 2;
        let mut iterations = 0;
        
        while !unvisited.is_empty() && iterations < max_iterations {
            iterations += 1;
            
            let mut best_node = None;
            let mut best_pos = 0;
            let mut max_regret = f64::NEG_INFINITY;
            
            for &node in &unvisited {
                let (regret, pos) = self.calculate_regret(instance, &tour, node);
                if regret > max_regret {
                    max_regret = regret;
                    best_node = Some(node);
                    best_pos = pos;
                }
            }
            
            if let Some(node) = best_node {
                tour.insert(best_pos + 1, node);
                unvisited.remove(&node);
            } else {
                break;
            }
        }
        
        let mut solution = Solution::from_tour(instance, tour, self.name());
        solution.computation_time = start.elapsed().as_secs_f64();
        solution
    }
    
    fn name(&self) -> &str {
        match self.k {
            2 => "Regret-2",
            3 => "Regret-3",
            _ => "Regret-k",
        }
    }
}

/// Deliver-Earliest Heuristic
///
/// Prioritizes delivery nodes (negative demand) early in the tour to reduce carried load.
pub struct DeliverEarliestHeuristic {
    pub seed: u64,
}

impl DeliverEarliestHeuristic {
    pub fn new() -> Self { DeliverEarliestHeuristic { seed: 42 } }
    pub fn with_seed(seed: u64) -> Self { DeliverEarliestHeuristic { seed } }
}

impl ConstructionHeuristic for DeliverEarliestHeuristic {
    fn construct(&self, instance: &PDTSPInstance) -> Solution {
        let start = std::time::Instant::now();
        let mut tour = vec![0];
        let mut visited: std::collections::HashSet<usize> = std::collections::HashSet::new();
        visited.insert(0);
        let mut current = 0usize;
        let mut load = instance.nodes[0].demand;

        while visited.len() < instance.dimension {
            // prefer feasible delivery nodes (demand < 0) closest to current
            let mut candidates: Vec<(usize, f64)> = (1..instance.dimension)
                .filter(|&n| !visited.contains(&n))
                .filter(|&n| {
                    let nl = load + instance.nodes[n].demand;
                    nl >= 0 && nl <= instance.capacity
                })
                .map(|n| (n, instance.distance(current, n)))
                .collect();

            if candidates.is_empty() {
                break;
            }

            candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            // find first delivery among top candidates
            let mut chosen = None;
            for &(n, _d) in &candidates {
                if instance.nodes[n].demand < 0 {
                    chosen = Some(n);
                    break;
                }
            }

            if chosen.is_none() {
                chosen = Some(candidates[0].0);
            }

            let next = chosen.unwrap();
            tour.push(next);
            visited.insert(next);
            load += instance.nodes[next].demand;
            current = next;
        }

        let mut solution = Solution::from_tour(instance, tour, self.name());
        solution.computation_time = start.elapsed().as_secs_f64();
        solution
    }

    fn name(&self) -> &str { "DeliverEarliest" }
}

/// Pickup-HighestProfit-First Heuristic
///
/// Chooses next pickup nodes by highest profit-to-distance ratio.
pub struct PickupHighProfitHeuristic {
    pub seed: u64,
}

impl PickupHighProfitHeuristic {
    pub fn new() -> Self { PickupHighProfitHeuristic { seed: 42 } }
    pub fn with_seed(seed: u64) -> Self { PickupHighProfitHeuristic { seed } }
}

impl ConstructionHeuristic for PickupHighProfitHeuristic {
    fn construct(&self, instance: &PDTSPInstance) -> Solution {
        let start = std::time::Instant::now();
        let mut tour = vec![0];
        let mut visited: std::collections::HashSet<usize> = std::collections::HashSet::new();
        visited.insert(0);
        let mut current = 0usize;
        let mut load = instance.nodes[0].demand;

        while visited.len() < instance.dimension {
            let mut candidates: Vec<(usize, f64)> = (1..instance.dimension)
                .filter(|&n| !visited.contains(&n))
                .filter(|&n| {
                    let nl = load + instance.nodes[n].demand;
                    nl >= 0 && nl <= instance.capacity
                })
                .map(|n| {
                    let dist = instance.distance(current, n);
                    let profit = instance.nodes[n].profit.max(1) as f64;
                    let score = profit / (1.0 + dist);
                    (n, score)
                })
                .collect();

            if candidates.is_empty() { break; }

            candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            let next = candidates[0].0;
            tour.push(next);
            visited.insert(next);
            load += instance.nodes[next].demand;
            current = next;
        }

        let mut solution = Solution::from_tour(instance, tour, self.name());
        solution.computation_time = start.elapsed().as_secs_f64();
        solution
    }

    fn name(&self) -> &str { "PickupHighProfit" }
}

 

/// Cluster-First Route-Second Heuristic
/// 
/// First clusters nodes based on proximity and demand balance,
/// then optimizes the visiting order within constraints.
pub struct ClusterFirstHeuristic {
    /// Number of clusters
    pub num_clusters: usize,
}

impl ClusterFirstHeuristic {
    pub fn new() -> Self {
        ClusterFirstHeuristic { num_clusters: 4 }
    }
    
    pub fn with_clusters(num_clusters: usize) -> Self {
        ClusterFirstHeuristic { num_clusters }
    }
    
    /// Simple k-means clustering
    fn cluster_nodes(&self, instance: &PDTSPInstance) -> Vec<Vec<usize>> {
        let n = instance.dimension - 1; // Exclude depot
        let k = self.num_clusters.min(n);
        
        
        let mut centroids: Vec<(f64, f64)> = Vec::new();
        let step = n / k;
        for i in 0..k {
            let node_idx = 1 + i * step;
            centroids.push((instance.nodes[node_idx].x, instance.nodes[node_idx].y));
        }
        
        let mut clusters = vec![Vec::new(); k];
        
        
        for i in 1..instance.dimension {
            let mut min_dist = f64::INFINITY;
            let mut best_cluster = 0;
            
            for (c, &(cx, cy)) in centroids.iter().enumerate() {
                let dx = instance.nodes[i].x - cx;
                let dy = instance.nodes[i].y - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                
                if dist < min_dist {
                    min_dist = dist;
                    best_cluster = c;
                }
            }
            
            clusters[best_cluster].push(i);
        }
        
        
        for (c, cluster) in clusters.iter().enumerate() {
            if !cluster.is_empty() {
                let sum_x: f64 = cluster.iter().map(|&n| instance.nodes[n].x).sum();
                let sum_y: f64 = cluster.iter().map(|&n| instance.nodes[n].y).sum();
                centroids[c] = (sum_x / cluster.len() as f64, sum_y / cluster.len() as f64);
            }
        }
        
        
        clusters = vec![Vec::new(); k];
        for i in 1..instance.dimension {
            let mut min_dist = f64::INFINITY;
            let mut best_cluster = 0;
            
            for (c, &(cx, cy)) in centroids.iter().enumerate() {
                let dx = instance.nodes[i].x - cx;
                let dy = instance.nodes[i].y - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                
                if dist < min_dist {
                    min_dist = dist;
                    best_cluster = c;
                }
            }
            
            clusters[best_cluster].push(i);
        }
        
        clusters
    }
    
    /// Order nodes within a cluster by angle from cluster centroid
    fn order_cluster(&self, instance: &PDTSPInstance, cluster: &[usize]) -> Vec<usize> {
        if cluster.is_empty() {
            return Vec::new();
        }
        
        let cx: f64 = cluster.iter().map(|&n| instance.nodes[n].x).sum::<f64>() / cluster.len() as f64;
        let cy: f64 = cluster.iter().map(|&n| instance.nodes[n].y).sum::<f64>() / cluster.len() as f64;
        
        let mut ordered = cluster.to_vec();
        ordered.sort_by_key(|&n| {
            let dx = instance.nodes[n].x - cx;
            let dy = instance.nodes[n].y - cy;
            OrderedFloat(dy.atan2(dx))
        });
        
        ordered
    }
}

impl Default for ClusterFirstHeuristic {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstructionHeuristic for ClusterFirstHeuristic {
    fn construct(&self, instance: &PDTSPInstance) -> Solution {
        let start = std::time::Instant::now();
        
        let clusters = self.cluster_nodes(instance);
        
        
        let mut cluster_order: Vec<(usize, f64)> = clusters.iter()
            .enumerate()
            .filter(|(_, c)| !c.is_empty())
            .map(|(i, c)| {
                let cx: f64 = c.iter().map(|&n| instance.nodes[n].x).sum::<f64>() / c.len() as f64;
                let cy: f64 = c.iter().map(|&n| instance.nodes[n].y).sum::<f64>() / c.len() as f64;
                (i, cy.atan2(cx))
            })
            .collect();
        cluster_order.sort_by_key(|&(_, angle)| OrderedFloat(angle));
        
        
        let mut tour = vec![0];
        for (cluster_idx, _) in cluster_order {
            let ordered = self.order_cluster(instance, &clusters[cluster_idx]);
            tour.extend(ordered);
        }
        
        
        if !instance.is_feasible(&tour) {
            
            let nodes: Vec<usize> = tour[1..].to_vec();
            tour = vec![0];
            let greedy_helper = GreedyInsertionHeuristic::new();

            for node in nodes {
                let mut inserted = false;
                
                if let Some((pos, _cost)) = greedy_helper.find_best_insertion(instance, &tour, node) {
                    tour.insert(pos + 1, node);
                    inserted = true;
                } else {
                    
                    for pos in 1..=tour.len() {
                        let mut test_tour = tour.clone();
                        test_tour.insert(pos, node);
                        if instance.is_partial_feasible(&test_tour) {
                            tour.insert(pos, node);
                            inserted = true;
                            break;
                        }
                    }
                }

                if !inserted {
                    // As a last resort, insert at the cheapest position (ignoring feasibility)
                    let mut best_pos_any: Option<usize> = None;
                    let mut best_cost_any = f64::INFINITY;
                    for pos in 1..=tour.len() {
                        let mut test_tour = tour.clone();
                        test_tour.insert(pos, node);
                        let cost = instance.tour_length(&test_tour);
                        if cost < best_cost_any {
                            best_cost_any = cost;
                            best_pos_any = Some(pos);
                        }
                    }
                    if let Some(p) = best_pos_any {
                        tour.insert(p, node);
                    }
                }
            }

            
            if !instance.is_feasible(&tour) || tour.len() < instance.dimension {
                // Ensure all nodes are present by inserting any missing ones at cheapest positions
                let mut tour2 = tour.clone();
                let missing: Vec<usize> = (1..instance.dimension).filter(|n| !tour2.contains(n)).collect();
                for n in missing.iter().cloned() {
                    let mut best_pos = None;
                    let mut best_cost = f64::INFINITY;
                    for pos in 1..=tour2.len() {
                        let mut test_tour = tour2.clone();
                        test_tour.insert(pos, n);
                        let cost = instance.tour_length(&test_tour);
                        if cost < best_cost {
                            best_cost = cost;
                            best_pos = Some(pos);
                        }
                    }
                    if let Some(pos) = best_pos {
                        tour2.insert(pos, n);
                    } else {
                        tour2.push(n);
                    }
                }

                let mut solution = Solution::from_tour(instance, tour2, self.name());
                solution.computation_time = start.elapsed().as_secs_f64();
                return solution;
            }
        }
        
        let mut solution = Solution::from_tour(instance, tour, self.name());
        solution.computation_time = start.elapsed().as_secs_f64();
        solution
    }
    
    fn name(&self) -> &str {
        "ClusterFirst"
    }
}

 

/// Multi-Start Construction
/// 
/// Runs multiple construction heuristics and returns the best result.
pub struct MultiStartConstruction {
    heuristics: Vec<Box<dyn ConstructionHeuristic + Send + Sync>>,
}

impl MultiStartConstruction {
    pub fn new() -> Self {
        MultiStartConstruction {
            heuristics: Vec::new(),
        }
    }
    
    pub fn with_all_heuristics() -> Self {
        let heuristics: Vec<Box<dyn ConstructionHeuristic + Send + Sync>> = vec![
            Box::new(NearestNeighborHeuristic::new()),
            Box::new(NearestNeighborHeuristic::randomized(1)),
            Box::new(NearestNeighborHeuristic::randomized(2)),
            Box::new(NearestNeighborHeuristic::randomized(3)),
            Box::new(GreedyInsertionHeuristic::new()),
            Box::new(GreedyInsertionHeuristic::farthest()),
            Box::new(SavingsHeuristic::new()),
            Box::new(SavingsHeuristic::with_lambda(0.8)),
            Box::new(SavingsHeuristic::with_lambda(1.2)),
            Box::new(SweepHeuristic::new()),
            Box::new(SweepHeuristic::with_start_angle(std::f64::consts::PI / 4.0)),
            Box::new(SweepHeuristic::with_start_angle(std::f64::consts::PI / 2.0)),
            Box::new(RegretInsertionHeuristic::new(2)),
            Box::new(RegretInsertionHeuristic::new(3)),
            Box::new(ClusterFirstHeuristic::new()),
            Box::new(ClusterFirstHeuristic::with_clusters(3)),
            Box::new(ClusterFirstHeuristic::with_clusters(5)),
            Box::new(DeliverEarliestHeuristic::new()),
            Box::new(PickupHighProfitHeuristic::new()),
        ];
        
        MultiStartConstruction { heuristics }
    }
    
    pub fn add_heuristic<H: ConstructionHeuristic + Send + Sync + 'static>(&mut self, h: H) {
        self.heuristics.push(Box::new(h));
    }
}

impl Default for MultiStartConstruction {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstructionHeuristic for MultiStartConstruction {
    fn construct(&self, instance: &PDTSPInstance) -> Solution {
        let start = std::time::Instant::now();
        
        let mut best_solution = Solution::new();
        
        for heuristic in &self.heuristics {
            let solution = heuristic.construct(instance);

            // Ignore trivial depot-only solutions; prefer non-trivial feasible starts
            if solution.feasible && solution.cost < best_solution.cost && solution.tour.len() > 1 {
                best_solution = solution;
            }
        }

        
        if best_solution.tour.is_empty() {
            for heuristic in &self.heuristics {
                let solution = heuristic.construct(instance);
                if !solution.tour.is_empty() && solution.tour.len() > 1 {
                    best_solution = solution;
                    break;
                }
            }
        }

        
        if best_solution.tour.is_empty() {
            let mut tour: Vec<usize> = (0..instance.nodes.len()).collect();
            
            if !tour.is_empty() && tour[0] != 0 {
                if let Some(pos0) = tour.iter().position(|&x| x == 0) {
                    tour.swap(0, pos0);
                }
            }
            best_solution = Solution::from_tour(instance, tour, self.name());
        }

        best_solution.algorithm = self.name().to_string();
        best_solution.computation_time = start.elapsed().as_secs_f64();
        // If best_solution misses nodes, insert missing nodes at cheapest positions
        if best_solution.tour.len() < instance.dimension {
            let mut tour2 = best_solution.tour.clone();
            let missing: Vec<usize> = (1..instance.dimension).filter(|n| !tour2.contains(n)).collect();
            for n in missing {
                let mut best_pos = None;
                let mut best_cost = f64::INFINITY;
                for pos in 1..=tour2.len() {
                    let mut test_tour = tour2.clone();
                    test_tour.insert(pos, n);
                    let cost = instance.tour_length(&test_tour);
                    if cost < best_cost {
                        best_cost = cost;
                        best_pos = Some(pos);
                    }
                }
                if let Some(pos) = best_pos {
                    tour2.insert(pos, n);
                } else {
                    tour2.push(n);
                }
            }
            best_solution = Solution::from_tour(instance, tour2, self.name());
        }

        best_solution
    }
    
    fn name(&self) -> &str {
        "MultiStart"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_instance() -> PDTSPInstance {
        use crate::instance::CostFunction;
        
        let nodes = vec![
            crate::instance::Node::new(0, 0.0, 0.0, 0, 0),
            crate::instance::Node::new(1, 1.0, 0.0, 5, 0),
            crate::instance::Node::new(2, 0.0, 1.0, -5, 0),
            crate::instance::Node::new(3, 1.0, 1.0, 0, 0),
        ];
        
        let mut instance = PDTSPInstance {
            cost_function: CostFunction::Distance,
            alpha: 0.1,
            beta: 0.5,
            name: "test".to_string(),
            comment: "test instance".to_string(),
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
    fn test_nearest_neighbor() {
        let instance = create_test_instance();
        let heuristic = NearestNeighborHeuristic::new();
        let solution = heuristic.construct(&instance);
        
        assert_eq!(solution.tour.len(), 4);
        assert_eq!(solution.tour[0], 0);
    }
    
    #[test]
    fn test_greedy_insertion() {
        let instance = create_test_instance();
        let heuristic = GreedyInsertionHeuristic::new();
        let solution = heuristic.construct(&instance);
        
        assert_eq!(solution.tour.len(), 4);
    }
}
