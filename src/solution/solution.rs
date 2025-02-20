use std::collections::HashSet;

use crate::problem::Problem;
use crate::solution::Route;
use crate::types::*;

#[derive(Debug)]
pub(crate) enum SolutionError {
    InvalidPickupIndex(String),
    InvalidDeliveryIndex(String),
    CallNotFound(String),
    VehicleOutOfBounds(String),
    InvalidInput(String),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Solution {
    routes: Vec<Route>,
    assignments: Vec<Option<VehicleId>>,
}

impl Solution {
    pub(crate) fn new(problem: &Problem) -> Self {
        Self::from_params(problem.n_vehicles.get() as usize, problem.n_calls.id() as usize)
    }

    pub(crate) fn from_params(n_vehicles: usize, n_calls: usize) -> Self {
        let routes = vec![Route::new(); n_vehicles];
        let assignments = vec![None; n_calls];

        Self { routes, assignments }
    }

    /// Creates a new solution from a Python-like list string.
    ///
    /// In the input string:
    /// - Nonzero integers represent call IDs.
    /// - Each call appears twice: the first occurrence is its pickup, the second its delivery.
    /// - A 0 signals a new vehicle.
    pub fn from_pylist(pylist: &str) -> Result<Solution, SolutionError> {
        let trimmed = pylist.trim().trim_start_matches('[').trim_end_matches(']');
        let parsed: Result<Vec<i32>, _> = trimmed
            .split(',')
            .map(|s| s.trim().parse::<i32>())
            .collect();

        let numbers = parsed.map_err(|_| SolutionError::InvalidInput("Failed to parse integers".to_string()))?;

        if numbers.is_empty() {
            return Err(SolutionError::InvalidInput("Input list is empty".to_string()));
        }

        // Split the vector into vehicle blocks using 0 as a separator.
        let mut vehicle_blocks: Vec<Vec<CallId>> = numbers
            .split(|&x| x == 0)
            .map(|block| {
                block.iter()
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
            return Err(SolutionError::InvalidInput("No vehicles found in input".to_string()));
        }

        // Determine total number of calls by taking the maximum absolute call value
        let max_call = numbers.iter().filter(|&&x| x != 0).map(|&x| x.abs()).max().unwrap_or(0);
        if max_call == 0 {
            return Err(SolutionError::InvalidInput("No valid calls found".to_string()));
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
                    VehicleId::new(
                        (veh_index + 1)
                            .try_into()
                            .map_err(|_| SolutionError::VehicleOutOfBounds("Too many vehicles".to_string()))?
                    )
                        .ok_or(SolutionError::InvalidInput("Vehicle index must be nonzero".to_string()))?
                );
            }
        }

        Ok(Solution {
            routes,
            assignments,
        })
    }


    /// Inserts a call into a vehicle’s route at the specified delivery index.
    pub fn insert_call(
        &mut self,
        vehicle: VehicleId,
        call: CallId,
        pickup_idx: usize,
        delivery_idx: usize,
    ) -> Result<(), SolutionError> {
        if delivery_idx <= pickup_idx {
            return Err(SolutionError::InvalidDeliveryIndex(format!(
                "Delivery index {} must be greater than the pickup index {}",
                delivery_idx, pickup_idx
            )));
        }

        // If already assigned, remove from its current vehicle.
        if matches!(self.assignments[call.index()], Some(_)) {
            self.remove_call(call)?;
        }

        let route = self
            .routes
            .get_mut(vehicle.get() as usize - 1)
            .ok_or_else(|| SolutionError::VehicleOutOfBounds(format!("Vehicle {} not found", vehicle)))?;

        route.insert(call, pickup_idx, delivery_idx);

        self.assignments[call.index()] = Some(vehicle);

        Ok(())
    }

    /// Removes a call from the specified vehicle’s route.
    pub fn remove_call(&mut self, call: CallId) -> Result<(), SolutionError> {
        let vehicle_ref = &mut self.assignments[call.index()];

        let vehicle = vehicle_ref.ok_or_else(||
            SolutionError::CallNotFound(format!("Call {} is not assigned a vehicle", call.raw())))?;

        let route = self
            .routes
            .get_mut(vehicle.get() as usize - 1)
            .ok_or_else(|| SolutionError::VehicleOutOfBounds(format!("Vehicle {} not found", vehicle)))?;

        route.remove(call);

        // Reset the assignment.
        *vehicle_ref = None;
        Ok(())
    }

    pub fn route(&self, vehicle: VehicleId) -> Vec<CallId> {
        if vehicle.get() as usize > self.routes.len() {
            return Vec::new();
        }

        self.routes[vehicle.get() as usize - 1].route()
    }

    /// Checks whether the specified call is unassigned.
    pub fn is_unassigned(&self, call: CallId) -> bool {
        self.assignments[call.index()] == None
    }

    /// Provides an iterator over the assignments vector.
    pub fn assignments(&self) -> impl Iterator<Item = &Option<VehicleId>> {
        self.assignments.iter()
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
    pub fn is_feasible(&self, problem: &Problem) -> bool {
        for (i, route) in self.routes.iter().enumerate() {
            let vehicle_id = VehicleId::new((i + 1) as u8)
                .expect("VehicleId must be nonzero");
            let sim = route.simulate(problem, vehicle_id);
            if !sim.is_feasible {
                return false;
            }
        }
        true
    }

    pub fn cost(&self, _problem: &Problem) -> u32 {
        unimplemented!("Cost calculation not implemented yet.")
    }
}
