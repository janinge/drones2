use rand::Rng;
use std::time::Instant;

use crate::operators::mutate::roulette_wheel_tuned;
use crate::problem::Problem;
use crate::solution::Solution;
use crate::types::Cost;

use crate::metrics::IterationRecord;
use crate::search::progress::SearchProgress;

pub fn simulated_annealing(
    problem: &Problem,
    mut incumbent: Solution,
    max_iter: usize,
    warmup_iter: usize,
    final_temp: f32,
    mut iteration_data: Option<&mut Vec<IterationRecord>>,
) -> (Cost, Solution) {
    let mut thread_rng = rand::rng();

    let mut best_cost = incumbent.cost(problem);
    let mut best_solution = incumbent.clone();
    let mut incumbent_cost = best_cost;

    let mut delta_sum = 0.0;
    let mut delta_count = 0;
    
    // Initialize progress tracker
    let mut progress = SearchProgress::new();
    progress.update_incumbent_cost(incumbent_cost);

    // Warm-up
    for i in 0..warmup_iter {
        let start_time = Instant::now();
    
        let mut candidate = incumbent.clone();
        
        let (evaluations, infeasible) = roulette_wheel_tuned(&mut candidate, problem);

        let candidate_cost = candidate.cost(problem);
        let delta_e = candidate_cost - incumbent_cost;

        progress.record_candidate(i, &candidate);

        if delta_e < 0 {
            incumbent = candidate;
            incumbent_cost = candidate_cost;
            if incumbent_cost < best_cost {
                best_cost = incumbent_cost;
                best_solution = incumbent.clone();
                progress.update_best(i, best_solution.clone());
            }
        } else {
            if delta_e > 0 {
                delta_sum += delta_e as f32;
                delta_count += 1;
            }

            if thread_rng.random_bool(0.8) {
                incumbent = candidate;
                incumbent_cost = candidate_cost;
            }
        }

        if let Some(ref mut iter_data) = iteration_data {
            iter_data.push(IterationRecord {
                iteration: i,
                candidate_cost,
                candidate_seen: progress.candidate_seen(),
                incumbent_cost,
                best_cost,
                evaluations,
                infeasible,
                time: start_time.elapsed().as_secs_f64(),
                temperature: None
            });
        }
    }

    let delta_avg = if delta_count > 0 {
        delta_sum / delta_count as f32
    } else {
        1.0
    };

    // Initial temperature and cooling factor.
    let mut temp = -delta_avg / f32::ln(0.8);
    let alpha = (final_temp / temp).powf(1.0 / (max_iter.saturating_sub(warmup_iter) as f32));

    // Main annealing loop.
    for i in warmup_iter..max_iter {
        let start_time = Instant::now();

        let mut candidate = incumbent.clone();
        
        let (evaluations, infeasible) = roulette_wheel_tuned(&mut candidate, problem);

        let candidate_cost = candidate.cost(problem);
        let delta_e = candidate_cost - incumbent_cost;

        progress.record_candidate(i, &candidate);

        if delta_e < 0 {
            incumbent = candidate;
            incumbent_cost = candidate_cost;
            if incumbent_cost < best_cost {
                best_cost = incumbent_cost;
                best_solution = incumbent.clone();
                progress.update_best(i, best_solution.clone());
            }
        } else if thread_rng.random_bool(f32::exp(-delta_e as f32 / temp) as f64) {
            incumbent = candidate;
            incumbent_cost = candidate_cost;
        }
        temp *= alpha;

        if let Some(ref mut iter_data) = iteration_data {
            iter_data.push(IterationRecord {
                iteration: i,
                candidate_cost,
                candidate_seen: progress.candidate_seen(),
                incumbent_cost,
                best_cost,
                evaluations,
                infeasible,
                time: start_time.elapsed().as_secs_f64(),
                temperature: Some(temp)
            });
        }
    }

    (best_cost, best_solution)
}
