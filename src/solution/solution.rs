use rand::SeedableRng;
use crate::problem::Problem;
use crate::solution::feasibility::FeasibleInsertions;
use crate::solution::route::CapacityResult;
use crate::solution::Route;
use crate::types::*;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use rand_xoshiro::rand_core::RngCore;
use rand_xoshiro::SplitMix64;

#[derive(Debug)]
pub enum SolutionError {
    InvalidPickupIndex(String),
    InvalidDeliveryIndex(String),
    CallNotFound(String),
    VehicleOutOfBounds(String),
    InvalidInput(String),
}

#[derive(Clone, Debug)]
pub struct Solution {
    routes: Vec<Route>,
    assignments: Vec<Option<VehicleId>>,
    costs: Vec<CallCost>
}

#[derive(Default, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CallCost {
    pub total: Cost,
    pub pickup: Cost,
    pub delivery: Cost,
}

impl Solution {
    pub fn new(problem: &Problem) -> Self {
        Self::from_params(
            problem.n_vehicles().get() as usize,
            problem.n_calls().id() as usize,
        )
    }

    /// Creates a new solution with the given number of vehicles and calls.
    pub(crate) fn from_params(n_vehicles: usize, n_calls: usize) -> Self {
        let routes = vec![Route::new(); n_vehicles];
        let assignments = vec![None; n_calls];

        Self {
            routes,
            assignments,
            costs: vec![CallCost::default(); n_calls],
        }
    }

    pub fn from_vehicle_routes(problem: &Problem, vehicle_routes: Vec<Vec<CallId>>) -> Result<Self, SolutionError> {
        let n_vehicles = problem.n_vehicles().get() as usize;
        let n_calls = problem.n_calls().id() as usize;

        let mut solution = Solution::from_params(n_vehicles, n_calls);

        // Process each vehicle's route
        for (veh_index, route_calls) in vehicle_routes.iter().enumerate() {
            // Skip if this vehicle's index is out of bounds
            if veh_index >= n_vehicles {
                continue;
            }

            let vehicle_id = VehicleId::from_index(veh_index).ok_or_else(|| {
                SolutionError::VehicleOutOfBounds(format!("Vehicle index {} out of bounds", veh_index))
            })?;

            let mut route = Route::with_capacity(route_calls.len());
            for &call in route_calls {
                route.push(call);

                // If this is a pickup call, record the assignment in the assignments vector
                if call.is_pickup() {
                    solution.assignments[call.index()] = Some(vehicle_id);
                } else {
                    #[cfg(debug_assertions)]
                    {
                        // If this is a delivery call, check if the corresponding pickup call is already assigned correctly
                        let pickup_vehicle = solution.assignments[call.index()];
                        if pickup_vehicle.is_none() {
                            return Err(SolutionError::InvalidInput(format!(
                                "Delivery call {} has no corresponding pickup assignment",
                                call.id()
                            )));
                        } else if pickup_vehicle != Some(vehicle_id) {
                            return Err(SolutionError::InvalidInput(format!(
                                "Delivery call {} is assigned to vehicle {:?}, but pickup call is assigned to vehicle {:?}",
                                call.id(),
                                vehicle_id,
                                pickup_vehicle
                            )));
                        }
                    }
                }
            }

            solution.routes[veh_index] = route;
        }

        solution.costs = vec![CallCost::default(); n_calls];

        Ok(solution)
    }

    /// Creates a new solution from a Python-like list string.
    ///
    /// In the input string:
    /// - Nonzero integers represent call IDs.
    /// - Each call appears twice: the first occurrence is its pickup, the second its delivery.
    /// - A 0 signals a new vehicle.
    pub fn from_pylist(pylist: &str) -> Result<Self, SolutionError> {
        let trimmed = pylist.trim().trim_start_matches('[').trim_end_matches(']');
        let parsed: Result<Vec<i32>, _> = trimmed
            .split(',')
            .map(|s| s.trim().parse::<i32>())
            .collect();

        let numbers = parsed
            .map_err(|_| SolutionError::InvalidInput("Failed to parse integers".to_string()))?;

        if numbers.is_empty() {
            return Err(SolutionError::InvalidInput(
                "Input list is empty".to_string(),
            ));
        }

        // Split the vector into vehicle blocks using 0 as a separator.
        let mut vehicle_blocks: Vec<Vec<CallId>> = numbers
            .split(|&x| x == 0)
            .map(|block| {
                block
                    .iter()
                    .filter_map(|&x| CallId::new_pickup(x.try_into().unwrap()))
                    .collect::<Vec<CallId>>()
            })
            .collect();

        // Remove any empty block at the end.
        if let Some(last) = vehicle_blocks.last() {
            if last.is_empty() {
                vehicle_blocks.pop();
            }
        }
        let n_vehicles = vehicle_blocks.len();
        if n_vehicles == 0 {
            return Err(SolutionError::InvalidInput(
                "No vehicles found in input".to_string(),
            ));
        }

        // Determine total number of calls by taking the maximum absolute call value
        let max_call = numbers
            .iter()
            .filter(|&&x| x != 0)
            .map(|&x| x.abs())
            .max()
            .unwrap_or(0);
        if max_call == 0 {
            return Err(SolutionError::InvalidInput(
                "No valid calls found".to_string(),
            ));
        }
        let n_calls = max_call as usize;

        let mut routes = Vec::with_capacity(n_vehicles);
        let mut assignments: Vec<Option<VehicleId>> = vec![None; n_calls];

        // Process each vehicle block.
        // For each block, use a set to track the first occurrence (pickup) of each call.
        for (veh_index, block) in vehicle_blocks.iter().enumerate() {
            let mut seen: HashSet<CallId> = HashSet::with_capacity(block.len());
            let mut route = Route::with_capacity(block.len());

            for &call in (*block).iter() {
                let call = call.pickup();

                if !seen.contains(&call) {
                    seen.insert(call);
                    route.push(call);
                } else {
                    route.push(call.delivery());
                }
            }

            routes.push(route);

            // For each call in the route, record that it is assigned to this vehicle (vehicles are 1-indexed).
            for call_id in routes[veh_index].route() {
                if call_id.is_delivery() {
                    continue;
                }

                assignments[call_id.index()] = Some(
                    VehicleId::new((veh_index + 1).try_into().map_err(|_| {
                        SolutionError::VehicleOutOfBounds("Too many vehicles".to_string())
                    })?)
                    .ok_or(SolutionError::InvalidInput(
                        "Vehicle index must be nonzero".to_string(),
                    ))?,
                );
            }
        }

        Ok(Solution {
            routes,
            assignments,
            costs: vec![CallCost::default(); n_calls],
        })
    }

    /// Returns a Python-like list string.
    ///
    /// In the input string:
    /// - Nonzero integers represent call IDs.
    /// - Each call appears twice: the first occurrence is its pickup, the second its delivery.
    /// - A 0 signals a new vehicle.
    pub fn to_pylist(&self, pos_deliveries: bool) -> String {
        let mut flattened = Vec::new();

        // For each route, add the calls and a 0 as a separator.
        for route in &self.routes {
            let calls = route.route();
            if pos_deliveries {
                flattened.extend(calls.iter().map(|&call| call.id()));
            } else {
                flattened.extend(calls.iter().map(|&call| call.raw()));
            }
            flattened.push(0); // Separator between vehicles
        }

        // For dummy calls, iterate over the assignments vector.
        // If an assignment is None, that call is in the dummy vehicle.
        let mut dummy_calls = Vec::new();
        for (i, assignment) in self.assignments.iter().enumerate() {
            if assignment.is_none() {
                // CallId are 1-indexed. Create the pickup call.
                let call = CallId::new_pickup((i + 1) as i16).expect("CallId should be nonzero");
                dummy_calls.push(call.raw());
                dummy_calls.push(call.raw());
            }
        }

        // Sort dummy for readability
        dummy_calls.sort();

        flattened.extend(dummy_calls);

        format!(
            "[{}]",
            flattened
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    pub fn verify_ordering(&self) -> Result<(), String> {
        for (veh_idx, route) in self.routes.iter().enumerate() {
            // Get the compact list of calls from the route.
            let calls = route.route();
            let mut occurrences: HashMap<i16, Vec<usize>> = HashMap::new();

            // Record all positions for each call (using the absolute call id).
            for (i, call) in calls.iter().enumerate() {
                occurrences.entry(call.id()).or_default().push(i);
            }

            // Now check that each call appears exactly twice and in the correct order.
            for (call_abs, indices) in occurrences.iter() {
                if indices.len() != 2 {
                    return Err(format!(
                        "Vehicle {}: Call {} appears {} times (expected 2)",
                        veh_idx + 1,
                        call_abs,
                        indices.len()
                    ));
                }
                let first_call = calls[*indices.first().unwrap()];
                let second_call = calls[*indices.last().unwrap()];
                if !first_call.is_pickup() {
                    return Err(format!(
                        "Vehicle {}: Call {}'s first occurrence (index {}) is not a pickup",
                        veh_idx + 1,
                        call_abs,
                        indices.first().unwrap()
                    ));
                }
                if !second_call.is_delivery() {
                    return Err(format!(
                        "Vehicle {}: Call {}'s second occurrence (index {}) is not a delivery",
                        veh_idx + 1,
                        call_abs,
                        indices.last().unwrap()
                    ));
                }
            }
        }
        Ok(())
    }

    /// Inserts a call into a vehicle’s route at the specified delivery index.
    pub fn insert_call(
        &mut self,
        vehicle: VehicleId,
        call: CallId,
        pickup_idx: usize,
        delivery_idx: usize,
    ) -> Result<(), SolutionError> {
        if delivery_idx < pickup_idx {
            return Err(SolutionError::InvalidDeliveryIndex(format!(
                "Delivery index {} must be greater than or equal to the pickup index {}",
                delivery_idx, pickup_idx
            )));
        }

        // If already assigned, remove from its current vehicle.
        if self.assignments[call.index()].is_some() {
            self.remove_call(call)?;
        }

        let route = self.routes.get_mut(vehicle.index()).ok_or_else(|| {
            SolutionError::VehicleOutOfBounds(format!("Vehicle {:?} not found", vehicle))
        })?;

        route.insert(call, pickup_idx, delivery_idx);

        self.assignments[call.index()] = Some(vehicle);

        Ok(())
    }

    /// Removes a call from its vehicle’s route.
    pub fn remove_call(&mut self, call: CallId) -> Result<(VehicleId, Option<usize>, Option<usize>), SolutionError> {
        let vehicle_ref = &mut self.assignments[call.index()];

        let vehicle = vehicle_ref.ok_or_else(|| {
            SolutionError::CallNotFound(format!("Call {} is not assigned a vehicle", call.raw()))
        })?;

        let route = self.routes.get_mut(vehicle.index()).ok_or_else(|| {
            SolutionError::VehicleOutOfBounds(format!("Vehicle {:?} not found", vehicle))
        })?;

        let (removed_pickup, removed_delivery) = route.remove(call);

        // Reset the assignment.
        *vehicle_ref = None;
        
        debug_assert!(removed_pickup.is_some() || removed_delivery.is_some(),
                      "Call {:?} is missing pickup/delivery in route {:?}: {:?}/{:?}", 
                      call, vehicle.index(), removed_pickup, removed_delivery);
        
        Ok((vehicle, removed_pickup, removed_delivery))
    }

    pub fn route(&self, vehicle: VehicleId) -> Vec<CallId> {
        if vehicle.get() as usize > self.routes.len() {
            return Vec::new();
        }

        self.routes[vehicle.index()].route()
    }

    /// Checks whether the specified call is unassigned.
    pub fn is_unassigned(&self, call: CallId) -> bool {
        self.assignments[call.index()].is_none()
    }

    /// Provides an iterator over the assignments vector.
    pub fn assignments(&self) -> impl Iterator<Item = &Option<VehicleId>> {
        self.assignments.iter()
    }

    /// Returns a slice of routes (for use by operators).
    pub(crate) fn routes(&self) -> &[Route] {
        &self.routes
    }

    pub fn call_assignments(&self) -> &[Option<VehicleId>] {
        &self.assignments
    }
    
    pub fn len(&self) -> usize {
        self.assignments.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.routes.iter().all(|route| route.is_empty())
    }

    pub fn find_spare_capacity_in_vehicle(
        &mut self,
        problem: &Problem,
        call: CallId,
        vehicle: VehicleId,
    ) -> (CargoSize, &Option<CapacityResult>) {
        let call_weight = problem.cargo_size(call);

        (
            call_weight,
            self.routes[vehicle.index()].find_spare_capacity(problem, call_weight, vehicle),
        )
    }

    /// Returns an iterator of feasible insertion points (pickup_idx, delivery_idx) for a call
    pub fn get_feasible_insertions<'a>(
        &'a self,
        problem: &'a Problem,
        call: CallId,
        vehicle: VehicleId,
        capacity_result: &'a Option<CapacityResult>,
    ) -> impl Iterator<Item = (usize, usize)> + 'a {
        FeasibleInsertions::new(problem, self, vehicle, call, capacity_result)
            .into_iter()
            .flatten()
    }

    /// Checks whether the solution is feasible with respect to the given problem.
    ///
    /// For each vehicle’s route, we simulate the schedule:
    /// - The vehicle starts at its home node with its starting time.
    /// - For each call:
    ///   - Check that the vehicle is allowed to serve this call.
    ///   - The vehicle’s load is updated (increased for pickups, decreased for deliveries)
    ///     and compared against its capacity.
    ///   - The time window for the call is respected.
    ///   - The appropriate service (loading/unloading) time is added.
    ///   - If there is a next call, the travel time from the current node to the next call’s node is added.
    pub fn feasible(&mut self, problem: &Problem) -> Result<(), SolutionError> {
        for (i, route) in self.routes.iter_mut().enumerate() {
            let vehicle_id = VehicleId::new((i + 1) as u8).expect("VehicleId must be nonzero");

            // Check if we already have a simulation result
            if let Some(sim) = route.last_simulation() {
                if !sim.is_feasible {
                    // If we have a simulation result and it's infeasible, return error
                    return Err(SolutionError::InvalidInput(format!(
                        "Vehicle {} is infeasible at {}: {:?}",
                        vehicle_id.get(),
                        sim.infeasible_at.unwrap(),
                        sim.error.clone().unwrap_or("No error message".to_string())
                    )));
                }
                // If simulation exists and is feasible, continue to next route
                continue;
            }

            // No simulation result yet, so run simulate
            if !route.simulate(problem, vehicle_id, Some(self.costs.as_mut())) {
                let sim = route.last_simulation().unwrap();
                return Err(SolutionError::InvalidInput(format!(
                    "Vehicle {} is infeasible at {}: {:?}",
                    vehicle_id.get(),
                    sim.infeasible_at.unwrap(),
                    sim.error.clone().unwrap_or("No error message".to_string())
                )));
            }
        }
        Ok(())
    }

    /// Computes the total cost of the solution.
    /// For each route (vehicle), the cost is the sum of its route cost and port cost as computed by simulate().
    pub fn cost(&mut self, problem: &Problem) -> Cost {
        let mut total_cost: i32 = 0;

        // For each vehicle/route
        for (i, route) in self.routes.iter_mut().enumerate() {
            let vehicle_id = VehicleId::new((i + 1) as u8).expect("VehicleId must be nonzero");

            // Check if we already have a simulation result
            if let Some(sim) = route.last_simulation() {
                if sim.is_feasible {
                    // If we have a feasible simulation result, use its cost values
                    total_cost += sim.route_cost + sim.port_cost;
                    continue;
                }
                // If simulation exists but is infeasible, re-simulate (fallthrough)
            }

            // No simulation result yet or existing simulation is infeasible, so run simulate
            if route.simulate(problem, vehicle_id, Some(self.costs.as_mut())) {
                let sim = route.last_simulation().unwrap();
                total_cost += sim.route_cost + sim.port_cost;
            }
        }

        // Dummy cost: for each call that is unassigned (assignment is None),
        // add its not_transport_cost.
        let dummy_cost: i32 = self
            .assignments
            .iter()
            .enumerate()
            .filter(|(_, assignment)| assignment.is_none())
            .map(|(i, _)| {
                let call = CallId::new_pickup((i + 1).try_into().unwrap())
                    .expect("CallId should be nonzero");
                problem.not_transport_cost(call)
            })
            .sum();

        total_cost + dummy_cost
    }

    pub fn call_costs(&self) -> &Vec<CallCost> {
        &self.costs
    }
}

impl Hash for Solution {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut hasher_rng = SplitMix64::seed_from_u64(self.routes.len() as u64);

        for &vehicle in &self.assignments {
            let random_val = hasher_rng.next_u64();
            let hashed_value = match vehicle {
                Some(nonzero) => random_val ^ (nonzero.get() as u64),
                None => random_val,
            };
            state.write_u64(hashed_value);
        }
    }
}
