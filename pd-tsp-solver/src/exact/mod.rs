//! Exact solvers module.

// When built with the `gurobi` feature, expose the real implementation
#[cfg(feature = "gurobi")]
mod gurobi;
#[cfg(feature = "gurobi")]
pub use gurobi::*;

// Otherwise provide a lightweight stub so the rest of the codebase can compile
#[cfg(not(feature = "gurobi"))]
mod gurobi_stub {
	use crate::instance::PDTSPInstance;
	use crate::solution::Solution;

	#[derive(Debug, Clone)]
	pub struct GurobiConfig {
		pub time_limit: f64,
		pub mip_gap: f64,
		pub threads: i32,
		pub verbose: bool,
		pub warm_start: Option<Vec<usize>>,
	}

	impl Default for GurobiConfig {
		fn default() -> Self {
			GurobiConfig { time_limit: 3600.0, mip_gap: 1e-6, threads: 0, verbose: false, warm_start: None }
		}
	}

	#[derive(Debug, Clone)]
	pub struct ExactResult {
		pub solution: Solution,
		pub lower_bound: f64,
		pub upper_bound: f64,
		pub gap: f64,
		pub optimal: bool,
		pub status: String,
		pub nodes_explored: i64,
	}

	pub struct GurobiSolver { pub config: GurobiConfig }

	impl GurobiSolver {
		pub fn new(config: GurobiConfig) -> Self { GurobiSolver { config } }
		pub fn solve(&self, _instance: &PDTSPInstance) -> Result<ExactResult, String> {
			Err("Gurobi feature not enabled in this build".to_string())
		}
	}
}

#[cfg(not(feature = "gurobi"))]
pub use gurobi_stub::*;
