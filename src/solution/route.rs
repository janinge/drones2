use crate::problem::Problem;
use crate::solution::compact::CompactIter;
use crate::types::{CallId, Cost, Time, VehicleId};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Route {
    pub(super) calls: Vec<Option<CallId>>,
    pub(super) length: usize,
}

#[derive(Debug)]
pub struct SimulationResult {
    pub times: Vec<Time>,
    pub loads: Vec<i32>,
    pub cost: Vec<Cost>,
    pub is_feasible: bool,
    pub infeasible_at: Option<usize>, // index of the call where infeasibility was detected
    pub error: Option<String>,
}

impl Route {
    pub(super) fn new() -> Self {
        Route {
            calls: Vec::new(),
            length: 0,
        }
    }

    pub(super) fn with_capacity(capacity: usize) -> Self {
        Route {
            calls: Vec::with_capacity(capacity),
            length: 0,
        }
    }

    pub(super) fn push(&mut self, call: CallId) {
        self.calls.push(Some(call));
        self.length += 1;
    }

    pub(super) fn insert(&mut self, call: CallId, pickup_idx: usize, delivery_idx: usize) {
        let (real_pickup, real_delivery) = self.logical_idx_to_real(pickup_idx, delivery_idx);

        assert!(real_pickup <= real_delivery,
                "Delivery index must be greater than or equal to the pickup index");

        self.insert_single(call.delivery(), real_delivery);
        self.insert_single(call.pickup(), real_pickup);

        self.length += 2;
    }

    fn insert_single(&mut self, call: CallId, idx: usize) {
        let prev_index = idx.checked_sub(1).unwrap_or(0);

        match self.calls.get(prev_index) {
            Some(None) => {
                // Slot before index empty: simply fill this.
                self.calls[prev_index] = Some(call);
            }
            Some(Some(_)) => {
                self.calls.insert(idx, Some(call));
            }
            None => {
                self.calls.push(Some(call));
            }
        }
    }

    pub(super) fn remove(&mut self, call_id: CallId) {
        let mut count = 0;

        for call in self.calls.iter_mut() {
            match call {
                Some(route) => {
                    if call_id.id() == route.id() {
                        *call = None;
                        count += 1;
                    }

                    // If presumably both pickup and delivery got removed, return
                    if count == 2 {
                        break;
                    }
                }
                None => continue
            }
        }
        self.length -= count;
    }

    /// Simulates the route schedule for the given vehicle.
    /// Returns a SimulationResult with cumulative times, loads, and travel costs.
    /// If a constraint is violated (e.g. vessel incompatibility or capacity exceeded),
    /// the simulation stops early and marks the route as infeasible.
    pub fn simulate(&self, problem: &Problem, vehicle: VehicleId) -> SimulationResult {
        let calls = self.route(); // Returns a compact Vec<CallId> from the route.
        if calls.is_empty() {
            return SimulationResult {
                times: vec![],
                loads: vec![],
                cost: vec![],
                is_feasible: true,
                infeasible_at: None,
                error: None,
            };
        }

        // For vehicle lookup; vehicles are stored in order so that route at index i
        // corresponds to vehicle with id = (i+1).
        let veh_idx = (vehicle.get() as usize) - 1;
        let max_capacity = problem.vehicles[veh_idx].capacity as i32;

        // Determine the starting node based on the first call.
        let start_node = if calls[0].is_pickup() {
            problem.origin_node(calls[0])
        } else {
            problem.destination_node(calls[0])
        };
        let mut current_time = problem.get_first_travel_time(vehicle, start_node);
        let mut current_load: i32 = 0;
        let mut current_node = start_node;

        let mut times = Vec::with_capacity(calls.len());
        let mut loads = Vec::with_capacity(calls.len());
        let mut cost_vec = Vec::with_capacity(calls.len());

        let mut feasible = true;
        let mut error = None;
        let mut infeasible_at = None;

        for (i, &call) in calls.iter().enumerate() {
            // Check if the vehicle is allowed to serve this call.
            if !problem.is_call_allowed(vehicle, call) {
                feasible = false;
                error = Some(format!("Vehicle {:?} is not allowed to serve call {:?}", vehicle, call));
                infeasible_at = Some(i);
                break;
            }

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
                error = Some(format!("Capacity exceeded on call {:?}: load {} > capacity {}", call, current_load, max_capacity));
                infeasible_at = Some(i);
                break;
            }
            loads.push(current_load);

            // Compute waiting time: how long until the call’s lower time window is open.
            let wait = problem.waiting_time(current_time, call);
            current_time += wait;

            // Add service time (loading or unloading) for this call.
            let service = problem.service_time(vehicle, call);
            current_time = current_time.saturating_add(service);

            times.push(current_time);

            // Compute travel cost from the current node to this call’s node.
            let call_node = if call.is_pickup() {
                problem.origin_node(call)
            } else {
                problem.destination_node(call)
            };
            let travel_cost = problem.get_travel_cost(vehicle, current_node, call_node);
            cost_vec.push(travel_cost);

            // Update current node.
            current_node = call_node;

            // If this is not the last call, add travel time to the next call.
            if i + 1 < calls.len() {
                let next_call = calls[i + 1];
                let next_node = if next_call.is_pickup() {
                    problem.origin_node(next_call)
                } else {
                    problem.destination_node(next_call)
                };
                let travel_time = problem.get_travel_time(vehicle, current_node, next_node);
                current_time = current_time.saturating_add(travel_time);
                // Also update current node for the next iteration.
                current_node = next_node;
            }
        }

        SimulationResult {
            times,
            loads,
            cost: cost_vec,
            is_feasible: feasible,
            infeasible_at,
            error,
        }
    }

    pub(super) fn route(&self) -> Vec<CallId> {
        if self.is_compact() {
            self.calls.iter().map(|x| x.unwrap()).collect()
        } else {
            self.calls.iter().filter_map(|x| *x).collect()
        }
    }

    pub(super) fn compact_iter(&mut self) -> impl Iterator<Item = CallId> + '_ {
        CompactIter::new(self)
    }

    pub(super) fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub(super) fn is_compact(&self) -> bool {
        self.length == self.calls.len()
    }

    pub(super) fn len(&self) -> usize {
        self.length
    }

    fn update_len(&mut self) {
        self.length = self.calls.iter().filter(|&x| x.is_some()).count();
    }

    fn logical_idx_to_real(&self, logical_pickup: usize, logical_delivery: usize) -> (usize, usize) {
        if self.is_compact() {
            return (logical_pickup, logical_delivery);
        }

        let mut real_pickup = None;
        let mut real_delivery = None;
        let mut non_zero = 0;

        for (idx, val) in self.calls.iter().enumerate() {
            if val.is_some() {
                if non_zero == logical_pickup {
                    real_pickup = Some(idx);
                }
                if non_zero == logical_delivery {
                    real_delivery = Some(idx);
                    break; // Stop early once both indices are found
                }
                non_zero += 1;
            }
        }

        let real_pickup = real_pickup.unwrap_or(self.calls.len());
        let real_delivery = real_delivery.unwrap_or(self.calls.len());

        (real_pickup, real_delivery)
    }
}
