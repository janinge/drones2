use crate::problem::Problem;
use crate::solution::compact::CompactIter;
use crate::types::{CallId, Capacity, CargoSize, Cost, Time, VehicleId};
use std::ops::RangeInclusive;
use crate::solution::solution::CallCost;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Route {
    pub(super) calls: Vec<Option<CallId>>,
    pub(super) length: usize,
    pub(super) simulation: Option<SimulationResult>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimulationResult {
    pub times: Vec<Time>,
    pub waiting: Vec<Time>,
    pub slack: Vec<Time>,
    pub min_slack: Vec<Time>, // reverse pass: min slack from index to the end of the route
    pub loads: Vec<Capacity>,
    pub capacity: Option<CapacityResult>,
    pub route_cost: Cost,
    pub port_cost: Cost,
    pub is_feasible: bool,
    pub infeasible_at: Option<usize>, // index of the call where infeasibility was detected
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CapacityResult {
    pub checked_min: Capacity,
    pub ranges: Vec<(Capacity, RangeInclusive<usize>)>,
}

/// Returns the index of the first element in `SimulationResult.times` that is greater than or equal to `target_time`.
impl SimulationResult {
    pub fn find_index_by_time(&self, target_time: Time) -> usize {
        self.times
            .binary_search(&target_time)
            .unwrap_or_else(|index| index)
    }
}

impl Route {
    pub(super) fn new() -> Self {
        Route {
            calls: Vec::new(),
            length: 0,
            simulation: None,
        }
    }

    /// Creates a new route with the given capacity.
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Route {
            calls: Vec::with_capacity(capacity),
            length: 0,
            simulation: None,
        }
    }

    pub(super) fn push(&mut self, call: CallId) {
        self.calls.push(Some(call));
        self.length += 1;
    }

    pub(super) fn insert(&mut self, call: CallId, pickup_idx: usize, delivery_idx: usize) {
        let (real_pickup, real_delivery) = self.logical_idx_to_real(pickup_idx, delivery_idx);

        self.insert_single(call.delivery(), real_delivery);
        self.insert_single(call.pickup(), real_pickup);

        self.length += 2;
        self.simulation = None;
    }

    /// Inserts a call into the route at the given index.
    /// If the index is out of bounds, the call is appended to the end of the route.
    fn insert_single(&mut self, call: CallId, idx: usize) {
        let prev_index = idx.saturating_sub(1);

        match self.calls.get(prev_index) {
            Some(None) => {
                // Slot before index empty: simply fill this.
                // TODO: Redo this logic. Equal real indexes gives pickup/delivery out of order.
                //self.calls[prev_index] = Some(call);

                // Same as bellow for now.
                self.calls.insert(idx, Some(call));
            }
            Some(Some(_)) => {
                self.calls.insert(idx, Some(call));
            }
            None => {
                self.calls.push(Some(call));
            }
        }
    }

    /// Removes the given call from the route.
    pub(super) fn remove(&mut self, call_id: CallId) -> (Option<usize>, Option<usize>) {
        let mut index = 0;
        
        let (mut pickup_index, mut delivery_index) = (None, None);

        for call in self.calls.iter_mut() {
            match call {
                Some(route) if call_id.id() == route.id() => {
                    if pickup_index.is_none() {
                        pickup_index = Some(index);
                        *call = None;
                    } else if delivery_index.is_none() {
                        delivery_index = Some(index);
                        *call = None;

                        // If presumably both pickup and delivery got removed, return
                        break;
                    }
                }
                Some(_) | None => {}
            }
            
            // Only count non-None (logical) indices.
            if call.is_some() {
                index += 1;
            }
        }
        self.length -= (pickup_index.is_some() as usize) + (delivery_index.is_some() as usize);
        self.simulation = None;
        
        (pickup_index, delivery_index)
    }

    /// Simulates the route schedule for the given vehicle.
    /// Returns a SimulationResult with cumulative times, loads, and travel costs.
    /// If a constraint is violated (e.g. vessel incompatibility or capacity exceeded),
    /// the simulation stops early and marks the route as infeasible
    pub fn simulate(&mut self, problem: &Problem, vehicle: VehicleId, mut call_costs: Option<&mut Vec<CallCost>>) -> bool {
        if self.is_empty() {
            self.simulation = Some(SimulationResult {
                times: vec![],
                waiting: vec![],
                slack: vec![],
                min_slack: vec![],
                loads: vec![],
                capacity: None,
                route_cost: 0,
                port_cost: 0,
                is_feasible: true,
                infeasible_at: None,
                error: None,
            });

            return true;
        }

        // For vehicle lookup; vehicles are stored in order so that route at index i
        // corresponds to vehicle with id = (i+1).
        let veh_idx = vehicle.index();
        let veh = &problem.vehicles[veh_idx];
        let max_capacity = veh.capacity;

        // Start at the depot node.
        let mut previous_node = veh.home_node;
        let mut route_cost: Cost = 0;
        let mut port_cost: Cost = 0;

        let mut current_time: Time = veh.starting_time;
        let mut current_load: i32 = 0;

        let mut times = Vec::with_capacity(self.len());
        let mut waiting = Vec::with_capacity(self.len());
        let mut slack = Vec::with_capacity(self.len());
        let mut loads = Vec::with_capacity(self.len());

        let mut feasible = true;
        let mut error = None;
        let mut infeasible_at = None;

        let route_calls = self.route();

        for (i, &call) in route_calls.iter().enumerate() {
            // Check if the call is allowed.
            if !problem.is_call_allowed(vehicle, call) {
                feasible = false;
                error = Some(format!(
                    "Vehicle {:?} not allowed to serve call {:?}",
                    vehicle, call
                ));
                infeasible_at = Some(i);
                break;
            }

            // Determine the node associated with the call.
            let call_node = if call.is_pickup() {
                problem.origin_node(call)
            } else {
                // For deliveries, add the port cost.
                port_cost += problem.port_cost_for_call(vehicle, call);
                problem.destination_node(call)
            };

            // Calculate this call's contribution to route cost
            if let Some(ref mut costs) = call_costs {
                // Calculate cost contribution based on position in route
                let this_call_cost: Cost = if i == 0 {
                    // First node in route
                    if route_calls.len() > 1 {
                        // Has a next node
                        let next_node = if route_calls[1].is_pickup() {
                            problem.origin_node(route_calls[1])
                        } else {
                            problem.destination_node(route_calls[1])
                        };

                        // Cost: depot -> this -> next, minus depot -> next
                        let cost_through = problem.get_travel_cost(vehicle, previous_node, call_node) +
                            problem.get_travel_cost(vehicle, call_node, next_node);
                        let cost_direct = problem.get_travel_cost(vehicle, previous_node, next_node);
                        cost_through - cost_direct
                    } else {
                        // Only node in route: just cost from depot
                        problem.get_travel_cost(vehicle, previous_node, call_node)
                    }
                } else if i == route_calls.len() - 1 {
                    // Last node in route: just cost from previous
                    problem.get_travel_cost(vehicle, previous_node, call_node)
                } else {
                    // Middle node
                    let next_node = if route_calls[i+1].is_pickup() {
                        problem.origin_node(route_calls[i+1])
                    } else {
                        problem.destination_node(route_calls[i+1])
                    };

                    // Cost: prev -> this -> next, minus prev -> next
                    let cost_through = problem.get_travel_cost(vehicle, previous_node, call_node) +
                        problem.get_travel_cost(vehicle, call_node, next_node);
                    let cost_direct = problem.get_travel_cost(vehicle, previous_node, next_node);
                    cost_through - cost_direct
                };

                // Get the call index for updating the costs
                let call_idx = call.index();

                // Update the appropriate cost fields
                if call.is_pickup() {
                    // For pickup, just store the cost
                    costs[call_idx].pickup = this_call_cost;
                } else {
                    // For delivery, update delivery cost and compute total
                    costs[call_idx].delivery = this_call_cost;
                    costs[call_idx].total = costs[call_idx].pickup + this_call_cost;
                }
            }


            // Add travel cost from previous node (starting depot or last call) to this call's node.
            route_cost += problem.get_travel_cost(vehicle, previous_node, call_node);
            // Also update simulation time by adding travel time.
            current_time = current_time.saturating_add(problem.get_travel_time(
                vehicle,
                previous_node,
                call_node,
            ));
            // Update previous node.
            previous_node = call_node;

            // Update load.
            // For a pickup, add cargo size; for a delivery, subtract it.
            let size = problem.cargo_size(call) as i32;
            if call.is_pickup() {
                current_load += size;
            } else {
                current_load -= size;
            }

            // Check capacity.
            if current_load > max_capacity {
                feasible = false;
                error = Some(format!(
                    "Capacity exceeded on call {:?}: load {} > capacity {}",
                    call, current_load, max_capacity
                ));
                infeasible_at = Some(i);
                break;
            }
            loads.push(current_load);

            let time_window = problem.time_window(call);

            // Compute slack time (how much we can delay before violating this call's upper bound)
            let slack_time = time_window.end().saturating_sub(current_time);
            slack.push(slack_time);

            // Compute waiting time (how much we have to wait before the call's lower bound opens)
            let waiting_time = time_window.start().saturating_sub(current_time);
            waiting.push(waiting_time);

            // Check time window.
            if waiting_time > 0 {
                // If the current time is before the call’s lower time window, wait until it opens.
                current_time = *time_window.start();
            } else if slack_time < 0 {
                // If the current time is after the call’s upper time window, the route is infeasible.
                feasible = false;
                error = Some(format!(
                    "Time window violated on call {:?}: time {} is outside [{}, {}]",
                    call,
                    current_time,
                    time_window.start(),
                    time_window.end()
                ));
                infeasible_at = Some(i);
                break;
            }

            // Add service time (loading or unloading) for this call.
            current_time = current_time.saturating_add(problem.service_time(vehicle, call));
            times.push(current_time);
        }

        let min_slack = Route::compute_min_remaining_slack(&slack, &waiting);

        self.simulation = Some(SimulationResult {
            times,
            waiting,
            slack,
            min_slack,
            loads,
            capacity: None,
            route_cost,
            port_cost,
            is_feasible: feasible,
            infeasible_at,
            error,
        });

        feasible
    }

    fn compute_min_remaining_slack(slack: &[Time], waiting: &[Time]) -> Vec<Time> {
        let n = slack.len();
        let mut min_slack = vec![0; n];

        if n == 0 {
            return min_slack;
        }

        min_slack[n - 1] = slack[n - 1];

        for i in (0..n - 1).rev() {
            if waiting[i + 1] > 0 {
                // Next call waiting? We can "recover" some slack:
                // i.e. see if (min_slack[i+1] - waiting[i+1]) is greater than or less than slack[i].
                let candidate = min_slack[i + 1].saturating_add(waiting[i + 1]);
                min_slack[i] = slack[i].min(candidate);
            } else {
                min_slack[i] = slack[i].min(min_slack[i + 1]);
            }
        }

        min_slack
    }

    pub(super) fn last_simulation(&self) -> Option<&SimulationResult> {
        self.simulation.as_ref()
    }

    /// Given a SimulationResult (with its sim.loads vector) and the call weight required,
    /// this function returns a vector of continuous ranges along the route (by index)
    /// where the available capacity (vehicle_capacity - sim.loads[i]) is at least call_weight.
    /// In other words, it merges candidate indices that are consecutive into ranges,
    /// and also computes the minimum available capacity within each range.
    pub(super) fn find_spare_capacity(&mut self, problem: &Problem, call_weight: CargoSize, vehicle: VehicleId) -> &Option<CapacityResult> {
        if self.simulation.is_none() {
            self.simulate(problem, vehicle, None);
        }
        
        let sim = self.simulation.as_ref().unwrap();
        let vehicle_capacity = problem.get_vehicle(vehicle).capacity;
        
        // Initialize our result vector
        let mut capacity_indices = Vec::new();
        
        // Always check capacity at index 0 (before any pickup)
        if vehicle_capacity >= call_weight as Capacity {
            capacity_indices.push(0);
        }
        
        // For each position in the route, check if there's enough capacity
        for i in 0..sim.loads.len() {
            let available_capacity = vehicle_capacity.saturating_sub(sim.loads[i]);
            if available_capacity >= call_weight as Capacity {
                capacity_indices.push(i + 1); // +1 because indices represent positions *after* stops
            }
        }
        
        // For empty routes, add index 0 if not already added
        if sim.loads.is_empty() && !capacity_indices.contains(&0) && vehicle_capacity >= call_weight as Capacity {
            capacity_indices.push(0);
        }
        
        // Always consider the end of the route (after the last stop)
        if !sim.loads.is_empty() && vehicle_capacity >= call_weight as Capacity {
            let last_idx = sim.loads.len();
            if !capacity_indices.contains(&last_idx) {
                capacity_indices.push(last_idx);
            }
        }
        
        // Find continuous ranges from the indices
        let mut continuous_ranges = Vec::new();
        
        if !capacity_indices.is_empty() {
            let mut start = capacity_indices[0];
            let mut end = start;
            
            for &idx in capacity_indices.iter().skip(1) {
                if idx == end + 1 {
                    // Continuous range, extend it
                    end = idx;
                } else {
                    // Gap found, push the current range and start a new one
                    continuous_ranges.push((vehicle_capacity, start..=end));
                    start = idx;
                    end = idx;
                }
            }
            
            // Don't forget the last range
            continuous_ranges.push((vehicle_capacity, start..=end));
        }
        
        let sim = self.simulation.as_mut().unwrap();
        
        // Update or initialize the capacity result
        match &mut sim.capacity {
            Some(capacity_result) => {
                // Update checked_min
                capacity_result.checked_min = capacity_result.checked_min.min(vehicle_capacity);
                // Append new ranges
                capacity_result.ranges.extend_from_slice(&continuous_ranges);
            },
            None => {
                // Initialize new capacity result
                sim.capacity = Some(CapacityResult {
                    checked_min: vehicle_capacity,
                    ranges: continuous_ranges,
                });
            }
        }
        
        &sim.capacity
    }

    pub(super) fn route(&self) -> Vec<CallId> {
        if self.is_compact() {
            self.calls.iter().map(|x| x.unwrap()).collect()
        } else {
            self.calls.iter().filter_map(|x| *x).collect()
        }
    }

    /// Returns an iterator over the route, compacting the route if necessary.
    pub(super) fn compact_iter(&mut self) -> impl Iterator<Item = CallId> + '_ {
        CompactIter::new(self)
    }

    /// Returns true if the route is empty.
    pub(super) fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Returns true if the route is compact, i.e. it has no empty slots.
    pub(super) fn is_compact(&self) -> bool {
        self.length == self.calls.len()
    }

    /// Returns the number of calls in the route.
    pub(super) fn len(&self) -> usize {
        self.length
    }

    fn update_len(&mut self) {
        self.length = self.calls.iter().filter(|&x| x.is_some()).count();
    }

    /// Converts a logical index (in the compact representation) to the real index (in the sparse representation).
    fn logical_idx_to_real(
        &self,
        logical_pickup: usize,
        logical_delivery: usize,
    ) -> (usize, usize) {
        if self.is_compact() {
            return (logical_pickup, logical_delivery);
        }

        let mut real_pickup_index: Option<usize> = None;
        let mut real_delivery_index: Option<usize> = None;
        let mut non_zero_count = 0;

        for (real_index, val) in self.calls.iter().enumerate() {
            if val.is_some() {
                if non_zero_count == logical_pickup {
                    real_pickup_index = Some(real_index);
                }
                if non_zero_count == logical_delivery {
                    real_delivery_index = Some(real_index);
                    break; // Stop early once both indices are found
                }
                non_zero_count += 1;
            }
        }

        let real_pickup = real_pickup_index.unwrap_or(self.calls.len()); // If not found, insert at the end
        let real_delivery = real_delivery_index.unwrap_or(self.calls.len()); // If not found, insert at the end

        (real_pickup, real_delivery)
    }
}
