//! Custom heuristic: Profit density insertion

use crate::instance::PDTSPInstance;
use crate::solution::Solution;
use crate::heuristics::construction::ConstructionHeuristic;
use std::collections::HashSet;

/// ProfitDensity heuristic: selects next node by a profit/distance score
/// Aims to be robust under both linear and quadratic cost models.
pub struct ProfitDensityHeuristic {
    /// small epsilon to avoid division by zero
    pub eps: f64,
}

impl ProfitDensityHeuristic {
    pub fn new() -> Self {
        ProfitDensityHeuristic { eps: 1e-6 }
    }

    fn score(&self, instance: &PDTSPInstance, current: usize, candidate: usize, _current_load: i32) -> f64 {
        let dist = instance.distance(current, candidate);
        let profit = instance.nodes[candidate].profit as f64;

        // Higher profit and smaller distance -> better. We return lower-is-better score.
        // Use negative profit/distance so that sorting by score picks large profit/density.
        let density = if dist + self.eps > 0.0 { profit / (dist + self.eps) } else { profit / self.eps };
        // We want lower scores to be better, so invert density and add a small penalty for distance.
        -density + 0.001 * dist
    }
}

impl Default for ProfitDensityHeuristic {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstructionHeuristic for ProfitDensityHeuristic {
    fn construct(&self, instance: &PDTSPInstance) -> Solution {
        let start = std::time::Instant::now();

        let mut tour = vec![0];
        let mut visited: HashSet<usize> = HashSet::new();
        visited.insert(0);

        let mut current = 0usize;
        // Vehicle starts with initial load (depot demands processed)
        let mut current_load = instance.starting_load();

        while visited.len() < instance.dimension {
            let mut best = None;
            let mut best_score = f64::INFINITY;

            for candidate in 1..instance.dimension {
                if visited.contains(&candidate) { continue; }
                let new_load = current_load + instance.nodes[candidate].demand;
                if new_load < 0 || new_load > instance.capacity { continue; }

                let sc = self.score(instance, current, candidate, current_load);
                if sc < best_score {
                    best_score = sc;
                    best = Some(candidate);
                }
            }

            if let Some(next) = best {
                tour.push(next);
                visited.insert(next);
                current_load += instance.nodes[next].demand;
                current = next;
            } else {
                break;
            }
        }

        let mut sol = Solution::from_tour(instance, tour, self.name());
        sol.computation_time = start.elapsed().as_secs_f64();
        sol
    }

    fn name(&self) -> &str {
        "ProfitDensity"
    }
}
