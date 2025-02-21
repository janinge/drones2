use crate::problem::index::ProblemIndex;
use crate::types::*;
use crate::utils::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ops::RangeInclusive;

#[derive(Debug)]
pub struct Vehicle {
    /// 0-indexed node where the vehicle starts.
    pub home_node: NodeId,
    /// Starting time (in hours).
    pub starting_time: Time,
    /// Capacity (e.g. 5800, 13200, …).
    pub capacity: Capacity,
}

#[derive(Debug)]
pub struct CallParameters {
    /// 0-indexed origin node.
    pub origin: NodeId,
    /// 0-indexed destination node.
    pub destination: NodeId,
    /// Cargo size.
    pub size: CargoSize,
    /// Cost of not transporting.
    pub not_transport_cost: Cost,
    /// Acceptable pickup time window (inclusive).
    pub pickup_window: RangeInclusive<Time>,
    /// Acceptable delivery time window (inclusive).
    pub delivery_window: RangeInclusive<Time>,
}

/// Enum to select a field from a call.
pub enum Cargo {
    OriginNode,
    DestinationNode,
    Size,
    CostOfNotTransporting,
}

/// The main problem data structure.
pub struct Problem {
    /// Number of nodes (always 39).
    pub n_nodes: NodeId,
    /// Number of vehicles.
    pub n_vehicles: VehicleId,
    /// Number of calls.
    pub n_calls: CallId,
    /// Vehicle-specific data.
    pub vehicles: Vec<Vehicle>,
    /// Call-specific data.
    pub calls: Vec<CallParameters>,
    /// For each vehicle, the travel time between every pair of nodes.
    /// Indexed as [vehicle][origin][destination].
    pub travel_time: Matrix3<Time>,
    /// For each vehicle, the travel cost between every pair of nodes.
    pub travel_cost: Matrix3<Cost>,
    /// For each vehicle, a boolean mask over calls (true if the call is allowed).
    pub vessel_cargo: Matrix2<bool>,
    /// For each vehicle and call, the loading time.
    pub loading_time: Matrix2<Time>,
    /// For each vehicle and call, the unloading time.
    pub unloading_time: Matrix2<Time>,
    /// For each vehicle and call, the port cost (origin cost + destination cost).
    pub port_cost: Matrix2<Cost>,
    /// Precomputed data structures.
    pub index: ProblemIndex,
}

impl Problem {
    /// Loads a problem from a CSV file.
    pub fn load(filename: &str) -> Result<Self, String> {
        let file = File::open(filename).map_err(|e| format!("File not found: {}", e))?;
        let reader = BufReader::new(file);
        let mut lines = reader
            .lines()
            .map(|l| l.map_err(|e| e.to_string()))
            .filter(|line| {
                if let Ok(ref s) = line {
                    let trimmed = s.trim();
                    !trimmed.is_empty()
                        && trimmed
                            .chars()
                            .next()
                            .map(|c| c.is_digit(10))
                            .unwrap_or(false)
                } else {
                    true
                }
            });

        // Read number of nodes
        let num_nodes_line = lines.next().ok_or("Expected number of nodes")??;
        let num_nodes: usize = num_nodes_line
            .trim()
            .parse()
            .map_err(|e| format!("Could not parse num_nodes: {}", e))?;

        // Read number of vehicles
        let num_vehicles_line = lines.next().ok_or("Expected number of vehicles")??;
        let num_vehicles: usize = num_vehicles_line
            .trim()
            .parse()
            .map_err(|e| format!("Could not parse num_vehicles: {}", e))?;

        // Read vehicle data
        let mut vehicles = Vec::with_capacity(num_vehicles);
        for _ in 0..num_vehicles {
            let line = lines.next().ok_or("Missing vehicle info")??;
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 4 {
                return Err("Vehicle info line has insufficient parts".into());
            }
            // parts: [vehicle_index, home_node, starting_time, capacity]
            let home_node_raw: u8 = parts[1]
                .trim()
                .parse()
                .map_err(|e| format!("Bad home node: {}", e))?;
            let starting_time_raw: u16 = parts[2]
                .trim()
                .parse()
                .map_err(|e| format!("Bad starting time: {}", e))?;
            let capacity_raw: u32 = parts[3]
                .trim()
                .parse()
                .map_err(|e| format!("Bad capacity: {}", e))?;
            vehicles.push(Vehicle {
                home_node: home_node_raw.checked_sub(1).ok_or("Home node underflow")? as NodeId,
                starting_time: starting_time_raw as Time,
                capacity: capacity_raw as Capacity,
            });
        }

        // Read number of calls
        let num_calls_line = lines.next().ok_or("Expected number of calls")??;
        let num_calls: usize = num_calls_line
            .trim()
            .parse()
            .map_err(|e| format!("Could not parse num_calls: {}", e))?;

        // Read vessel cargo (allowed calls)
        let mut vessel_cargo = Matrix2::new(num_vehicles, num_calls, false);
        for i in 0..num_vehicles {
            let line = lines.next().ok_or("Missing vessel cargo info")??;
            let parts: Vec<&str> = line.split(',').collect();
            // The first part should be the vehicle index.
            for part in parts.iter().skip(1) {
                let call_index: usize = part
                    .trim()
                    .parse()
                    .map_err(|e| format!("Bad call index in vessel cargo: {}", e))?;
                let idx = call_index
                    .checked_sub(1)
                    .ok_or("Call index underflow in vessel cargo")?;
                *vessel_cargo.get_mut(i, idx) = true;
            }
        }

        // Read call data
        let mut calls = Vec::with_capacity(num_calls);
        for _ in 0..num_calls {
            let line = lines.next().ok_or("Missing call info")??;
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 9 {
                return Err("Call info line has insufficient parts".into());
            }
            // parts: [call_index, origin, destination, size, cost, pickup_lb, pickup_ub, delivery_lb, delivery_ub]
            let origin_raw: u8 = parts[1]
                .trim()
                .parse()
                .map_err(|e| format!("Bad origin: {}", e))?;
            let destination_raw: u8 = parts[2]
                .trim()
                .parse()
                .map_err(|e| format!("Bad destination: {}", e))?;
            let size_raw: u16 = parts[3]
                .trim()
                .parse()
                .map_err(|e| format!("Bad size: {}", e))?;
            let not_transport_cost_raw: u32 = parts[4]
                .trim()
                .parse()
                .map_err(|e| format!("Bad cost: {}", e))?;
            let pickup_lb_raw: u16 = parts[5]
                .trim()
                .parse()
                .map_err(|e| format!("Bad pickup lb: {}", e))?;
            let pickup_ub_raw: u16 = parts[6]
                .trim()
                .parse()
                .map_err(|e| format!("Bad pickup ub: {}", e))?;
            let delivery_lb_raw: u16 = parts[7]
                .trim()
                .parse()
                .map_err(|e| format!("Bad delivery lb: {}", e))?;
            let delivery_ub_raw: u16 = parts[8]
                .trim()
                .parse()
                .map_err(|e| format!("Bad delivery ub: {}", e))?;
            calls.push(CallParameters {
                origin: origin_raw.checked_sub(1).ok_or("Origin underflow")? as NodeId,
                destination: destination_raw
                    .checked_sub(1)
                    .ok_or("Destination underflow")? as NodeId,
                size: size_raw as CargoSize,
                not_transport_cost: not_transport_cost_raw as Cost,
                pickup_window: (pickup_lb_raw as Time)..=(pickup_ub_raw as Time),
                delivery_window: (delivery_lb_raw as Time)..=(delivery_ub_raw as Time),
            });
        }

        // Read travel times and costs
        let total_travel_entries = num_vehicles * num_nodes * num_nodes;
        let mut travel_time = Matrix3::new(num_vehicles, num_nodes, num_nodes, 0 as Time);
        let mut travel_cost = Matrix3::new(num_vehicles, num_nodes, num_nodes, 0 as Cost);
        for _ in 0..total_travel_entries {
            let line = lines.next().ok_or("Missing travel time/cost")??;
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 5 {
                return Err("Travel time/cost line has insufficient parts".into());
            }
            // parts: [vehicle, origin, destination, travel_time, travel_cost]
            let vehicle_raw: usize = parts[0]
                .trim()
                .parse()
                .map_err(|e| format!("Bad vehicle index in travel: {}", e))?;
            let origin_raw: usize = parts[1]
                .trim()
                .parse()
                .map_err(|e| format!("Bad origin in travel: {}", e))?;
            let destination_raw: usize = parts[2]
                .trim()
                .parse()
                .map_err(|e| format!("Bad destination in travel: {}", e))?;
            let time: Time = parts[3]
                .trim()
                .parse()
                .map_err(|e| format!("Bad travel time: {}", e))?;
            let cost: Cost = parts[4]
                .trim()
                .parse()
                .map_err(|e| format!("Bad travel cost: {}", e))?;
            let v = vehicle_raw
                .checked_sub(1)
                .ok_or("Vehicle index underflow in travel data")?;
            let o = origin_raw
                .checked_sub(1)
                .ok_or("Origin index underflow in travel data")?;
            let d = destination_raw
                .checked_sub(1)
                .ok_or("Destination index underflow in travel data")?;
            *travel_time.get_mut(v, o, d) = time;
            *travel_cost.get_mut(v, o, d) = cost;
        }

        // Read node times/costs
        let total_node_entries = num_vehicles * num_calls;
        let mut loading_time = Matrix2::new(num_vehicles, num_calls, 0 as Time);
        let mut unloading_time = Matrix2::new(num_vehicles, num_calls, 0 as Time);
        let mut port_cost = Matrix2::new(num_vehicles, num_calls, 0 as Cost);
        for _ in 0..total_node_entries {
            let line = lines.next().ok_or("Missing node times/costs")??;
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 6 {
                return Err("Node times/costs line has insufficient parts".into());
            }
            // parts: [vehicle, call, load_time, origin_cost, unload_time, destination_cost]
            let vehicle_raw: usize = parts[0]
                .trim()
                .parse()
                .map_err(|e| format!("Bad vehicle index in node info: {}", e))?;
            let call_raw: usize = parts[1]
                .trim()
                .parse()
                .map_err(|e| format!("Bad call index in node info: {}", e))?;
            let load: Time = parts[2]
                .trim()
                .parse()
                .map_err(|e| format!("Bad load time: {}", e))?;
            let origin_cost: Cost = parts[3]
                .trim()
                .parse()
                .map_err(|e| format!("Bad origin cost: {}", e))?;
            let unload: Time = parts[4]
                .trim()
                .parse()
                .map_err(|e| format!("Bad unload time: {}", e))?;
            let destination_cost: Cost = parts[5]
                .trim()
                .parse()
                .map_err(|e| format!("Bad destination cost: {}", e))?;
            let v = vehicle_raw
                .checked_sub(1)
                .ok_or("Vehicle index underflow in node info")?;
            let c = call_raw
                .checked_sub(1)
                .ok_or("Call index underflow in node info")?;
            *loading_time.get_mut(v, c) = load;
            *unloading_time.get_mut(v, c) = unload;
            *port_cost.get_mut(v, c) = origin_cost + destination_cost;
        }

        let mut problem = Problem {
            n_nodes: num_nodes
                .try_into()
                .map_err(|e| format!("Too many nodes: {}", e))?,
            n_vehicles: num_vehicles
                .try_into()
                .ok()
                .and_then(VehicleId::new)
                .ok_or("Too few/many vehicles")?,
            n_calls: num_calls
                .try_into()
                .ok()
                .and_then(CallId::new_pickup)
                .ok_or("Too few/many calls")?,
            vehicles,
            calls,
            travel_time,
            travel_cost,
            vessel_cargo,
            loading_time,
            unloading_time,
            port_cost,
            index: ProblemIndex::default(),
        };

        problem.index = ProblemIndex::new(&problem);

        Ok(problem)
    }

    /// Returns the travel time for a given vehicle and node pair.
    pub fn get_travel_time(&self, vehicle: VehicleId, origin: NodeId, destination: NodeId) -> Time {
        *self
            .travel_time
            .get(vehicle.index(), origin as usize, destination as usize)
    }

    /// Returns the travel time for a given vehicle and two calls.
    /// If `origin` or `destination` is positive, it represents a pickup; if negative, it's a delivery.
    pub fn get_travel_time_between_calls(
        &self,
        vehicle: VehicleId,
        origin: CallId,
        destination: CallId,
    ) -> Time {
        let origin_node = if origin.is_pickup() {
            self.calls[origin.index()].origin
        } else {
            self.calls[origin.index()].destination
        };

        let destination_node = if destination.is_pickup() {
            self.calls[origin.index()].origin
        } else {
            self.calls[origin.index()].destination
        };

        self.get_travel_time(vehicle, origin_node, destination_node)
    }

    /// Returns the travel cost for a given vehicle and node pair.
    pub fn get_travel_cost(&self, vehicle: VehicleId, origin: NodeId, destination: NodeId) -> Cost {
        *self
            .travel_cost
            .get(vehicle.index(), origin as usize, destination as usize)
    }

    /// Returns the travel cost for a given vehicle and two calls.
    /// If origin is positive, it represents a pickup; if negative, it's a delivery.
    pub fn get_travel_cost_between_calls(
        &self,
        vehicle: VehicleId,
        origin: CallId,
        destination: CallId,
    ) -> Cost {
        let origin_node = if origin.is_pickup() {
            self.calls[origin.index()].origin
        } else {
            self.calls[origin.index()].destination
        };

        let destination_node = if destination.is_pickup() {
            self.calls[destination.index()].origin
        } else {
            self.calls[destination.index()].destination
        };

        self.get_travel_cost(vehicle, origin_node, destination_node)
    }

    /// Computes “first travel time” on the fly (from vehicle’s home node plus starting time).
    pub fn get_first_travel_time(&self, vehicle: VehicleId, destination: NodeId) -> Time {
        let veh = &self.vehicles[vehicle.index()];
        self.get_travel_time(vehicle, veh.home_node, destination) + veh.starting_time
    }

    /// Computes “first travel cost” on the fly.
    pub fn get_first_travel_cost(&self, vehicle: VehicleId, destination: NodeId) -> Cost {
        let veh = &self.vehicles[vehicle.index()];
        self.get_travel_cost(vehicle, veh.home_node, destination)
    }

    /// Returns the appropriate time window for a call.
    /// A positive call indicates a pickup; a negative call a delivery.
    #[deprecated]
    pub fn get_time_window(&self, call: CallId) -> (Time, Time) {
        if call.is_pickup() {
            (
                *self.calls[call.index()].pickup_window.start(),
                *self.calls[call.index()].pickup_window.end(),
            )
        } else {
            (
                *self.calls[call.index()].delivery_window.start(),
                *self.calls[call.index()].delivery_window.end(),
            )
        }
    }

    /// Returns a slice of vehicle IDs that are compatible with the given call
    pub fn get_compatible_vehicles(&self, call: CallId) -> &[VehicleId] {
        &self.index.cargo_vessel[call.index()]
    }

    /// Returns the origin node for the given call.
    #[inline(always)]
    pub fn origin_node(&self, call: CallId) -> NodeId {
        self.calls[call.index()].origin
    }

    /// Returns the destination node for the given call.
    #[inline(always)]
    pub fn destination_node(&self, call: CallId) -> NodeId {
        self.calls[call.index()].destination
    }

    /// Returns the cargo size for the given call.
    #[inline(always)]
    pub fn cargo_size(&self, call: CallId) -> CargoSize {
        self.calls[call.index()].size
    }

    /// Returns the penalty cost for not transporting the given call.
    #[inline(always)]
    pub fn not_transport_cost(&self, call: CallId) -> Cost {
        self.calls[call.index()].not_transport_cost
    }

    /// Returns the pickup time window for the given call.
    #[inline(always)]
    pub fn pickup_time_window(&self, call: CallId) -> RangeInclusive<Time> {
        self.calls[call.index()].pickup_window.clone()
    }

    /// Returns the delivery time window for the given call.
    #[inline(always)]
    pub fn delivery_time_window(&self, call: CallId) -> RangeInclusive<Time> {
        self.calls[call.index()].delivery_window.clone()
    }

    #[inline(always)]
    pub fn time_window(&self, call: CallId) -> RangeInclusive<Time> {
        let idx = call.index();
        if call.is_pickup() {
            self.calls[idx].pickup_window.clone()
        } else {
            self.calls[idx].delivery_window.clone()
        }
    }

    pub fn get_vehicle(&self, vehicle: VehicleId) -> &Vehicle {
        &self.vehicles[vehicle.index()]
    }

    /// Returns the service time for a call on the specified vehicle.
    /// For pickups, this is the loading time; for deliveries, the unloading time.
    #[inline(always)]
    pub fn service_time(&self, vehicle: VehicleId, call: CallId) -> Time {
        let veh_idx = vehicle.index();
        let call_idx = call.index();
        if call.is_pickup() {
            *self.loading_time.get(veh_idx, call_idx)
        } else {
            *self.unloading_time.get(veh_idx, call_idx)
        }
    }

    /// Returns the port cost (origin + destination cost) for the given call on the specified vehicle.
    #[inline(always)]
    pub fn port_cost_for_call(&self, vehicle: VehicleId, call: CallId) -> Cost {
        *self.port_cost.get(vehicle.index(), call.index())
    }

    /// Checks if a given call is allowed for the specified vehicle.
    #[inline(always)]
    pub fn is_call_allowed(&self, vehicle: VehicleId, call: CallId) -> bool {
        *self.vessel_cargo.get(vehicle.index(), call.index())
    }

    /// Computes the waiting time before service for a call,
    /// given the current time. If the current time is less than the call's lower bound,
    /// returns the difference; otherwise, returns 0.
    #[inline(always)]
    pub fn waiting_time(&self, current_time: Time, call: CallId) -> Time {
        let lower_bound = *self.time_window(call).start();
        if current_time < lower_bound {
            lower_bound - current_time
        } else {
            0
        }
    }

    /// Returns the travel time between two calls for the given vehicle.
    /// The calls are mapped to nodes based on their role:
    /// - For a pickup call, use the origin node.
    /// - For a delivery call, use the destination node.
    #[inline(always)]
    pub fn travel_time_between_calls(
        &self,
        vehicle: VehicleId,
        origin: CallId,
        destination: CallId,
    ) -> Time {
        let origin_node = if origin.is_pickup() {
            self.origin_node(origin)
        } else {
            self.destination_node(origin)
        };
        let destination_node = if destination.is_pickup() {
            self.origin_node(destination)
        } else {
            self.destination_node(destination)
        };
        self.get_travel_time(vehicle, origin_node, destination_node)
    }

    /// Returns the travel cost between two calls for the given vehicle,
    /// similar to travel_time_between_calls.
    #[inline(always)]
    pub fn travel_cost_between_calls(
        &self,
        vehicle: VehicleId,
        origin: CallId,
        destination: CallId,
    ) -> Cost {
        let origin_node = if origin.is_pickup() {
            self.origin_node(origin)
        } else {
            self.destination_node(origin)
        };
        let destination_node = if destination.is_pickup() {
            self.origin_node(destination)
        } else {
            self.destination_node(destination)
        };
        self.get_travel_cost(vehicle, origin_node, destination_node)
    }
}
