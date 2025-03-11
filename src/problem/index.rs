use crate::problem::Problem;
use crate::types::*;
use crate::utils::Matrix2;

/// Precomputed data structures
#[derive(Default)]
pub(super) struct ProblemIndex {
    /// For each call, the list of compatible vehicles
    pub(super) cargo_vessel: Vec<Vec<VehicleId>>,
}

impl ProblemIndex {
    /// Create a new problem index with precomputed data structures
    pub fn new(problem: &Problem) -> Self {
        ProblemIndex {
            cargo_vessel: Self::create_cargo_vessel(&problem.vessel_cargo),
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
