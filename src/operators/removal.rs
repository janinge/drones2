use std::cmp::{max, min};
use std::collections::HashMap;
use rand::{rng, Rng};
use rand::seq::index::sample;
use rand::seq::SliceRandom;

use crate::solution::Solution;
use crate::types::{CallId, Time};

use super::params::RemovalParams;

pub(crate) fn global_waiting(solution: &Solution, params: &RemovalParams) -> Vec<CallId> {
    // Register each CallId with its aggregated waiting time.
    let mut aggregated_waiting: HashMap<CallId, Time> = HashMap::with_capacity(solution.len());

    for route in solution.routes() {
        if let Some(simulation) = route.last_simulation() {
            let route_calls = route.route();

            // Don't try to iterate longer than over the shortest slice
            let count = min(route_calls.len(), simulation.waiting.len());

            for i in 0..count {
                let call = route_calls[i];
                let wait = simulation.waiting[i];

                if let Some(existing) = aggregated_waiting.get_mut(&call) {
                    if wait > 0 {
                        *existing += wait;
                    }
                } else {
                    // Only record a (pickup) call if its waiting time is positive.
                    aggregated_waiting.insert(call, if wait > 0 { wait } else { 0 });
                }
            }
        }
    }

    // Convert the hashmap into a vector and sort it by descending aggregated waiting time.
    let mut waiting_calls: Vec<(CallId, Time)> = aggregated_waiting.into_iter().collect();
    waiting_calls.sort_by(|a, b| b.1.cmp(&a.1));

    let total_calls = solution.len() as f32;

    let cut = min(max(min(params.max_removals, waiting_calls.len()), params.min_removals),
                  (params.selection_ratio * total_calls) as usize);

    waiting_calls
        .into_iter()
        .take(cut)
        .map(|(call, _)| call)
        .collect()
}

pub(crate) fn combined_cost(solution: &Solution, params: &RemovalParams) -> Vec<CallId> {
    let mut thread_rng = rng();
    let total_calls = solution.len() as f32;
    
    let num_unassigned = max(min(
        (params.selection_ratio * 0.5 * (1.0 - params.assignment_bias) * total_calls) as usize,
        params.max_removals
    ), 1);

    // Get calls from unassigned/dummy
    let mut unassigned_calls = random_unassigned(solution, num_unassigned);
    
    if unassigned_calls.len() == params.max_removals {
        return unassigned_calls;
    }
    
    let num_costly = max(
        (params.selection_ratio * 0.5 * params.assignment_bias * total_calls) as usize,
        params.min_removals
    );

    // Get most costly calls
    let mut costly_calls = global_cost(solution, num_costly);
    
    let mut combined = Vec::with_capacity(unassigned_calls.len() + costly_calls.len());
    combined.append(&mut unassigned_calls);
    combined.append(&mut costly_calls);
    
    combined.shuffle(&mut thread_rng);
    
    let cut = max(min(params.max_removals, combined.len()), params.min_removals);
    
    combined.into_iter().take(cut).collect()
}

pub(crate) fn global_cost(solution: &Solution, amount: usize) -> Vec<CallId> {
    let mut costs: Vec<(usize, _)> = solution
        .call_costs()
        .iter()
        .enumerate()
        .filter(|&(idx, _cost)| 
            solution.call_assignments()[idx].is_some()
        )
        .map(|(idx, cost)| (idx, cost.total))
        .collect();

    costs.sort_unstable_by_key(|&(_, total)| std::cmp::Reverse(total));

    costs
        .iter()
        .take(amount)
        .map(|&(idx, _)| CallId::try_from(idx + 1).unwrap())
        .collect()
}

pub(crate) fn broken_vehicle(solution: &Solution, _params: &RemovalParams) -> Vec<CallId> {
    let mut thread_rng = rand::rng();
    let vehicle_count = solution.routes().len();

    for _ in 0..vehicle_count {
        let vehicle_index = thread_rng.random_range(1..=vehicle_count);
        let route = solution.route(vehicle_index.try_into().unwrap());

        if !route.is_empty() {
            return route;
        }
    }

    Vec::new()
}

pub(crate) fn random_calls(solution: &Solution, amount: usize) -> Vec<CallId> {
    let n = solution.call_assignments().len();
    let mut thread_rng = rng();
    sample(&mut thread_rng, n, amount)
        .iter()
        .map(|idx| {
            (idx + 1)
                .try_into()
                .expect("Out of range value generated for CallId")
        })
        .collect::<Vec<CallId>>()
}

pub(crate) fn random_unassigned(solution: &Solution, amount: usize) -> Vec<CallId> {
    let unassigned_calls: Vec<usize> = solution
        .call_assignments()
        .iter()
        .enumerate()
        .filter_map(|(idx, assignment)| {
            if assignment.is_none() {
                Some(idx)
            } else {
                None
            }
        })
        .collect();

    let mut thread_rng = rng();
    let sample_size = amount.min(unassigned_calls.len());
    let sampled_indices = sample(&mut thread_rng, unassigned_calls.len(), sample_size);

    sampled_indices
        .iter()
        .map(|idx| 
            (unassigned_calls[idx] + 1)
                .try_into()
                .expect("Out of range value generated for CallId")
        )
        .collect()
}
