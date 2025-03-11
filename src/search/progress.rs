use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use crate::solution::Solution;
use crate::types::Cost;

/// Tracks the progress and state of a metaheuristic search algorithm
#[derive(Debug)]
pub struct SearchProgress {
    /// Number of iterations performed so far
    pub iteration: usize,
    /// Incumbent solution cost
    pub incumbent_cost: Cost,
    /// Best solutions found so far
    pub best_solutions: Vec<Solution>,
    /// Iterations at which the best solutions was found
    pub best_iterations: Vec<usize>,
    /// Map tracking how many times each candidate solution has been encountered
    pub candidate_frequency: HashMap<u64, usize>,
    /// Hash of the current candidate solution
    pub candidate_hash: u64,
}

impl SearchProgress {
    pub fn new() -> Self {
        SearchProgress {
            iteration: 0,
            best_solutions: vec![],
            best_iterations: vec![],
            incumbent_cost: 0,
            candidate_frequency: HashMap::new(),
            candidate_hash: 0,
        }
    }
    
    pub fn record_candidate(&mut self, iteration: usize, solution: &Solution) {
        self.iteration = iteration;
        
        let mut hasher = DefaultHasher::new();
        solution.hash(&mut hasher);
        let hash = hasher.finish();
        
        self.candidate_hash = hash;
        
        *self.candidate_frequency.entry(hash).or_insert(0) += 1;
    }
    
    pub fn candidate_seen(&self) -> usize {
        *self.candidate_frequency.get(&self.candidate_hash).unwrap_or(&0)
    }
    
    pub fn update_best(&mut self, iteration: usize, best_solution: Solution) {
        self.best_solutions.push(best_solution);
        self.best_iterations.push(iteration);
    }
    
    pub fn update_incumbent_cost(&mut self, incumbent_cost: Cost) {
        self.incumbent_cost = incumbent_cost;
    }
}
