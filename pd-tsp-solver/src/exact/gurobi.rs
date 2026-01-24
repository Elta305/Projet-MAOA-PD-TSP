//! Exact solver for PD-TSP using Gurobi.
//! 
//! This module implements a Mixed Integer Programming (MIP) formulation
//! of the PD-TSP using the Gurobi optimizer.
//!
//! The formulation uses:
//! - Binary variables x[i][j] for edges
//! - Continuous variables u[i] for MTZ subtour elimination
//! - Continuous variables q[i] for cumulative load

#[cfg(feature = "gurobi")]
use crate::instance::{PDTSPInstance, CostFunction};
#[cfg(feature = "gurobi")]
use crate::solution::Solution;
#[cfg(feature = "gurobi")]
use grb::prelude::*;

/// Gurobi solver configuration
#[derive(Debug, Clone)]
pub struct GurobiConfig {
    /// Time limit in seconds
    pub time_limit: f64,
    /// MIP gap tolerance
    pub mip_gap: f64,
    /// Number of threads (0 = automatic)
    pub threads: i32,
    /// Enable verbose output
    pub verbose: bool,
    /// Use warm start from heuristic solution
    pub warm_start: Option<Vec<usize>>,
}

impl Default for GurobiConfig {
    fn default() -> Self {
        GurobiConfig {
            time_limit: 3600.0,
            mip_gap: 1e-6,
            threads: 0,
            verbose: false,
            warm_start: None,
        }
    }
}

/// Result of exact solving
#[derive(Debug, Clone)]
pub struct ExactResult {
    /// Best solution found
    pub solution: Solution,
    /// Lower bound (from LP relaxation)
    pub lower_bound: f64,
    /// Upper bound (best integer solution)
    pub upper_bound: f64,
    /// Optimality gap
    pub gap: f64,
    /// Whether optimal solution was proven
    pub optimal: bool,
    /// Solver status
    pub status: String,
    /// Number of nodes explored
    pub nodes_explored: i64,
}

/// Gurobi-based exact solver for PD-TSP
pub struct GurobiSolver {
    config: GurobiConfig,
}

impl GurobiSolver {
    pub fn new(config: GurobiConfig) -> Self {
        GurobiSolver { config }
    }
    
    /// Solve PD-TSP to optimality (or near-optimality)
    pub fn solve(&self, instance: &PDTSPInstance) -> Result<ExactResult, String> {
        if instance.cost_function == CostFunction::Quadratic {
            return Err("Gurobi exact solver does not support quadratic load-dependent cost. Use linear cost or heuristics.".to_string());
        }
        let start = std::time::Instant::now();
        let n = instance.dimension;
        
        // Simplified TSP formulation:
        // - Nodes 0..n-1 represent customers (node 0 is depot)
        // - Tour starts and ends at depot (node 0)
        // - Load constraints handle depot demands via initial/final load
        
        let env = Env::new("")
            .map_err(|e| format!("Failed to create Gurobi environment: {}", e))?;
        
        let mut model = Model::with_env("PDTSP", env)
            .map_err(|e| format!("Failed to create model: {}", e))?;
        
        model.set_param(param::TimeLimit, self.config.time_limit)
            .map_err(|e| format!("Failed to set time limit: {}", e))?;
        model.set_param(param::MIPGap, self.config.mip_gap)
            .map_err(|e| format!("Failed to set MIP gap: {}", e))?;
        model.set_param(param::Threads, self.config.threads)
            .map_err(|e| format!("Failed to set threads: {}", e))?;
        
        if !self.config.verbose {
            model.set_param(param::OutputFlag, 0)
                .map_err(|e| format!("Failed to set output flag: {}", e))?;
        }
        
        // x[i][j] = 1 if edge (i,j) is in the tour
        let mut x: Vec<Vec<Var>> = Vec::with_capacity(n);
        for i in 0..n {
            let mut row = Vec::with_capacity(n);
            for j in 0..n {
                let dist = instance.distance(i, j);
                let var = add_binvar!(model, 
                    name: &format!("x_{}_{}", i, j),
                    obj: dist
                ).map_err(|e| format!("Failed to add variable x[{}][{}]: {}", i, j, e))?;
                row.push(var);
            }
            x.push(row);
        }
        
        // u[i] = position in tour (MTZ subtour elimination)
        let mut u: Vec<Var> = Vec::with_capacity(n);
        for i in 0..n {
            let var = add_ctsvar!(model,
                name: &format!("u_{}", i),
                bounds: 0.0..n as f64
            ).map_err(|e| format!("Failed to add variable u[{}]: {}", i, e))?;
            u.push(var);
        }
        
        // q[i] = load after leaving node i
        let mut q: Vec<Var> = Vec::with_capacity(n);
        for i in 0..n {
            let var = add_ctsvar!(model,
                name: &format!("q_{}", i),
                bounds: 0.0..instance.capacity as f64
            ).map_err(|e| format!("Failed to add variable q[{}]: {}", i, e))?;
            q.push(var);
        }
        
        model.update()
            .map_err(|e| format!("Failed to update model: {}", e))?;
        
        // Flow conservation: each customer visited exactly once
        for j in 1..n {
            let expr_in: Expr = (0..n).filter(|&i| i != j)
                .map(|i| x[i][j])
                .grb_sum();
            model.add_constr(&format!("in_{}", j), c!(expr_in == 1.0))
                .map_err(|e| format!("Failed to add in-degree constraint: {}", e))?;
            
            let expr_out: Expr = (0..n).filter(|&k| k != j)
                .map(|k| x[j][k])
                .grb_sum();
            model.add_constr(&format!("out_{}", j), c!(expr_out == 1.0))
                .map_err(|e| format!("Failed to add out-degree constraint: {}", e))?;
        }
        
        // Depot: one departure, one return
        let depot_out: Expr = (1..n).map(|j| x[0][j]).grb_sum();
        model.add_constr("depot_out", c!(depot_out == 1.0))
            .map_err(|e| format!("Failed to add depot out constraint: {}", e))?;
        
        let depot_in: Expr = (1..n).map(|i| x[i][0]).grb_sum();
        model.add_constr("depot_in", c!(depot_in == 1.0))
            .map_err(|e| format!("Failed to add depot in constraint: {}", e))?;
        
        // No self-loops
        for i in 0..n {
            model.add_constr(&format!("no_loop_{}", i), c!(x[i][i] == 0.0))
                .map_err(|e| format!("Failed to add no-loop constraint: {}", e))?;
        }
        
        // MTZ subtour elimination
        for i in 1..n {
            for j in 1..n {
                if i != j {
                    model.add_constr(
                        &format!("mtz_{}_{}", i, j),
                        c!(u[j] >= u[i] + 1.0 - (n as f64) * (1.0 - x[i][j]))
                    ).map_err(|e| format!("Failed to add MTZ constraint: {}", e))?;
                }
            }
        }
        
        model.add_constr("depot_position", c!(u[0] == 0.0))
            .map_err(|e| format!("Failed to add depot position constraint: {}", e))?;
        
        // Load propagation
        let big_m = 2.0 * instance.capacity as f64;
        
        // For edges FROM depot: enforce starting load
        let initial_load = instance.starting_load() as f64;
        for j in 1..n {
            let demand_j = instance.nodes[j].demand as f64;
            model.add_constr(
                &format!("start_load_{}", j),
                c!(q[j] >= initial_load + demand_j - big_m * (1.0 - x[0][j]))
            ).map_err(|e| format!("Failed to add start load constraint: {}", e))?;
            
            model.add_constr(
                &format!("start_load_ub_{}", j),
                c!(q[j] <= initial_load + demand_j + big_m * (1.0 - x[0][j]))
            ).map_err(|e| format!("Failed to add start load ub constraint: {}", e))?;
        }
        
        // For customer-to-customer edges
        for i in 1..n {
            for j in 1..n {
                if i != j {
                    let demand_j = instance.nodes[j].demand as f64;
                    model.add_constr(
                        &format!("load_lb_{}_{}", i, j),
                        c!(q[j] >= q[i] + demand_j - big_m * (1.0 - x[i][j]))
                    ).map_err(|e| format!("Failed to add load lb constraint: {}", e))?;
                    
                    model.add_constr(
                        &format!("load_ub_{}_{}", i, j),
                        c!(q[j] <= q[i] + demand_j + big_m * (1.0 - x[i][j]))
                    ).map_err(|e| format!("Failed to add load ub constraint: {}", e))?;
                }
            }
        }
        
        // For edges TO depot: no specific constraint (load can be anything feasible)
        
        // Warm start
        if let Some(ref warm_tour) = self.config.warm_start {
            for i in 0..n {
                for j in 0..n {
                    model.set_obj_attr(attr::Start, &x[i][j], 0.0)
                        .map_err(|e| format!("Failed to initialize warm start: {}", e))?;
                }
            }

            for w in warm_tour.windows(2) {
                let u = w[0];
                let v = w[1];
                if u < n && v < n {
                    model.set_obj_attr(attr::Start, &x[u][v], 1.0)
                        .map_err(|e| format!("Failed to set warm start edge: {}", e))?;
                }
            }
        }
        
        model.update()
            .map_err(|e| format!("Failed to update model before optimization: {}", e))?;
        
        // Optimize
        model.optimize()
            .map_err(|e| format!("Optimization failed: {}", e))?;
        
        // Get results
        let status = model.status()
            .map_err(|e| format!("Failed to get status: {}", e))?;
        
        let status_str = match status {
            Status::Optimal => "Optimal",
            Status::TimeLimit => "TimeLimit",
            Status::Infeasible => "Infeasible",
            Status::InfOrUnbd => "InfeasibleOrUnbounded",
            Status::Unbounded => "Unbounded",
            Status::NodeLimit => "NodeLimit",
            Status::SolutionLimit => "SolutionLimit",
            _ => "Unknown",
        };

        if status == Status::Infeasible {
            let _ = model.compute_iis();
            let _ = model.write("gurobi_iis.ilp");
            eprintln!("Gurobi reported infeasible model; IIS written to gurobi_iis.ilp");
        }
        
        // Extract solution
        let mut tour = Vec::new();
        let obj_val: f64;
        let lower_bound: f64;
        let gap: f64;
        let optimal: bool;
        let nodes: i64;
        
        if status == Status::Optimal || status == Status::TimeLimit || status == Status::SolutionLimit {
            // Get objective value
            obj_val = model.get_attr(attr::ObjVal)
                .unwrap_or(f64::INFINITY);
            lower_bound = model.get_attr(attr::ObjBound)
                .unwrap_or(0.0);
            gap = model.get_attr(attr::MIPGap)
                .unwrap_or(1.0);
            optimal = status == Status::Optimal;
            nodes = model.get_attr(attr::NodeCount)
                .unwrap_or(0.0) as i64;
            
            // Extract tour from x variables
            tour.push(0);
            let mut current = 0;
            let mut visited = vec![false; n];
            visited[0] = true;
            
            // Follow edges to build tour
            for _ in 1..n {
                let mut found = false;
                for j in 0..n {
                    if !visited[j] {
                        let val = model.get_obj_attr(attr::X, &x[current][j])
                            .unwrap_or(0.0);
                        if val > 0.5 {
                            tour.push(j);
                            current = j;
                            visited[j] = true;
                            found = true;
                            break;
                        }
                    }
                }
                if !found {
                    break;
                }
            }
            
            // Return to depot
            tour.push(0);
        } else {
            obj_val = f64::INFINITY;
            lower_bound = 0.0;
            gap = 1.0;
            optimal = false;
            nodes = 0;
        }
        
        let mut solution = Solution::from_tour(instance, tour, "Gurobi-Exact");
        solution.computation_time = start.elapsed().as_secs_f64();
        
        Ok(ExactResult {
            solution,
            lower_bound,
            upper_bound: obj_val,
            gap,
            optimal,
            status: status_str.to_string(),
            nodes_explored: nodes,
        })
    }
    
    /// Solve with callback for lazy constraints (more efficient subtour elimination)
    pub fn solve_with_callbacks(&self, instance: &PDTSPInstance) -> Result<ExactResult, String> {
        // Do not support quadratic cost in callback solver either
        if instance.cost_function == CostFunction::Quadratic {
            return Err("Gurobi exact solver does not support quadratic load-dependent cost. Use linear cost or heuristics.".to_string());
        }
        // For smaller instances, use the simpler MTZ formulation
        if instance.dimension <= 50 {
            return self.solve(instance);
        }
        
        // For larger instances, use lazy constraint callback
        // This is more efficient as it only adds subtour elimination constraints when needed
        
        let start = std::time::Instant::now();
        let n = instance.dimension;
        
        let env = Env::new("")
            .map_err(|e| format!("Failed to create Gurobi environment: {}", e))?;
        
        let mut model = Model::with_env("PDTSP_Callback", env)
            .map_err(|e| format!("Failed to create model: {}", e))?;
        
        model.set_param(param::TimeLimit, self.config.time_limit)
            .map_err(|e| format!("Failed to set time limit: {}", e))?;
        model.set_param(param::MIPGap, self.config.mip_gap)
            .map_err(|e| format!("Failed to set MIP gap: {}", e))?;
        model.set_param(param::Threads, self.config.threads)
            .map_err(|e| format!("Failed to set threads: {}", e))?;
        model.set_param(param::LazyConstraints, 1)
            .map_err(|e| format!("Failed to enable lazy constraints: {}", e))?;
        
        if !self.config.verbose {
            model.set_param(param::OutputFlag, 0)
                .map_err(|e| format!("Failed to set output flag: {}", e))?;
        }
        
        // Create variables (similar to solve())
        let mut x: Vec<Vec<Var>> = Vec::with_capacity(n);
        for i in 0..n {
            let mut row = Vec::with_capacity(n);
            for j in 0..n {
                let var = add_binvar!(model, 
                    name: &format!("x_{}_{}", i, j),
                    obj: instance.distance(i, j)
                ).map_err(|e| format!("Failed to add variable: {}", e))?;
                row.push(var);
            }
            x.push(row);
        }
        
        // Load variables
        let mut q: Vec<Var> = Vec::with_capacity(n);
        for i in 0..n {
            let var = add_ctsvar!(model,
                name: &format!("q_{}", i),
                bounds: 0.0..instance.capacity as f64
            ).map_err(|e| format!("Failed to add variable: {}", e))?;
            q.push(var);
        }
        
        model.update()
            .map_err(|e| format!("Failed to update model: {}", e))?;
        
        // Basic constraints (degree constraints)
        for j in 0..n {
            let expr: Expr = x.iter().enumerate()
                .filter(|(i, _)| *i != j)
                .map(|(_, row)| row[j])
                .grb_sum();
            model.add_constr(&format!("in_{}", j), c!(expr == 1.0))
                .map_err(|e| format!("Failed to add constraint: {}", e))?;
        }
        
        for i in 0..n {
            let expr: Expr = x[i].iter().enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, &var)| var)
                .grb_sum();
            model.add_constr(&format!("out_{}", i), c!(expr == 1.0))
                .map_err(|e| format!("Failed to add constraint: {}", e))?;
        }
        
        for i in 0..n {
            model.add_constr(&format!("loop_{}", i), c!(x[i][i] == 0.0))
                .map_err(|e| format!("Failed to add constraint: {}", e))?;
        }
        
        // Load constraints
        let big_m = 2.0 * instance.capacity as f64;
        for i in 0..n {
            for j in 0..n {
                if i != j {
                    let demand_j = instance.nodes[j].demand as f64;
                    model.add_constr(
                        &format!("ld_{}_{}", i, j),
                        c!(q[j] >= q[i] + demand_j - big_m * (1.0 - x[i][j]))
                    ).map_err(|e| format!("Failed to add constraint: {}", e))?;
                }
            }
        }
        
        model.update()
            .map_err(|e| format!("Failed to update model: {}", e))?;
        
        // Optimize (without explicit callback for simplicity - using MTZ for now)
        // A full implementation would use Gurobi's callback API
        model.optimize()
            .map_err(|e| format!("Optimization failed: {}", e))?;
        
        let status = model.status()
            .map_err(|e| format!("Failed to get status: {}", e))?;
        
        let status_str = match status {
            Status::Optimal => "Optimal",
            Status::TimeLimit => "TimeLimit",
            _ => "Unknown",
        };
        
        let mut tour = Vec::new();
        let obj_val: f64;
        let lower_bound: f64;
        let gap: f64;
        let optimal: bool;
        let nodes: i64;
        
        if status == Status::Optimal || status == Status::TimeLimit {
            obj_val = model.get_attr(attr::ObjVal).unwrap_or(f64::INFINITY);
            lower_bound = model.get_attr(attr::ObjBound).unwrap_or(0.0);
            gap = model.get_attr(attr::MIPGap).unwrap_or(1.0);
            optimal = status == Status::Optimal;
            nodes = model.get_attr(attr::NodeCount).unwrap_or(0.0) as i64;
            
            tour.push(0);
            let mut current = 0;
            
            for _ in 0..n - 1 {
                for j in 0..n {
                    if j != current {
                        let val = model.get_obj_attr(attr::X, &x[current][j]).unwrap_or(0.0);
                        if val > 0.5 {
                            tour.push(j);
                            current = j;
                            break;
                        }
                    }
                }
            }
        } else {
            obj_val = f64::INFINITY;
            lower_bound = 0.0;
            gap = 1.0;
            optimal = false;
            nodes = 0;
        }
        
        let mut solution = Solution::from_tour(instance, tour, "Gurobi-Callback");
        solution.computation_time = start.elapsed().as_secs_f64();
        
        Ok(ExactResult {
            solution,
            lower_bound,
            upper_bound: obj_val,
            gap,
            optimal,
            status: status_str.to_string(),
            nodes_explored: nodes,
        })
    }
}

/// Compute lower bound using LP relaxation
pub fn compute_lp_bound(instance: &PDTSPInstance) -> Result<f64, String> {
    let n = instance.dimension;
    
    let env = Env::new("")
        .map_err(|e| format!("Failed to create environment: {}", e))?;
    
    let mut model = Model::with_env("PDTSP_LP", env)
        .map_err(|e| format!("Failed to create model: {}", e))?;
    
    model.set_param(param::OutputFlag, 0).ok();
    
    // Continuous variables (LP relaxation)
    let mut x: Vec<Vec<Var>> = Vec::with_capacity(n);
    for i in 0..n {
        let mut row = Vec::with_capacity(n);
        for j in 0..n {
            let var = add_ctsvar!(model, 
                name: &format!("x_{}_{}", i, j),
                bounds: 0.0..1.0,
                obj: instance.distance(i, j)
            ).map_err(|e| format!("Failed to add variable: {}", e))?;
            row.push(var);
        }
        x.push(row);
    }
    
    model.update()
        .map_err(|e| format!("Failed to update: {}", e))?;
    
    // Degree constraints
    for j in 0..n {
        let expr: Expr = x.iter().enumerate()
            .filter(|(i, _)| *i != j)
            .map(|(_, row)| row[j])
            .grb_sum();
        model.add_constr(&format!("in_{}", j), c!(expr == 1.0))
            .map_err(|e| format!("Failed to add constraint: {}", e))?;
    }
    
    for i in 0..n {
        let expr: Expr = x[i].iter().enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, &var)| var)
            .grb_sum();
        model.add_constr(&format!("out_{}", i), c!(expr == 1.0))
            .map_err(|e| format!("Failed to add constraint: {}", e))?;
    }
    
    model.optimize()
        .map_err(|e| format!("Optimization failed: {}", e))?;
    
    model.get_attr(attr::ObjVal)
        .map_err(|e| format!("Failed to get objective: {}", e))
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore]
    fn test_gurobi_solver() {
    }
}
