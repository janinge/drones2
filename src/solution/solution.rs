use std::cmp::{max, Ordering};
use std::collections::{HashMap};
use crate::problem::Problem;
use crate::types::*;

#[derive(Debug)]
pub enum SolutionError {
    InvalidPickupIndex(String),
    InvalidDeliveryIndex(String),
    CallNotFound(String),
    VehicleOutOfBounds(String),
    InvalidInput(String),
}

#[derive(Clone, Debug, PartialEq)] // Note: Not deriving Eq because f32 does not implement Eq.
pub struct Solution {
    routes: Vec<Vec<CallId>>,
    routes_len: Vec<usize>,
    assignments: Vec<VehicleId>,
    deliveries: Vec<f32>,
}

impl Solution {
    fn logical_idx_to_real(route: &[CallId], logical_index: usize) -> Option<usize> {
        let mut count = 0;
        for (i, &val) in route.iter().enumerate() {
            if val != 0 {
                if count == logical_index {
                    return Some(i);
                }
                count += 1;
            } else {
                if count == logical_index {
                    return Some(i);
                }
            }
        }
        if count == logical_index {
            Some(route.len())
        } else {
            None
        }
    }

    pub fn new(problem: &Problem) -> Self {
        Self::from_params(problem.n_vehicles as usize, problem.n_calls as usize)
    }

    pub fn from_params(n_vehicles: usize, n_calls: usize) -> Self {
        let routes = vec![Vec::with_capacity(n_calls >> 1); n_vehicles];
        let routes_len = vec![0; n_vehicles];

        let assignments = vec![0; n_calls];
        let deliveries = vec![0.0; n_calls];

        Self { routes, routes_len, assignments, deliveries }
    }

    /// Creates a new solution from a Python-like list string.
    ///
    /// In the input string:
    /// - Nonzero integers represent call IDs.
    /// - Each call appears twice: the first occurrence is its pickup, the second its delivery.
    /// - A 0 signals a new vehicle.
    pub fn from_pylist(pylist: &str) -> Result<Solution, SolutionError> {
        let trimmed = pylist.trim().trim_start_matches('[').trim_end_matches(']');
        let parsed: Result<Vec<CallId>, _> = trimmed
            .split(',')
            .map(|s| s.trim().parse::<CallId>())
            .collect();

        let numbers = parsed.map_err(|_| SolutionError::InvalidInput("Failed to parse integers".to_string()))?;

        if numbers.is_empty() {
            return Err(SolutionError::InvalidInput("Input list is empty".to_string()));
        }

        // Split the vector into vehicle blocks using 0 as a separator.
        let mut vehicle_blocks: Vec<&[CallId]> = numbers.split(|&x| x == 0).collect();
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

        let mut routes: Vec<Vec<CallId>> = Vec::with_capacity(n_vehicles);
        let mut routes_len: Vec<usize> = vec![0; n_vehicles];
        let mut assignments: Vec<VehicleId> = vec![0; n_calls];
        let mut deliveries: Vec<f32> = vec![0.0; n_calls];

        // Process each vehicle block.
        // For each block, use a set to track the first occurrence (pickup) of each call.
        for (veh_index, block) in vehicle_blocks.iter().enumerate() {
            let mut seen: HashMap<CallId, f32> = HashMap::new();
            let mut route: Vec<CallId> = Vec::new();
            let block_len = block.len() as f32;

            for (call_idx, &call) in (*block).iter().enumerate() {
                let call = call.abs();

                if !seen.contains_key(&call) {
                    seen.insert(call, call_idx as f32);
                    route.push(call);
                } else {
                    // If the call is already in the route, this is the delivery event.
                    let pickup_idx = *seen.get(&call).unwrap();
                    let delivery_position = (f32::MAX / (block_len - pickup_idx)) * (call_idx as f32 - pickup_idx);
                    deliveries[(call - 1) as usize] = delivery_position;
                }
            }

            // Save the route and its length.
            routes_len[veh_index] = route.len();

            routes.push(route);

            // For each call in the route, record that it is assigned to this vehicle (vehicles are 1-indexed).
            for call_id in &routes[veh_index] {
                let call_idx = (call_id - 1) as usize;
                assignments[call_idx] = veh_index as VehicleId;
            }
        }

        Ok(Solution {
            routes,
            routes_len,
            assignments,
            deliveries,
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

        let call_index = (call - 1) as usize;
        // If already assigned, remove from its current vehicle.
        if self.assignments[call_index] != 0 {
            self.remove_call(call)?;
        }

        // Get mutable reference to the route for the target vehicle.
        let route = self
            .routes
            .get_mut(vehicle as usize)
            .ok_or_else(|| SolutionError::VehicleOutOfBounds(format!("Vehicle {} not found", vehicle)))?;

        let real_index = Self::logical_idx_to_real(route, pickup_idx)
            .ok_or_else(|| SolutionError::InvalidPickupIndex(format!("Invalid pickup index {}", pickup_idx)))?;

        // Now perform the insertion:
        if real_index < route.len() {
            let prev_index = max(real_index - 1, 0);

            if route[prev_index] == 0 {
                // Slot before index empty: simply fill this.
                route[prev_index] = call;
            } else {
                // Slot is occupied: shift elements to the right.
                route.insert(real_index, call);
            }
        } else {
            // Append the call at the end.
            route.push(call);
        }

        // Update the route's length.
        self.routes_len[vehicle as usize] += 1;

        // Record the assignment.
        self.assignments[call_index] = vehicle;

        let route_len = self.routes_len[vehicle as usize] as f32;
        let delivery_position = f32::MAX / (route_len - (pickup_idx as f32)) * (delivery_idx as f32 - pickup_idx as f32);

        self.deliveries[call_index] = delivery_position;

        Ok(())
    }

    /// Removes a call from the specified vehicle’s route.
    pub fn remove_call(
        &mut self,
        call: CallId,
    ) -> Result<(), SolutionError> {
        let vehicle = self.assignments[call as usize - 1];

        let route = self
            .routes
            .get_mut(vehicle as usize)
            .ok_or_else(|| SolutionError::VehicleOutOfBounds(format!("Vehicle {} not found", vehicle)))?;

        let index = route.iter().position(|&x| x == call)
            .ok_or_else(|| SolutionError::CallNotFound(format!("Call {} not found in vehicle {}", call, vehicle)))?;

        // Replace the value at this index with 0.
        route[index] = 0;

        self.routes_len[vehicle as usize] -= 1;

        // Reset the assignment.
        self.assignments[(call - 1) as usize] = 0;
        Ok(())
    }

    fn update_route_length(&mut self, vehicle: VehicleId) {
        self.routes_len[vehicle as usize] = self.routes[vehicle as usize]
            .iter().filter(|&&val| val != 0).count()
    }

    /// Returns an immutable view of the raw route for a given vehicle.
    pub fn route_raw(&self, vehicle: VehicleId) -> Option<&[CallId]> {
        self.routes.get(vehicle as usize).map(|r| r.as_slice())
    }

    pub fn route(&self, vehicle: VehicleId) -> Vec<CallId> {
        if vehicle as usize >= self.routes.len() {
            return Vec::new();
        }

        let counter = self.routes_len[vehicle as usize];
        let routes = self.routes[vehicle as usize].as_slice();

        if counter == routes.len() {
            Vec::from(routes)
        } else {
            routes.iter().filter(|&&val| val != 0).copied().collect()
        }
    }

    /// Returns a Vec of events where deliveries have been interspersed with pickups.
    pub fn intersperse_deliveries(&self, vehicle: VehicleId) -> Option<Vec<CallId>> {
        let pickups = self.route(vehicle);
        let route_len = pickups.len();
        if route_len == 0 {
            return Some(Vec::new());
        }

        let mut events: Vec<(CallId, f32)> = Vec::with_capacity(route_len * 2);

        for (i, &call) in pickups.iter().enumerate() {
            let pickup_time = i as f32;
            events.push((call, pickup_time));
            let call_index = (call - 1) as usize;
            let delivery_raw = self.deliveries[call_index];
            // Scale the delivery value (which is in the range [0, f32::MAX]) into the range [i, count - 1].
            // This is done by: new_delivery = i + (delivery_raw / f32::MAX) * ((count - 1) - i)
            let new_delivery = i as f32 + (delivery_raw / f32::MAX) * (route_len as f32 - i as f32);
            // For deliveries, we store the call id as negative.
            events.push((-call, new_delivery));
        }

        // Sort events by time; if times are equal, sort by call id in descending order
        // so that pickups (positive) come before deliveries (negative).
        events.sort_by(|a, b| {
            let time_cmp = a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal);
            if time_cmp == Ordering::Equal {
                b.0.cmp(&a.0)
            } else {
                time_cmp
            }
        });

        println!("{:?}", events);
        Some(events.into_iter().map(|(call_id, _)| call_id).collect())
    }

    /// Checks whether the specified call is unassigned.
    pub fn is_unassigned(&self, call: CallId) -> bool {
        let call_index = (call - 1) as usize;
        self.assignments[call_index] == 0
    }

    /// Provides an iterator over the assignments vector.
    pub fn assignments(&self) -> impl Iterator<Item = &VehicleId> {
        self.assignments.iter()
    }

    pub fn is_feasible(&self, _problem: &Problem) -> bool {
        unimplemented!("Feasibility checking not implemented yet.")
    }

    pub fn cost(&self, _problem: &Problem) -> u32 {
        unimplemented!("Cost calculation not implemented yet.")
    }
}
