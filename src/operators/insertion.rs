use rand::prelude::*;
use crate::problem::Problem;
use crate::solution::Solution;
use crate::types::{CallId, VehicleId};


pub fn random_placement_one(solution: &mut Solution, problem: &Problem, calls: Vec<CallId>) -> (usize, usize) {
    let mut rng = rand::thread_rng();
    let (mut evaluated, mut infeasible) = (0, 0);
    let first_call = *calls.first().expect("At least one call must be provided");

    let mut inserted = false;
    for call in calls {
        // Remove the call from its current placement (if any) and record its original location.
        let removed = match solution.remove_call(call) {
            Ok((vehicle, pickup, delivery)) => (Some(vehicle), pickup, delivery),
            Err(_) => (None, None, None),
        };

        if attempt_insert_call(solution, problem, call, removed, &mut rng, &mut evaluated, &mut infeasible) {
            inserted = true;
            break;
        } else {
            // Reinstate the call into its original location if available.
            if let (Some(vehicle), Some(pickup), Some(delivery)) = removed {
                if let Err(err) = solution.insert_call(vehicle, call, pickup, delivery) {
                    eprintln!(
                        "Failed to reinsert call {:?} into vehicle {:?} at positions ({}, {}): {:?}",
                        call, vehicle, pickup, delivery, err
                    );
                }
            }
        }
    }

    // If no call was successfully repositioned, remove the first call (leaving it unassigned).
    if !inserted {
        let _ = solution.remove_call(first_call);
    }

    (evaluated, infeasible)
}

pub fn random_placement_all(
    solution: &mut Solution,
    problem: &Problem,
    calls: Vec<CallId>,
) -> (usize, usize) {
    let mut rng = rand::thread_rng();
    let mut evaluated = 0;
    let mut infeasible = 0;

    // Remove all specified calls from the current solution.
    for &call in &calls {
        let _ = solution.remove_call(call);
    }

    // For each call, attempt to insert it into a random feasible position.
    for call in calls {
        let _ = attempt_insert_call(solution, problem, call, (None, None, None), &mut rng, &mut evaluated, &mut infeasible);
    }

    (evaluated, infeasible)
}

/// Attempts to insert the given call into a random feasible insertion point,
/// skipping the original location given by `removed` (if any).
fn attempt_insert_call(
    solution: &mut Solution,
    problem: &Problem,
    call: CallId,
    removed: (Option<VehicleId>, Option<usize>, Option<usize>),
    rng: &mut impl Rng,
    evaluated: &mut usize,
    infeasible: &mut usize,
) -> bool {
    let mut available_vehicles = problem.get_compatible_vehicles(call).to_vec();
    available_vehicles.shuffle(rng);

    for vehicle in available_vehicles {
        // Get spare capacity: note that capacity_result is already a reference (&Option<CapacityResult>).
        let (_call_weight, capacity_result) =
            solution.find_spare_capacity_in_vehicle(problem, call, vehicle);

        if capacity_result.is_some() {
            let capacity_result = capacity_result.clone();
            // Pass capacity_result directly (it has type &Option<CapacityResult>).
            let mut feasible_insertions: Vec<(usize, usize)> = solution
                .get_feasible_insertions(problem, call, vehicle, &capacity_result)
                .collect();
            feasible_insertions.shuffle(rng);

            for (pickup_idx, delivery_idx) in feasible_insertions {
                *evaluated += 1;

                // If we have an original position, skip that exact insertion.
                if let (Some(orig_vehicle), Some(orig_pickup), Some(orig_delivery)) = removed {
                    if orig_vehicle == vehicle && orig_pickup == pickup_idx && orig_delivery == delivery_idx {
                        continue;
                    }
                }

                if solution.insert_call(vehicle, call, pickup_idx, delivery_idx).is_ok() {
                    if solution.feasible(problem).is_err() {
                        let _ = solution.remove_call(call);
                        *infeasible += 1;
                    } else {
                        return true;
                    }
                }
            }
        }
    }
    false
}
