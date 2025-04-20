use rand::Rng;
use crate::problem::Problem;
use crate::solution::Solution;
use crate::types::{CallId, OperatorPair};

use crate::operators::mutate::PARAMS as REMOVAL_PARAMS;

/// A simple warm‐up pass to estimate an initial temperature T0
pub struct Warmup<'a> {
    operator_combinations: &'a [OperatorPair],
}

impl<'a> Warmup<'a> {
    pub fn new(operator_combinations: &'a [OperatorPair]) -> Self {
        Warmup { operator_combinations }
    }

    /// Runs max_iter random moves, collects positive ΔE samples,
    /// and returns T0 such that initial acceptance probability p0 is 0.8:
    ///   T0 = -avg(ΔE) / ln(p0)
    pub fn run(
        &self,
        problem: &Problem,
        mut incumbent: Solution,
        max_iter: usize,
        target_p0: f64,
    ) -> f32 {
        let mut incumbent_cost = incumbent.cost(problem);
        let mut thread_rng = rand::rng();

        // Accumulate "worsening" deltas
        let mut delta_sum: f32 = 0.0;
        let mut delta_count: usize = 0;

        for _ in 0..max_iter {
            let idx = thread_rng.random_range(0..self.operator_combinations.len());
            let (removal_op_fn, insertion_op_fn) = self.operator_combinations[idx];

            // Generate a candidate solution
            let mut candidate = incumbent.clone();
            let mut calls_to_remove = removal_op_fn(&candidate, &REMOVAL_PARAMS);
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
            let (_evals, _infeasible) =
                insertion_op_fn(&mut candidate, problem, calls_to_remove);

            // Compute cost difference
            let candidate_cost = candidate.cost(problem);
            let delta_e = candidate_cost - incumbent_cost;

            // Always accept improvements
            if delta_e < 0 {
                incumbent = candidate.clone();
                incumbent_cost = candidate_cost;
            }

            // Record and sometimes accept worsening
            if delta_e > 0 {
                delta_sum += delta_e as f32;
                delta_count += 1;
                
                if thread_rng.random_bool(target_p0) {
                    incumbent = candidate;
                    incumbent_cost = candidate_cost;
                }
            }
        }

        // Compute average ΔE and derive T0 for p0 = target_p0
        let avg_delta = if delta_count > 0 {
            delta_sum / delta_count as f32
        } else {
            1.0
        };
        let p0: f32 = target_p0 as f32;
        -avg_delta / p0.ln()
    }
}
