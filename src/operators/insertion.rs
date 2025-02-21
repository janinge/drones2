use rand::prelude::*;
use rand::rng;

use crate::problem::Problem;
use crate::solution::Solution;
use crate::types::CallId;

pub fn random_placement(mut solution: Solution, problem: &Problem, calls: Vec<CallId>) -> Solution {
    let mut thread_rng = rng();

    for call in calls {
        let available_vehicles = problem.get_compatible_vehicles(call).to_vec();

        let mut shuffled_vehicles = available_vehicles.to_vec();
        shuffled_vehicles.shuffle(&mut thread_rng);

        let mut inserted = false;

        for vehicle in shuffled_vehicles {
            // Get capacity feasibility information
            let (_call_weight, capacity_result) =
                solution.find_spare_capacity_in_vehicle(problem, call, vehicle);

            // Clone the capacity result to avoid borrow issues
            let capacity_result_clone = capacity_result.clone();

            if capacity_result.is_some() {
                // Collect all feasible insertion points using our iterator
                let feasible_insertions: Vec<(usize, usize)> = solution
                    .get_feasible_insertions(problem, call, vehicle, &capacity_result_clone)
                    .collect();

                if !feasible_insertions.is_empty() {
                    // Choose a random feasible insertion point
                    let (pickup_idx, delivery_idx) =
                        *feasible_insertions.choose(&mut thread_rng).unwrap();

                    // Insert the call
                    if solution.insert_call(vehicle, call, pickup_idx, delivery_idx).is_ok() {
                        inserted = true;
                        break;
                    }
                }
            }
        }

        if inserted { break; }
    }

    solution
}
