use crate::problem::Problem;
use crate::types::{CallId, CargoSize, Time, VehicleId};

use std::collections::{BTreeSet, HashSet};
use std::iter::Peekable;
use std::boxed::Box;
use std::cmp::Ordering;


#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingDelivery {
    call: CallId,
    deadline: Time,
}

impl Ord for PendingDelivery {
    fn cmp(&self, other: &Self) -> Ordering {
        self.deadline
            .cmp(&other.deadline)
            .then_with(|| self.call.id().cmp(&other.call.id()))
    }
}

impl PartialOrd for PendingDelivery {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct VehicleState<'a> {
    vehicle: VehicleId,
    now: Time,
    load: CargoSize,
    pickup_stack: Vec<CallId>,
    pending_deliveries: BTreeSet<PendingDelivery>,
    route: Vec<CallId>,
    time_stack: Vec<Time>,
    active: HashSet<CallId>,
    pu_starts: Peekable<Box<dyn Iterator<Item = (Time, CallId)> + 'a>>,
    pu_ends: Peekable<Box<dyn Iterator<Item = (Time, CallId)> + 'a>>,
    de_starts: Peekable<Box<dyn Iterator<Item = (Time, CallId)> + 'a>>,
    de_ends: Peekable<Box<dyn Iterator<Item = (Time, CallId)> + 'a>>,
    finished: bool,
}

impl<'a> VehicleState<'a> {
    fn new(problem: &'a Problem, vehicle: VehicleId) -> VehicleState<'a> {
        let now = problem.get_vehicle_start_time(vehicle);
        let pickup_tree = problem.pickup_tree();
        let delivery_tree = problem.delivery_tree();

        VehicleState {
            vehicle,
            now,
            load: 0,
            pickup_stack: Vec::new(),
            pending_deliveries: BTreeSet::new(),
            route: Vec::new(),
            time_stack: Vec::new(),
            active: HashSet::new(),
            pu_starts: (Box::new(pickup_tree.start_events_from(now)) as Box<dyn Iterator<Item = (Time, CallId)>>).peekable(),
            pu_ends: (Box::new(pickup_tree.end_events_from(now)) as Box<dyn Iterator<Item = (Time, CallId)>>).peekable(),
            de_starts: (Box::new(delivery_tree.start_events_from(now)) as Box<dyn Iterator<Item = (Time, CallId)>>).peekable(),
            de_ends: (Box::new(delivery_tree.end_events_from(now)) as Box<dyn Iterator<Item = (Time, CallId)>>).peekable(),
            finished: false,
        }
    }
    
    fn insert_pending_delivery(&mut self, call: CallId, problem: &Problem) {
        let deadline = *problem.delivery_time_window(call).end();
        self.pending_deliveries.insert(PendingDelivery { call, deadline });
    }
    
    fn remove_pending_delivery(&mut self, call: CallId, problem: &Problem) {
        let deadline = *problem.delivery_time_window(call).end();
        self.pending_deliveries.remove(&PendingDelivery { call, deadline });
    }
    
    fn pending_contains(&self, call: &CallId, problem: &Problem) -> bool {
        let deadline = *problem.delivery_time_window(*call).end();
        self.pending_deliveries.contains(&PendingDelivery { call: *call, deadline })
    }
    
    fn advance_active(&mut self) {
        while let Some(&(_, call)) = self.pu_starts.peek() {
            self.active.insert(call);
            self.pu_starts.next();
        }
        while let Some(&(t, call)) = self.pu_ends.peek() {
            if t > self.now { break; }
            self.active.remove(&call);
            self.pu_ends.next();
        }
        while let Some(&(_, call)) = self.de_starts.peek() {
            self.active.insert(call);
            self.de_starts.next();
        }
        while let Some(&(t, call)) = self.de_ends.peek() {
            if t > self.now { break; }
            self.active.remove(&call);
            self.de_ends.next();
        }
    }
    
    fn extend_one(
        &mut self,
        problem: &Problem,
        global_pool: &mut HashSet<CallId>
    ) -> bool {
        self.advance_active();

        let last_node = self.route.last().map(|&c| {
            if c.is_pickup() {
                problem.origin_node(c)
            } else {
                problem.destination_node(c)
            }
        }).unwrap_or(problem.get_vehicle_home_node(self.vehicle));
        
        let earliest_pd_opt = self.pending_deliveries.iter().next().cloned();
        
        let mut cands = Vec::new();
        for &c in self.active.iter() {
            if c.is_pickup() {
                if !global_pool.contains(&c) {
                    continue;
                }
                if i32::from(self.load + problem.cargo_size(c))
                    > problem.get_vehicle_capacity(self.vehicle) {
                    continue;
                }
                let node = problem.origin_node(c);
                let tw = problem.pickup_time_window(c);
                let travel = problem.get_travel_time(self.vehicle, last_node, node);
                let arrival = (self.now + travel).max(*tw.start());
                if arrival > *tw.end() {
                    continue;
                }
                // Simulate the new state after inserting this pickup
                let new_time = arrival + problem.service_time(self.vehicle, c);
                
                // If there is an earliest pending delivery, ensure that after this insertion it remains feasible
                if let Some(ref pd) = earliest_pd_opt {
                    let pd_tw = problem.delivery_time_window(pd.call);
                    let pd_node = problem.destination_node(pd.call);
                    let travel_pd = problem.get_travel_time(self.vehicle, node, pd_node);
                    let effective_arrival = (new_time + travel_pd).max(*pd_tw.start());
                    if effective_arrival > *pd_tw.end() {
                        // Inserting this pickup would invalidate a pending delivery
                        continue;
                    }
                }
                cands.push(c);
            } else {
                if !self.pending_contains(&c, problem) {
                    continue;
                }
                let node = problem.destination_node(c);
                let tw = problem.delivery_time_window(c);
                let travel = problem.get_travel_time(self.vehicle, last_node, node);
                let arrival = (self.now + travel).max(*tw.start());
                if arrival > *tw.end() {
                    continue;
                }
                cands.push(c);
            }
        }

        // If no candidate is feasible and there is a pending delivery, force insertion
        if cands.is_empty() {
            if let Some(pd) = earliest_pd_opt {
                let forced = pd.call; // this is a delivery call.
                let node = problem.destination_node(forced);
                let tw = problem.delivery_time_window(forced);
                let travel = problem.get_travel_time(self.vehicle, last_node, node);
                let arrival = (self.now + travel).max(*tw.start());
                if arrival > *tw.end() {
                    self.finished = true;
                    return false;
                }
                self.time_stack.push(self.now);
                self.route.push(forced);
                self.remove_pending_delivery(forced, problem);
                self.now = arrival + problem.service_time(self.vehicle, forced);
                return true;
            } else {
                self.finished = true;
                return false;
            }
        }

        // Otherwise, choose one candidate
        let choice = cands[0];

        // Record the current time.
        self.time_stack.push(self.now);
        self.route.push(choice);
        if choice.is_pickup() {
            self.load += problem.cargo_size(choice);
            self.pickup_stack.push(choice);
            self.insert_pending_delivery(choice.delivery(), problem);
            global_pool.remove(&choice);
            self.active.insert(choice.delivery());
        } else {
            let pu = choice.inverse();
            self.load -= problem.cargo_size(pu);
            self.remove_pending_delivery(choice, problem);
        }

        let node = if choice.is_pickup() {
            problem.origin_node(choice)
        } else {
            problem.destination_node(choice)
        };
        let tw = if choice.is_pickup() {
            problem.pickup_time_window(choice)
        } else {
            problem.delivery_time_window(choice)
        };
        let travel = problem.get_travel_time(self.vehicle, last_node, node);
        let arrival = (self.now + travel).max(*tw.start());
        self.now = arrival + problem.service_time(self.vehicle, choice);

        true
    }
}

pub fn weighted_random_calls(problem: &Problem) -> Vec<Vec<CallId>> {
    let mut builders: Vec<VehicleState> = (1..=problem.n_vehicles().get())
        .map(|i| VehicleState::new(problem, VehicleId::new(i).unwrap()))
        .collect();
    let mut global_pool: HashSet<CallId> = problem.all_calls().collect();
    
    while !global_pool.is_empty() && builders.iter().any(|b| !b.finished) {
        for b in &mut builders {
            if !b.finished {
                let _ = b.extend_one(problem, &mut global_pool);
            }
        }
    }
    builders.into_iter().map(|b| b.route).collect()
}
