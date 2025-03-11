use rand::prelude::*;
use rand::rng;

use crate::problem::Problem;
use crate::solution::Solution;
use crate::types::CallId;

pub fn random_placement(solution: &mut Solution, problem: &Problem, calls: Vec<CallId>) -> (usize, usize) {
    let mut thread_rng = rng();

    let (mut evaluated, mut infeasible) = (0, 0);

    let first_call = *calls.first().unwrap();

    let inserted = 'placement: {
        for call in calls {
            let (removed_vehicle, removed_pickup, removed_delivery) = match solution.remove_call(call) {
                Ok((vehicle, pickup, delivery)) => (Some(vehicle), pickup, delivery),
                Err(_) => { (None, None, None) }
            };

            let available_vehicles = problem.get_compatible_vehicles(call).to_vec();

            let mut shuffled_vehicles = available_vehicles.to_vec();
            shuffled_vehicles.shuffle(&mut thread_rng);

            for vehicle in shuffled_vehicles {
                // Get capacity feasibility information
                let (_call_weight, capacity_result) =
                    solution.find_spare_capacity_in_vehicle(problem, call, vehicle);

                if capacity_result.is_some() {
                    let capacity_result_clone = capacity_result.clone();

                    // Collect all feasible insertion points using our iterator
                    let mut feasible_insertions: Vec<(usize, usize)> = solution
                        .get_feasible_insertions(problem, call, vehicle, &capacity_result_clone)
                        .collect();

                    let combinations = feasible_insertions.len();
                    let start_eval = evaluated;

                    feasible_insertions.shuffle(&mut thread_rng);

                    for (pickup_idx, delivery_idx) in feasible_insertions {
                        evaluated += 1;

                        if matches!(
                            (removed_vehicle, removed_pickup, removed_delivery),
                            (Some(removed_vehicle), Some(pickup), Some(delivery))
                            if removed_vehicle == vehicle && pickup == pickup_idx && delivery == delivery_idx) {
                            continue;
                        }

                        if solution.insert_call(vehicle, call, pickup_idx, delivery_idx).is_ok() {
                            if solution.feasible(problem).is_err() {
                                solution.remove_call(call).unwrap();
                                infeasible += 1;
                            } else {
                                break 'placement true;
                            }
                        }
                    }
                }
            }

            // Reinsert this call into the vehicle it was removed from, before moving on to the next call.
            if let (Some(removed_vehicle), Some(pickup), Some(delivery)) = (removed_vehicle, removed_pickup, removed_delivery) {
                if let Err(err) = solution.insert_call(removed_vehicle, call, pickup, delivery) {
                    eprintln!("Failed to reinsert call {:?} into vehicle {:?}: {:?}", call, removed_vehicle, err);
                }
            }
        }
        false
    };

    // Couldn't find any valid placement for any call, so just move the first call to the dummy vehicle.
    if !inserted {
        let _ = solution.remove_call(first_call);
    }

    (evaluated, infeasible)
}
