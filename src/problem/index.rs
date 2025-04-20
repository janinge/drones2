use crate::problem::Problem;
use crate::types::*;
use crate::utils::{Matrix2, IntervalTree};

/// Precomputed data structures
#[derive(Default)]
pub(super) struct ProblemIndex {
    /// For each call, the list of compatible vehicles
    pub(super) cargo_vessel: Vec<Vec<VehicleId>>,
    /// Interval tree for pickup windows
    pub(super) pickup_tree: IntervalTree,
    /// Interval tree for delivery windows
    pub(super) delivery_tree: IntervalTree,
}

impl ProblemIndex {
    /// Create a new problem index with precomputed data structures
    pub fn new(problem: &Problem) -> Self {
        // build global cargo->vehicles map
        let cargo_vessel = Self::create_cargo_vessel(&problem.vessel_cargo);

        // build interval trees for pickups and deliveries
        let pickup_windows = (1..=problem.n_calls.index()).map(|i| {
            let c = CallId::new_pickup(i as i16).unwrap();
            (c, problem.pickup_time_window(c))
        });
        let delivery_windows = (1..=problem.n_calls.index()).map(|i| {
            let c = CallId::new_pickup(i as i16).unwrap().delivery();
            (c, problem.delivery_time_window(c))
        });
        let pickup_tree = IntervalTree::new(pickup_windows);
        let delivery_tree = IntervalTree::new(delivery_windows);

        ProblemIndex {
            cargo_vessel,
            pickup_tree,
            delivery_tree,
        }
    }

    /// Create a new cargo_vessel Vec<Vec> from the vessel_cargo matrix
    fn create_cargo_vessel(vessel_cargo: &Matrix2<bool>) -> Vec<Vec<VehicleId>> {
        let num_calls = vessel_cargo.cols;
        let num_vehicles = vessel_cargo.rows;

        let mut compatible_vehicles = vec![Vec::new(); num_calls];

        // Transpose the vessel_cargo matrix
        for call_idx in 0..num_calls {
            for veh_idx in 0..num_vehicles {
                if *vessel_cargo.get(veh_idx, call_idx) {
                    let vehicle_id = VehicleId::new((veh_idx + 1) as u8).unwrap();
                    compatible_vehicles[call_idx].push(vehicle_id);
                }
            }
        }

        compatible_vehicles
    }

    /// Get compatible vehicles for a call
    #[inline(always)]
    pub fn get_compatible_vehicles(&self, call: CallId) -> &[VehicleId] {
        &self.cargo_vessel[call.index()]
    }
}
