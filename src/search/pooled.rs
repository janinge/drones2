use crate::problem::Problem;
use crate::solution::Solution;
use crate::search::progress::SearchProgress;
use crate::types::{CallId, Cost, OperatorPair};

use rand::prelude::*;

use crate::operators::mutate::PARAMS as REMOVAL_PARAMS;

pub struct Pooled<'a> {
    operator_combinations: &'a [OperatorPair]
}

impl<'a> Pooled<'a> {
    pub fn new(
        operator_combinations: &'a [OperatorPair]
    ) -> Self {
        let n = operator_combinations.len();
        Pooled {
            operator_combinations
        }
    }

    pub fn run(
        &mut self,
        problem: &Problem,
        initial_solution: Solution,
        max_iter: usize,
        mut temp: f32,
        alpha: f32,
    ) -> (Cost, Solution) {
        let mut incumbent = initial_solution;
        let mut best_solution = incumbent.clone();
        let mut best_cost = incumbent.cost(problem);
        let mut incumbent_cost = best_cost;

        let mut stagnation_segments: usize = 0;
        let mut last_segment_best = best_cost;

        let mut delta_sum = 0.0;
        let mut delta_count = 0;

        let mut progress = SearchProgress::new();
        progress.update_incumbent_cost(incumbent_cost);

        let mut thread_rng = rand::rng();

        let mut segment_candidate_seen_total: usize = 0;

        for iteration in 0..max_iter {
            let mut candidate = incumbent.clone();

            let idx = thread_rng.random_range(0..self.operator_combinations.len());
            let (removal_op_fn, insertion_op_fn) = self.operator_combinations[idx];

            let mut calls_to_remove = removal_op_fn(&candidate, &REMOVAL_PARAMS);

            // If no calls were removed, we try to move unassigned calls
            if calls_to_remove.is_empty() {
                calls_to_remove = candidate
                    .call_assignments()
                    .iter()
                    .enumerate()
                    .filter_map(|(i, assignment)| {
                        if assignment.is_none() {
                            CallId::new_pickup((i + 1) as i16)
                        } else {
                            None
                        }
                    })
                    .collect();
            }

            let (evaluations, infeasible) = insertion_op_fn(&mut candidate, problem, calls_to_remove);

            let candidate_cost = candidate.cost(problem);
            let delta_e = candidate_cost - incumbent_cost;

            progress.record_candidate(iteration, &candidate);

            let seen = progress.candidate_seen();

            segment_candidate_seen_total += seen;
            
            if delta_e < 0 {
                // Improvement
                incumbent = candidate.clone();
                incumbent_cost = candidate_cost;

                if candidate_cost < best_cost {
                    // New best solution found
                    best_cost = candidate_cost;
                    best_solution = candidate;
                    progress.update_best(iteration, best_solution.clone());
                }
            } else {
                // Worsening:
                // Temperature based acceptance
                let acceptance_probability = (-(delta_e as f64) / (temp as f64)).exp();
                if thread_rng.random_bool(acceptance_probability) {
                    incumbent = candidate.clone();
                    incumbent_cost = candidate_cost;
                }
            }

            // Update temperature (cooling schedule)
            temp *= alpha;
        }

        (best_cost, best_solution)
    }
}
