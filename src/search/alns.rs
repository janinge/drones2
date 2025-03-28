use crate::problem::Problem;
use crate::solution::Solution;
use crate::operators::params::RemovalParams;
use crate::operators::mutate::PARAMS as REMOVAL_PARAMS;
use crate::metrics::IterationRecord;
use crate::search::progress::SearchProgress;
use crate::types::{CallId, Cost, OperatorPair};

use rand::distr::weighted::WeightedIndex;
use rand::prelude::*;
use std::time::Instant;
use crate::operators::removal::random_calls;

#[derive(Copy, Clone)]
pub struct ScoreParams {
    pub improvement: f32,
    pub best: f32,
    pub novelty: f32,
}

pub struct ALNS<'a> {
    operator_combinations: &'a [OperatorPair],
    weights: Vec<f32>,
    usage: Vec<Vec<u32>>,
    scores: Vec<Vec<f32>>,
    rho: f32,
    segment_length: usize,
    score_params: ScoreParams,
    final_temp: f32,
    alpha: Option<f32>,
    removal_params: RemovalParams,
}

impl<'a> ALNS<'a> {
    pub fn new(
        operator_combinations: &'a [OperatorPair],
        rho: f32,
        segment_length: usize,
        score_params: ScoreParams,
        final_temp: f32
    ) -> Self {
        let n = operator_combinations.len();
        ALNS {
            operator_combinations,
            weights: vec![1.0; n],
            usage: vec![vec![0; segment_length]; n],
            scores: vec![vec![0.0; segment_length]; n],
            rho,
            segment_length,
            score_params,
            final_temp,
            alpha: None,
            removal_params: REMOVAL_PARAMS
        }
    }

    pub fn run(
        &mut self,
        problem: &Problem,
        initial_solution: Solution,
        max_iter: usize,
        mut iteration_data: Option<&mut Vec<IterationRecord>>,
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
        let mut temp: f32 = 0.0;

        let mut segment_candidate_seen_total: usize = 0;

        for iteration in 0..max_iter {
            let start_time = Instant::now();

            let mut candidate = incumbent.clone();

            let dist = WeightedIndex::new(&self.weights).unwrap();
            let idx = dist.sample(&mut thread_rng);
            let (removal_op_fn, insertion_op_fn) = self.operator_combinations[idx];

            let mut calls_to_remove = removal_op_fn(&candidate, &self.removal_params);

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

            // Update usage counts
            let segment = iteration % self.segment_length;
            self.usage[idx][segment] += 1;

            let seen = progress.candidate_seen();

            segment_candidate_seen_total += seen;

            if seen <= 1 {
                self.scores[idx][segment] += self.score_params.novelty;
            }

            if delta_e < 0 {
                // Improvement
                self.scores[idx][segment] += self.score_params.improvement;

                incumbent = candidate.clone();
                incumbent_cost = candidate_cost;

                if candidate_cost < best_cost {
                    // New best solution found
                    self.scores[idx][segment] += self.score_params.best;

                    best_cost = candidate_cost;
                    best_solution = candidate;
                    progress.update_best(iteration, best_solution.clone());
                }
            } else {
                if delta_e > 0 {
                    delta_sum += delta_e as f32;
                    delta_count += 1;
                }

                // Warm-up:
                if self.alpha.is_none() {
                    // Accept with a fixed probability (e.g., 0.8)
                    if thread_rng.random_bool(0.8) {
                        incumbent = candidate.clone();
                        incumbent_cost = candidate_cost;
                    }
                } else {
                    // Otherwise, temperature based acceptance
                    let acceptance_probability = f32::exp(-delta_e as f32 / temp) as f64;
                    if thread_rng.random_bool(acceptance_probability) {
                        incumbent = candidate.clone();
                        incumbent_cost = candidate_cost;
                    }
                }
            }

            if (iteration + 1) % self.segment_length == 0 {
                self.update_weights();

                // Reset segment scores and usage for next segment
                for op_idx in 0..self.weights.len() {
                    self.scores[op_idx].fill(0.0);
                    self.usage[op_idx].fill(0);
                }

                // Check for stagnation
                if best_cost >= last_segment_best {
                    stagnation_segments += 1;
                } else {
                    stagnation_segments = 0;
                }

                let seen_too_much = segment_candidate_seen_total >= 3 * self.segment_length;

                if stagnation_segments >= 6 || seen_too_much {
                    // Reset temperature based on average worsening cost difference
                    let delta_avg = if delta_count > 0 {
                        delta_sum / delta_count as f32
                    } else {
                        1.0
                    };

                    // Recompute alpha and temperature
                    self.alpha = None;

                    // Remove a percentage of calls from all vehicles
                    let removal_fraction: f32 = 0.5;
                    let num_remove = ((incumbent.call_assignments().len() as f32) * removal_fraction).ceil() as usize;
                    let removal_list = random_calls(&incumbent, num_remove);

                    for call in removal_list {
                        let _ = incumbent.remove_call(call);
                    }
                    incumbent_cost = incumbent.cost(problem);

                    stagnation_segments = 0;
                }

                last_segment_best = best_cost;
                segment_candidate_seen_total = 0;
                
                // Warm up phase or escaping
                if self.alpha.is_none() {
                    let delta_avg = if delta_count > 0 { delta_sum / delta_count as f32 } else { 1.0 };
                    let initial_temp = -delta_avg / f32::ln(0.8);
                    let remaining_iter = max_iter - (iteration + 1);
                    let computed_alpha = (self.final_temp / initial_temp).powf(1.0 / (remaining_iter as f32));
                    self.alpha = Some(computed_alpha);
                    temp = initial_temp;
                }
            }

            // Update temperature (cooling schedule)
            if let Some(alpha) = self.alpha {
                temp *= alpha;
            }

            if let Some(ref mut iter_data) = iteration_data {
                iter_data.push(IterationRecord {
                    iteration,
                    candidate_cost,
                    candidate_seen: progress.candidate_seen(),
                    incumbent_cost,
                    best_cost,
                    evaluations,
                    infeasible,
                    time: start_time.elapsed().as_secs_f64(),
                    temperature: if self.alpha.is_some() { Some(temp) } else { None },
                });
            }
        }

        (best_cost, best_solution)
    }

    fn update_weights(&mut self) {
        for i in 0..self.weights.len() {
            let total_usage: u32 = self.usage[i].iter().sum();
            if total_usage > 0 {
                let average_score: f32 = self.scores[i].iter().sum::<f32>() / total_usage as f32;
                self.weights[i] = (self.weights[i] * (1.0 - self.rho) + self.rho * average_score).max(0.1);
            }
            // If an operator wasn't used in this segment, leave its weight unchanged
        }
    }
}
