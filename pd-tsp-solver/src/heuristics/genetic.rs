//! Genetic Algorithm for PD-TSP.
//! 
//! This module implements a sophisticated genetic algorithm with:
//! - Multiple crossover operators (OX, PMX, Edge Recombination)
//! - Adaptive mutation strategies
//! - Fitness-based selection with diversity preservation
//! - Local search integration (memetic algorithm)

use crate::instance::PDTSPInstance;
use crate::solution::Solution;
use crate::heuristics::construction::{
    ConstructionHeuristic,
    NearestNeighborHeuristic,
    GreedyInsertionHeuristic,
    SavingsHeuristic,
    SweepHeuristic,
    RegretInsertionHeuristic,
    ClusterFirstHeuristic,
    MultiStartConstruction,
};
use crate::heuristics::local_search::{LocalSearch, VND};
use crate::heuristics::profit_density::ProfitDensityHeuristic;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use ordered_float::OrderedFloat;
use std::collections::HashSet;

/// Individual in the genetic algorithm population
#[derive(Debug, Clone)]
pub struct Individual {
    /// The tour representation
    pub tour: Vec<usize>,
    /// Fitness (negative of tour cost, higher is better)
    pub fitness: f64,
    /// Whether the solution is feasible
    pub feasible: bool,
    /// Travel cost used in objective calculation
    pub travel_cost: f64,
    /// Total profit collected by this individual
    pub total_profit: i32,
}

impl Individual {
    pub fn new(tour: Vec<usize>, instance: &PDTSPInstance) -> Self {
        let travel_cost = instance.tour_cost(&tour);
        let total_profit = instance.tour_profit(&tour);
        let objective = total_profit as f64 - travel_cost;
        let feasible = instance.is_feasible(&tour);
        let fitness = if feasible { objective } else { objective - 1e9 }; // heavy penalty

        Individual {
            tour,
            fitness,
            feasible,
            travel_cost,
            total_profit,
        }
    }
    
    pub fn cost(&self) -> f64 {
        self.travel_cost
    }
}

/// Crossover operator types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CrossoverType {
    /// Order Crossover (OX)
    OrderCrossover,
    /// Partially Mapped Crossover (PMX)
    PMX,
    /// Edge Recombination Crossover
    EdgeRecombination,
    /// Cycle Crossover
    CycleCrossover,
}

/// Mutation operator types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MutationType {
    /// Swap two random nodes
    Swap,
    /// Reverse a random segment (2-opt move)
    Inversion,
    /// Move a random node to a random position
    Insertion,
    /// Swap two adjacent nodes
    Adjacent,
    /// Scramble a random segment
    Scramble,
}

/// Selection method types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionType {
    /// Tournament selection
    Tournament,
    /// Roulette wheel selection
    RouletteWheel,
    /// Rank-based selection
    RankBased,
}

/// Genetic Algorithm configuration
#[derive(Debug, Clone)]
pub struct GAConfig {
    /// Population size
    pub population_size: usize,
    /// Number of generations
    pub max_generations: usize,
    /// Maximum generations without improvement
    pub max_no_improve: usize,
    /// Crossover probability
    pub crossover_prob: f64,
    /// Mutation probability
    pub mutation_prob: f64,
    /// Elite count (best individuals preserved)
    pub elite_count: usize,
    /// Tournament size for selection
    pub tournament_size: usize,
    /// Crossover operator
    pub crossover_type: CrossoverType,
    /// Mutation operator
    pub mutation_type: MutationType,
    /// Selection method
    pub selection_type: SelectionType,
    /// Apply local search to offspring (memetic algorithm)
    pub use_local_search: bool,
    /// Local search probability
    pub local_search_prob: f64,
    /// Random seed
    pub seed: u64,
    /// Time limit in seconds for the GA run (optional)
    pub time_limit: f64,
    /// Adaptive mutation (increase when stuck)
    pub adaptive_mutation: bool,
}

impl Default for GAConfig {
    fn default() -> Self {
        GAConfig {
            population_size: 50,
            max_generations: 200,
            max_no_improve: 100,
            crossover_prob: 0.9,
            mutation_prob: 0.1,
            elite_count: 5,
            tournament_size: 5,
            crossover_type: CrossoverType::OrderCrossover,
            mutation_type: MutationType::Inversion,
            selection_type: SelectionType::Tournament,
            use_local_search: true,
            local_search_prob: 0.2,
            seed: 42,
            time_limit: 60.0,
            adaptive_mutation: true,
        }
    }
}

/// Genetic Algorithm implementation
pub struct GeneticAlgorithm {
    config: GAConfig,
    instance: PDTSPInstance,
    population: Vec<Individual>,
    best_individual: Option<Individual>,
    rng: ChaCha8Rng,
    generation: usize,
    no_improve_count: usize,
    current_mutation_prob: f64,
    time_limit: f64,
}

impl GeneticAlgorithm {
    pub fn new(instance: PDTSPInstance, config: GAConfig) -> Self {
        let rng = ChaCha8Rng::seed_from_u64(config.seed);
        let current_mutation_prob = config.mutation_prob;
        let time_limit = config.time_limit;

        GeneticAlgorithm {
            config,
            instance,
            population: Vec::new(),
            best_individual: None,
            rng,
            generation: 0,
            no_improve_count: 0,
            current_mutation_prob,
            time_limit,
        }
    }
    
    /// Initialize population using various construction heuristics
    fn initialize_population(&mut self) {
        self.population.clear();
        
        
        let constructions: Vec<Box<dyn ConstructionHeuristic + Send + Sync>> = vec![
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
            Box::new(ProfitDensityHeuristic::new()),
        ];

        
        for h in constructions.into_iter() {
            let sol = h.construct(&self.instance);

            // Normalize tour: some constructions return only customer sequence (no depot).
            let mut candidate = if sol.tour.len() == self.instance.dimension - 1 {
                Solution::from_tour(&self.instance, {
                    let mut t = sol.tour.clone();
                    t.insert(0, 0);
                    t
                }, "GA-init")
            } else if sol.tour.len() == self.instance.dimension {
                sol
            } else {
                continue;
            };

            if !candidate.feasible {
                let vnd = VND::with_standard_operators();
                vnd.improve(&self.instance, &mut candidate);
            }

            if candidate.tour.len() == self.instance.dimension && candidate.feasible {
                self.population.push(Individual::new(candidate.tour, &self.instance));
            }

            if self.population.len() >= self.config.population_size {
                break;
            }
        }
        
        
        for seed in 0..(self.config.population_size / 3).max(1) {
            let nn = NearestNeighborHeuristic::randomized(seed as u64 + self.config.seed);
            let sol = nn.construct(&self.instance);

            let mut candidate = if sol.tour.len() == self.instance.dimension - 1 {
                Solution::from_tour(&self.instance, {
                    let mut t = sol.tour.clone();
                    t.insert(0, 0);
                    t
                }, "GA-init")
            } else if sol.tour.len() == self.instance.dimension {
                sol
            } else {
                continue;
            };

            if !candidate.feasible {
                let vnd = VND::with_standard_operators();
                vnd.improve(&self.instance, &mut candidate);
            }
            if candidate.tour.len() == self.instance.dimension && candidate.feasible {
                self.population.push(Individual::new(candidate.tour, &self.instance));
            }
            if self.population.len() >= self.config.population_size {
                break;
            }
        }
        
        
        let mut attempts = 0;
        let max_attempts = self.config.population_size * 100; // Reduced from 1000
        
        while self.population.len() < self.config.population_size && attempts < max_attempts {
            let tour = self.generate_random_tour();
            let individual = Individual::new(tour, &self.instance);
            
            if individual.feasible {
                self.population.push(individual);
            } else if attempts > max_attempts / 4 {
                self.population.push(individual);
            }
            
            attempts += 1;
        }
        
        
        while self.population.len() < self.config.population_size / 2 {
            let tour = self.generate_random_tour();
            let individual = Individual::new(tour, &self.instance);
            self.population.push(individual);
        }
        
        
        self.population.sort_by_key(|ind| OrderedFloat(-ind.fitness));
        
        
        if let Some(best) = self.population.first() {
            self.best_individual = Some(best.clone());
        }

        
        let feasible_count = self.population.iter().filter(|i| i.feasible).count();
        let infeasible_count = self.population.len().saturating_sub(feasible_count);
        println!(
            "[GA] Initialized population: {} (feasible: {}, infeasible: {})",
            self.population.len(),
            feasible_count,
            infeasible_count
        );

        // If no feasible individuals were produced by the heuristics, attempt to
        // generate feasible solutions using a multi-start construction + local search
        if feasible_count == 0 {
            let mut attempts = 0;
            let max_attempts = self.config.population_size * 5;
            let multi = MultiStartConstruction::with_all_heuristics();

            while self.population.len() < self.config.population_size && attempts < max_attempts {
                    let sol = multi.construct(&self.instance);

                    let mut candidate = if sol.tour.len() == self.instance.dimension - 1 {
                        Solution::from_tour(&self.instance, {
                            let mut t = sol.tour.clone();
                            t.insert(0, 0);
                            t
                        }, "GA-fallback")
                    } else if sol.tour.len() == self.instance.dimension {
                        sol
                    } else {
                        attempts += 1;
                        continue;
                    };

                    if !candidate.feasible {
                        let vnd = VND::with_standard_operators();
                        vnd.improve(&self.instance, &mut candidate);
                    }

                    if candidate.tour.len() == self.instance.dimension {
                        self.population.push(Individual::new(candidate.tour, &self.instance));
                    }
                attempts += 1;
            }

            self.population.sort_by_key(|ind| OrderedFloat(-ind.fitness));
            if let Some(best) = self.population.first() {
                self.best_individual = Some(best.clone());
            }

            let feasible_count = self.population.iter().filter(|i| i.feasible).count();
            let infeasible_count = self.population.len().saturating_sub(feasible_count);
            println!(
                "[GA] After fallback initialization: {} (feasible: {}, infeasible: {})",
                self.population.len(),
                feasible_count,
                infeasible_count
            );
        }
    }
    
    /// Generate a random feasible tour
    fn generate_random_tour(&mut self) -> Vec<usize> {
        let n = self.instance.dimension;
        let mut tour: Vec<usize> = (1..n).collect();
        let mut attempts = 0;

        while attempts < 500 {
            tour.shuffle(&mut self.rng);
            let mut full_tour = tour.clone();
            full_tour.insert(0, 0);

            if self.instance.is_feasible(&full_tour) || attempts > 100 {
                return full_tour;
            }

            attempts += 1;
        }

        let nn = NearestNeighborHeuristic::new();
        let sol = nn.construct(&self.instance);
        sol.tour
    }
    
    /// Tournament selection
    fn tournament_select(&mut self) -> &Individual {
        let mut best_idx = self.rng.gen_range(0..self.population.len());
        
        for _ in 1..self.config.tournament_size {
            let idx = self.rng.gen_range(0..self.population.len());
            if self.population[idx].fitness > self.population[best_idx].fitness {
                best_idx = idx;
            }
        }
        
        &self.population[best_idx]
    }
    
    /// Roulette wheel selection
    fn roulette_select(&mut self) -> &Individual {
        let min_fitness = self.population.iter()
            .map(|i| i.fitness)
            .fold(f64::INFINITY, f64::min);
        
        let adjusted: Vec<f64> = self.population.iter()
            .map(|i| i.fitness - min_fitness + 1.0)
            .collect();
        
        let total: f64 = adjusted.iter().sum();
        let mut pick = self.rng.gen::<f64>() * total;
        
        for (i, &fitness) in adjusted.iter().enumerate() {
            pick -= fitness;
            if pick <= 0.0 {
                return &self.population[i];
            }
        }
        
        self.population.last().unwrap()
    }
    
    /// Rank-based selection
    fn rank_select(&mut self) -> &Individual {
        let n = self.population.len();
        let total_rank: usize = (n * (n + 1)) / 2;
        let pick = self.rng.gen_range(0..total_rank);
        
        let mut cumulative = 0;
        for (rank, individual) in self.population.iter().enumerate() {
            cumulative += n - rank; // Higher rank = higher weight
            if cumulative > pick {
                return individual;
            }
        }
        
        self.population.last().unwrap()
    }
    
    /// Select a parent using the configured method
    fn select_parent(&mut self) -> Individual {
        match self.config.selection_type {
            SelectionType::Tournament => self.tournament_select().clone(),
            SelectionType::RouletteWheel => self.roulette_select().clone(),
            SelectionType::RankBased => self.rank_select().clone(),
        }
    }
    
    /// Order Crossover (OX)
    fn order_crossover(&mut self, parent1: &[usize], parent2: &[usize]) -> Vec<usize> {
        let n = parent1.len();
        if n < 4 {
            return parent1.to_vec();
        }
        
        let start = self.rng.gen_range(1..n.saturating_sub(1).max(2));
        let end = self.rng.gen_range((start + 1)..(n.max(start + 2)));
        
        let mut child = vec![usize::MAX; n];
        child[0] = 0; // Keep depot
        
        for i in start..=end.min(n - 1) {
            child[i] = parent1[i];
        }
        
        let segment_set: HashSet<usize> = child[start..=end.min(n - 1)].iter().cloned().collect();
        let mut p2_iter = parent2.iter()
            .filter(|&&x| !segment_set.contains(&x) && x != 0)
            .cloned();
        
        for i in 1..n {
            if child[i] == usize::MAX {
                if let Some(val) = p2_iter.next() {
                    child[i] = val;
                }
            }
        }
        
        if child.contains(&usize::MAX) {
            return parent1.to_vec();
        }
        
        child
    }
    
    /// Partially Mapped Crossover (PMX)
    fn pmx_crossover(&mut self, parent1: &[usize], parent2: &[usize]) -> Vec<usize> {
        let n = parent1.len();
        if n < 4 {
            return parent1.to_vec();
        }
        
        let start = self.rng.gen_range(1..n.saturating_sub(1).max(2));
        let end = self.rng.gen_range((start + 1)..(n.max(start + 2)));
        
        let mut child = parent2.to_vec();
        
        let mut mapping = vec![usize::MAX; n];
        for i in start..=end.min(n - 1) {
            let p1_val = parent1[i];
            let p2_val = parent2[i];
            if p1_val < n && p2_val < n {
                mapping[p1_val] = p2_val;
            }
        }
        
        for i in start..=end.min(n - 1) {
            child[i] = parent1[i];
        }
        
        for i in (1..start).chain(end + 1..n) {
            let mut val = child[i];
            while mapping[val] != usize::MAX && mapping[val] != val {
                val = mapping[val];
            }
            child[i] = val;
        }
        
        
        let used: HashSet<usize> = child.iter().cloned().collect();
        let missing: Vec<usize> = (0..n).filter(|x| !used.contains(x)).collect();
        
        let mut missing_iter = missing.iter();
        for i in 1..n {
            if child.iter().take(i).any(|&x| x == child[i]) {
                if let Some(&val) = missing_iter.next() {
                    child[i] = val;
                }
            }
        }
        
        child[0] = 0;
        child
    }
    
    /// Edge Recombination Crossover
    fn edge_recombination(&mut self, parent1: &[usize], parent2: &[usize]) -> Vec<usize> {
        let n = parent1.len();
        
        
        let mut adj: Vec<HashSet<usize>> = vec![HashSet::new(); n];
        
        for parent in [parent1, parent2] {
            for i in 0..n {
                let prev = if i == 0 { n - 1 } else { i - 1 };
                let next = (i + 1) % n;
                adj[parent[i]].insert(parent[prev]);
                adj[parent[i]].insert(parent[next]);
            }
        }
        
        let mut child = Vec::with_capacity(n);
        let mut visited = vec![false; n];
        
        child.push(0);
        visited[0] = true;
        
        let mut current = 0;
        
        while child.len() < n {
            for list in &mut adj {
                list.remove(&current);
            }
            
            let neighbors: Vec<usize> = adj[current].iter()
                .filter(|&&x| !visited[x])
                .cloned()
                .collect();
            let next = if neighbors.is_empty() {
                (0..n)
                    .filter(|&x| !visited[x])
                    .min_by_key(|&x| adj[x].len())
                    .unwrap_or(0)
            } else {
                *neighbors.iter()
                    .min_by_key(|&&x| adj[x].len())
                    .unwrap()
            };
            
            child.push(next);
            visited[next] = true;
            current = next;
        }
        
        child
    }
    
    /// Cycle Crossover
    fn cycle_crossover(&mut self, parent1: &[usize], parent2: &[usize]) -> Vec<usize> {
        let n = parent1.len();
        let mut child = vec![usize::MAX; n];
        let mut in_cycle = vec![false; n];
        
        
        let mut pos_in_p2 = vec![0; n];
        for (i, &val) in parent2.iter().enumerate() {
            pos_in_p2[val] = i;
        }
        
        let mut cycle_num = 0;
        let mut start = 0;
        
        while child.iter().any(|&x| x == usize::MAX) {
            while start < n && in_cycle[start] {
                start += 1;
            }
            if start >= n {
                break;
            }
            
        
            let mut pos = start;
            loop {
                in_cycle[pos] = true;
                if cycle_num % 2 == 0 {
                    child[pos] = parent1[pos];
                } else {
                    child[pos] = parent2[pos];
                }
                pos = pos_in_p2[parent1[pos]];
                if pos == start {
                    break;
                }
            }
            
            cycle_num += 1;
        }
        
        child[0] = 0;
        child
    }
    
    /// Perform crossover using configured method
    fn crossover(&mut self, parent1: &Individual, parent2: &Individual) -> Individual {
        if self.rng.gen::<f64>() > self.config.crossover_prob {
            return parent1.clone();
        }
        
        let child_tour = match self.config.crossover_type {
            CrossoverType::OrderCrossover => self.order_crossover(&parent1.tour, &parent2.tour),
            CrossoverType::PMX => self.pmx_crossover(&parent1.tour, &parent2.tour),
            CrossoverType::EdgeRecombination => self.edge_recombination(&parent1.tour, &parent2.tour),
            CrossoverType::CycleCrossover => self.cycle_crossover(&parent1.tour, &parent2.tour),
        };
        
        Individual::new(child_tour, &self.instance)
    }
    
    /// Swap mutation
    fn mutate_swap(&mut self, tour: &mut Vec<usize>) {
        let n = tour.len();
        if n < 3 {
            return;
        }
        
        let i = self.rng.gen_range(1..n);
        let j = self.rng.gen_range(1..n);
        if i != j {
            tour.swap(i, j);
        }
    }
    
    /// Inversion mutation (2-opt)
    fn mutate_inversion(&mut self, tour: &mut Vec<usize>) {
        let n = tour.len();
        if n < 4 {
            return;
        }
        
        let i = self.rng.gen_range(1..n - 1);
        let j = self.rng.gen_range(i + 1..n);
        tour[i..=j].reverse();
    }
    
    /// Insertion mutation
    fn mutate_insertion(&mut self, tour: &mut Vec<usize>) {
        let n = tour.len();
        if n < 3 {
            return;
        }
        
        let from = self.rng.gen_range(1..n);
        let to = self.rng.gen_range(1..n);
        if from != to {
            let node = tour.remove(from);
            tour.insert(to, node);
        }
    }
    
    /// Adjacent swap mutation
    fn mutate_adjacent(&mut self, tour: &mut Vec<usize>) {
        let n = tour.len();
        if n < 3 {
            return;
        }
        
        let i = self.rng.gen_range(1..n - 1);
        tour.swap(i, i + 1);
    }
    
    /// Scramble mutation
    fn mutate_scramble(&mut self, tour: &mut Vec<usize>) {
        let n = tour.len();
        if n < 4 {
            return;
        }
        
        let start = self.rng.gen_range(1..n - 2);
        let end = self.rng.gen_range(start + 1..n);
        
        let mut segment: Vec<usize> = tour[start..=end].to_vec();
        segment.shuffle(&mut self.rng);
        tour[start..=end].copy_from_slice(&segment);
    }
    
    /// Perform mutation using configured method
    fn mutate(&mut self, individual: &mut Individual) {
        if self.rng.gen::<f64>() > self.current_mutation_prob {
            return;
        }
        
        let mut tour = individual.tour.clone();
        
        match self.config.mutation_type {
            MutationType::Swap => self.mutate_swap(&mut tour),
            MutationType::Inversion => self.mutate_inversion(&mut tour),
            MutationType::Insertion => self.mutate_insertion(&mut tour),
            MutationType::Adjacent => self.mutate_adjacent(&mut tour),
            MutationType::Scramble => self.mutate_scramble(&mut tour),
        }
        
        
        if tour[0] != 0 {
            if let Some(depot_pos) = tour.iter().position(|&x| x == 0) {
                tour.rotate_left(depot_pos);
            }
        }
        
        *individual = Individual::new(tour, &self.instance);
    }
    
    /// Apply local search to improve an individual
    fn apply_local_search(&self, individual: &mut Individual) {
        let vnd = VND::with_standard_operators();
        let mut solution = Solution::from_tour(&self.instance, individual.tour.clone(), "GA-LS");
        
        vnd.improve(&self.instance, &mut solution);
        
        *individual = Individual::new(solution.tour, &self.instance);
    }
    
    /// Create new generation
    fn evolve(&mut self) {
        let mut new_population = Vec::with_capacity(self.config.population_size);
        
        
        new_population.extend(
            self.population.iter()
                .take(self.config.elite_count)
                .cloned()
        );
        
        
        let mut attempts: usize = 0;
        let max_attempts: usize = (self.config.population_size).saturating_mul(50).max(500);

        while new_population.len() < self.config.population_size {
            let parent1 = self.select_parent();
            let parent2 = self.select_parent();

            let mut offspring = self.crossover(&parent1, &parent2);
            self.mutate(&mut offspring);

            
            if self.config.use_local_search && self.rng.gen::<f64>() < self.config.local_search_prob {
                self.apply_local_search(&mut offspring);
            }

            
            if offspring.feasible {
                new_population.push(offspring);
                
                attempts = 0; // reset attempts on success
            } else if new_population.len() < self.config.population_size.saturating_sub(10) {
                
                self.apply_local_search(&mut offspring);
                if offspring.feasible {
                    new_population.push(offspring);
                    
                    attempts = 0;
                } else {
                    attempts += 1;
                }
            } else {
                
                attempts += 1;

                if attempts > max_attempts {
                    
                    if let Some(best) = self.population.first().cloned().or_else(|| self.best_individual.clone()) {
                        println!("[GA] max_attempts exceeded ({}). Cloning best individual to fill population.", attempts);
                        while new_population.len() < self.config.population_size {
                            new_population.push(best.clone());
                        }
                    } else {
                        
                        println!("[GA] max_attempts exceeded but no best individual found; accepting infeasible offspring.");
                        new_population.push(offspring);
                    }
                    break;
                } else {
                    
                    if self.rng.gen::<f64>() < 0.05 {
                        println!("[GA] Accepting infeasible offspring to diversify (attempt {}).", attempts);
                        new_population.push(offspring);
                    }

                    
                    if attempts % 50 == 0 {
                        println!(
                            "[GA] evolve attempts={} new_population={}/{}",
                            attempts,
                            new_population.len(),
                            self.config.population_size
                        );
                    }
                }
            }
        }
        
        new_population.sort_by_key(|ind| OrderedFloat(-ind.fitness));
        
        if let Some(best) = new_population.first() {
            if let Some(ref current_best) = self.best_individual {
                if best.fitness > current_best.fitness {
                    self.best_individual = Some(best.clone());
                    self.no_improve_count = 0;
                } else {
                    self.no_improve_count += 1;
                }
            } else {
                self.best_individual = Some(best.clone());
            }
        }
        
        if self.config.adaptive_mutation {
            if self.no_improve_count > 10 {
                self.current_mutation_prob = (self.config.mutation_prob * 2.0).min(0.5);
            } else {
                self.current_mutation_prob = self.config.mutation_prob;
            }
        }
        
        self.population = new_population;
        self.generation += 1;
    }
    
    /// Run the genetic algorithm
    pub fn run(&mut self) -> Solution {
        let start = std::time::Instant::now();
        
        self.initialize_population();
        
        while self.generation < self.config.max_generations 
            && self.no_improve_count < self.config.max_no_improve 
            && start.elapsed().as_secs_f64() < self.time_limit
        {
            self.evolve();

            if let Some(ref best) = self.best_individual {
                println!(
                    "[GA] Gen {}  Best cost {:.3}  Feasible {}  Diversity {:.2}  Elapsed {:.2}s",
                    self.generation,
                    best.cost(),
                    best.feasible,
                    self.population_diversity(),
                    start.elapsed().as_secs_f64()
                );
            }
        }
        
        let best = self.best_individual.as_ref()
            .expect("No solution found");
        
        let mut solution = Solution::from_tour(&self.instance, best.tour.clone(), "GeneticAlgorithm");
        solution.computation_time = start.elapsed().as_secs_f64();
        solution.iterations = Some(self.generation);
        
        solution
    }
    
    /// Get current best solution
    pub fn best_solution(&self) -> Option<Solution> {
        self.best_individual.as_ref().map(|ind| {
            Solution::from_tour(&self.instance, ind.tour.clone(), "GeneticAlgorithm")
        })
    }
    
    /// Get current generation
    pub fn current_generation(&self) -> usize {
        self.generation
    }
    
    /// Get population diversity (average distance between individuals)
    pub fn population_diversity(&self) -> f64 {
        if self.population.len() < 2 {
            return 0.0;
        }
        
        let mut total_diff = 0.0;
        let mut count = 0;
        
        for i in 0..self.population.len().min(20) {
            for j in i + 1..self.population.len().min(20) {
                let diff = self.population[i].tour.iter()
                    .zip(self.population[j].tour.iter())
                    .filter(|(a, b)| a != b)
                    .count();
                total_diff += diff as f64;
                count += 1;
            }
        }
        
        if count > 0 {
            total_diff / count as f64
        } else {
            0.0
        }
    }
}

/// Memetic Algorithm (GA + Intensive Local Search)
pub struct MemeticAlgorithm {
    ga: GeneticAlgorithm,
}

impl MemeticAlgorithm {
    pub fn new(instance: PDTSPInstance) -> Self {
        let config = GAConfig {
            population_size: 50,
            max_generations: 200,
            max_no_improve: 50,
            crossover_prob: 0.8,
            mutation_prob: 0.15,
            elite_count: 3,
            use_local_search: true,
            local_search_prob: 0.5,
            ..Default::default()
        };
        
        MemeticAlgorithm {
            ga: GeneticAlgorithm::new(instance, config),
        }
    }
    
    pub fn with_config(instance: PDTSPInstance, config: GAConfig) -> Self {
        MemeticAlgorithm {
            ga: GeneticAlgorithm::new(instance, config),
        }
    }
    
    pub fn run(&mut self) -> Solution {
        let mut solution = self.ga.run();
        
        let vnd = VND::with_standard_operators();
        vnd.improve(&self.ga.instance, &mut solution);
        
        solution.algorithm = "MemeticAlgorithm".to_string();
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
            Node::new(4, 2.0, 1.0, 0, 0),
        ];
        
        let mut instance = PDTSPInstance {
            cost_function: CostFunction::Distance,
            alpha: 0.1,
            beta: 0.5,
            name: "test".to_string(),
            comment: "test".to_string(),
            dimension: 5,
            capacity: 10,
            nodes: nodes.clone(),
            distance_matrix: Vec::new(),
            return_depot_demand: 0,
        };
        
        instance.distance_matrix = vec![vec![0.0; 5]; 5];
        for i in 0..5 {
            for j in 0..5 {
                let dx = instance.nodes[i].x - instance.nodes[j].x;
                let dy = instance.nodes[i].y - instance.nodes[j].y;
                instance.distance_matrix[i][j] = (dx * dx + dy * dy).sqrt();
            }
        }
        
        instance
    }
    
    #[test]
    fn test_genetic_algorithm() {
        let instance = create_test_instance();
        let config = GAConfig {
            population_size: 20,
            max_generations: 10,
            ..Default::default()
        };
        
        let mut ga = GeneticAlgorithm::new(instance, config);
        let solution = ga.run();
        
        assert!(solution.feasible);
        assert_eq!(solution.tour.len(), 5);
    }
}
