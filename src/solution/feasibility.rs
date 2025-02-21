use crate::problem::Problem;
use crate::solution::route::SimulationResult;
use crate::solution::Solution;
use crate::types::{CallId, Capacity, Time, VehicleId};
use std::ops::RangeInclusive;

/// An iterator that yields feasible insertion point combinations (pickup_idx, delivery_idx)
/// while respecting time windows
pub struct FeasibleInsertions<'a> {
    problem: &'a Problem,
    vehicle: VehicleId,
    call: CallId,
    capacity_ranges: Vec<(Capacity, RangeInclusive<usize>)>,
    route_calls: Vec<CallId>,
    simulation: &'a SimulationResult,
    current_range_idx: usize,
    current_pickup_idx: usize,
    current_delivery_idx: usize,
    max_pickup_idx: usize,
    max_delivery_idx: usize,
}

impl<'a> FeasibleInsertions<'a> {
    pub fn new(
        problem: &'a Problem,
        solution: &'a Solution,
        vehicle: VehicleId,
        call: CallId,
        capacity_result: &'a Option<crate::solution::route::CapacityResult>,
    ) -> Option<Self> {
        if capacity_result.is_none() {
            return None;
        }

        let route = &solution.routes()[vehicle.index()];
        let route_calls = route.route();

        if let Some(sim) = route.last_simulation() {
            // Find the maximum indices for pickup and delivery based on time windows
            let pickup_window_end = *problem.pickup_time_window(call).end();
            let delivery_window_end = *problem.delivery_time_window(call).end();

            let max_pickup_idx = sim.find_index_by_time(pickup_window_end);
            let max_delivery_idx = sim.find_index_by_time(delivery_window_end);

            // Clone capacity ranges for iteration
            let capacity_ranges = capacity_result.as_ref().unwrap().ranges.clone();

            if !capacity_ranges.is_empty() {
                let first_range_start = *capacity_ranges[0].1.start();

                return Some(Self {
                    problem,
                    vehicle,
                    call,
                    capacity_ranges,
                    route_calls,
                    simulation: sim,
                    current_range_idx: 0,
                    current_pickup_idx: first_range_start,
                    current_delivery_idx: first_range_start,
                    max_pickup_idx,
                    max_delivery_idx,
                });
            }
        }

        None
    }
}

impl<'a> Iterator for FeasibleInsertions<'a> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Check if we've exhausted all ranges
            if self.current_range_idx >= self.capacity_ranges.len() {
                return None;
            }

            let current_range = &self.capacity_ranges[self.current_range_idx];
            let range_end = *current_range.1.end();

            // Pickup limit based on time window
            let pickup_limit = range_end.min(self.max_pickup_idx);

            // Check if current pickup is valid
            if self.current_pickup_idx > pickup_limit {
                // Move to next range
                self.current_range_idx += 1;
                if self.current_range_idx < self.capacity_ranges.len() {
                    let next_range = &self.capacity_ranges[self.current_range_idx];
                    self.current_pickup_idx = *next_range.1.start();
                    self.current_delivery_idx = self.current_pickup_idx;
                }
                continue;
            }

            // Delivery limit based on time window (delivery must be at or after pickup)
            let delivery_limit = range_end.min(self.max_delivery_idx);

            // Check if current delivery is valid
            if self.current_delivery_idx > delivery_limit {
                // Move to next pickup and reset delivery
                self.current_pickup_idx += 1;
                self.current_delivery_idx = self.current_pickup_idx;
                continue;
            }

            // Save current indices and advance delivery for next iteration
            let pickup_idx = self.current_pickup_idx;
            let delivery_idx = self.current_delivery_idx;
            self.current_delivery_idx += 1;

            // Check time feasibility
            let is_feasible = is_insertion_time_feasible(
                self.problem,
                &self.route_calls,
                self.simulation,
                self.vehicle,
                self.call,
                pickup_idx,
                delivery_idx,
            );

            if is_feasible {
                return Some((pickup_idx, delivery_idx));
            }
        }
    }
}

fn is_insertion_time_feasible(
    problem: &Problem,
    route_calls: &[CallId],
    sim: &SimulationResult,
    vehicle: VehicleId,
    call: CallId,
    pickup_idx: usize,
    delivery_idx: usize,
) -> bool {
    // Determine predecessor node for pickup
    let p_node = if pickup_idx == 0 || route_calls.is_empty() {
        problem.get_vehicle(vehicle).home_node
    } else if pickup_idx <= route_calls.len() {
        let prev_call = route_calls[pickup_idx - 1];
        if prev_call.is_pickup() {
            problem.origin_node(prev_call)
        } else {
            problem.destination_node(prev_call)
        }
    } else {
        // Handle out-of-bounds pickup_idx
        let last_call = *route_calls.last().unwrap();
        if last_call.is_pickup() {
            problem.origin_node(last_call)
        } else {
            problem.destination_node(last_call)
        }
    };

    // Determine successor node for delivery
    let d_node = if route_calls.is_empty() {
        problem.get_vehicle(vehicle).home_node
    } else if delivery_idx < route_calls.len() {
        let next_call = route_calls[delivery_idx];
        if next_call.is_pickup() {
            problem.origin_node(next_call)
        } else {
            problem.destination_node(next_call)
        }
    } else {
        // For delivery at the end of the route
        problem.get_vehicle(vehicle).home_node
    };

    // Compute the "original" travel time
    let orig_time = problem.get_travel_time(vehicle, p_node, d_node);

    // Compute new travel time with the inserted call
    let new_pickup = problem.origin_node(call);
    let new_delivery = problem.destination_node(call);

    // Calculate the new path times
    let new_time = problem.get_travel_time(vehicle, p_node, new_pickup)
        + problem.service_time(vehicle, call.pickup())
        + problem.get_travel_time(vehicle, new_pickup, new_delivery)
        + problem.service_time(vehicle, call.delivery())
        + problem.get_travel_time(vehicle, new_delivery, d_node);

    // Calculate the extra time required
    let delta = new_time.saturating_sub(orig_time);

    // Check if there's enough slack at this position
    let available_slack = if pickup_idx < sim.min_slack.len() {
        sim.min_slack[pickup_idx]
    } else if sim.min_slack.is_empty() {
        // For empty routes
        Time::MAX
    } else {
        // For insertions at the end
        sim.min_slack[sim.min_slack.len() - 1]
    };

    delta <= available_slack
}