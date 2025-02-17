use super::solution::*; // Import everything from solution.rs
use crate::types::*; // Import types (VehicleId, CallId, etc.)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_route() {
        // Create a solution with 2 vehicles and 5 calls.
        let mut sol = Solution::from_params(2, 5);

        // Insert call 1 into vehicle 1 at pickup logical index 0 and delivery logical index 1.
        sol.insert_call(1, 1, 0, 1).unwrap();
        // Expect vehicle 1's raw route to contain call 1.
        assert_eq!(sol.route_raw(1).unwrap(), &[1]);
        // Logical route should be [1].
        assert_eq!(sol.route(1), vec![1]);
        // Intersperse deliveries for vehicle 1.
        let events = sol.intersperse_deliveries(1).unwrap();
        // With one call, pickup time is 0.0 and delivery time becomes 0.0.
        assert_eq!(events, vec![1, -1]);

        // Insert call 2 into vehicle 1 at pickup index 1 and delivery index 2.
        sol.insert_call(1, 2, 1, 2).unwrap();
        // Raw route now should be [2, 1, 2].
        assert_eq!(sol.route_raw(1).unwrap(), &[1, 2]);
        // Logical route should be [1, 2].
        assert_eq!(sol.route(1), vec![1, 2]);
        let events = sol.intersperse_deliveries(1).unwrap();
        // We expect pickup events for call 1 at time 0 and call 2 at time 1.
        assert_eq!(*events.first().unwrap(), 1);
        assert_eq!(*events.last().unwrap(), -2);

        // Remove call 1.
        sol.remove_call(1).unwrap();
        // Logical route should now be [2].
        assert_eq!(sol.route(1), vec![2]);
        let events = sol.intersperse_deliveries(1).unwrap();
        // For one call, events should be pickup (2, 0.0) and delivery (-2, 0.0).
        assert_eq!(events, vec![2, -2]);
    }

    #[test]
    fn test_multiple_inserts_and_removes() {
        // Create a solution with 1 vehicle and 5 calls.
        let mut sol = Solution::from_params(1, 5);
        // Insert calls 1, 2, 3 sequentially.
        sol.insert_call(0, 1, 0, 1).unwrap();
        sol.insert_call(0, 2, 1, 2).unwrap();
        sol.insert_call(0, 3, 2, 3).unwrap();
        assert_eq!(sol.route(0), vec![1, 2, 3]);
        // Remove call 2.
        sol.remove_call(2).unwrap();
        assert_eq!(sol.route(0), vec![1, 3]);
        // Insert call 4 into logical position 1 (between call 1 and 3).
        sol.insert_call(0, 4, 1, 2).unwrap();
        assert_eq!(sol.route(0), vec![1, 4, 3]);
        let events = sol.intersperse_deliveries(0).unwrap();
        // Expect 6 events (3 pickups and 3 deliveries).
        assert_eq!(events.len(), 6);
        assert_eq!(*events.first().unwrap(), 1);
        assert_eq!(*events.last().unwrap(), -3);
    }

    fn setup_pylist() -> Solution {
        let pylist = "[\
            70, 18, 18, 70, 69, 48, 73, 69, 48, 56, 56, 73, 0, \
            64, 64, 49, 49, 0, \
            67, 42, 67, 3, 3, 42, 80, 80, 0, \
            15, 15, 11, 11, 61, 74, 61, 46, 74, 36, 50, 46, 14, 36, 50, 14, 0, \
            71, 71, 12, 72, 72, 12, 0, \
            22, 22, 34, 59, 34, 17, 59, 27, 27, 17, 33, 33, 0, \
            8, 8, 20, 20, 16, 16, 0, \
            39, 39, 55, 55, 10, 10, 0, \
            53, 41, 23, 41, 23, 53, 62, 35, 62, 45, 45, 65, 65, 35, 7, 7, 0, \
            54, 63, 54, 63, 0, 21, 68, 68, 21, 0, \
            4, 4, 26, 37, 37, 26, 0, \
            25, 25, 9, 19, 9, 19, 52, 52, 0, \
            57, 57, 28, 28, 2, 2, 0, \
            66, 1, 66, 1, 0, \
            32, 29, 29, 32, 78, 78, 24, 24, 0, \
            30, 30, 43, 43, 5, 5, 58, 77, 58, 77, 47, 47, 6, 6, 0, \
            38, 51, 38, 31, 51, 31, 0, \
            76, 44, 44, 76, 0, \
            60, 40, 60, 40, 13, 79, 13, 75, 79, 75, 0\
        ]";

        Solution::from_pylist(pylist).expect("Failed to create Solution from pylist")
    }

    #[test]
    fn test_pylist_match_pickups() {
        // The Python-like list string, exactly as given:
        let sol = setup_pylist();

        // Expected routes for each vehicle
        let expected_pickups: Vec<Vec<CallId>> = vec![
            vec![70, 18, 69, 48, 73, 56],
            vec![64, 49],
            vec![67, 42, 3, 80],
            vec![15, 11, 61, 74, 46, 36, 50, 14],
            vec![71, 12, 72],
            vec![22, 34, 59, 17, 27, 33],
            vec![8, 20, 16],
            vec![39, 55, 10],
            vec![53, 41, 23, 62, 35, 45, 65, 7],
            vec![54, 63],
            vec![21, 68],
            vec![4, 26, 37],
            vec![25, 9, 19, 52],
            vec![57, 28, 2],
            vec![66, 1],
            vec![32, 29, 78, 24],
            vec![30, 43, 5, 58, 77, 47, 6],
            vec![38, 51, 31],
            vec![76, 44],
            vec![60, 40, 13, 79, 75],
        ];

        // Compare actual deliveries to expected deliveries
        for (vehicle, expected_route) in expected_pickups.iter().enumerate() {
            let vehicle_id = vehicle as VehicleId;
            let actual_route = sol.route(vehicle_id);
            assert_eq!(
                actual_route, *expected_route,
                "Mismatch in vehicle {} delivery: expected {:?}, got {:?}",
                vehicle_id, expected_route, actual_route
            );
        }
    }

    #[test]
    fn test_pylist_match_expected() {
        // The Python-like list string, exactly as given:
        let sol = setup_pylist();

        // Expected routes for each vehicle
        let expected_route: Vec<Vec<CallId>> = vec![
            // Vehicle 1:
            vec![70, 18, -18, -70, 69, 48, -69, 73, -48, 56, -56, -73],
            // Vehicle 2:
            vec![64, -64, 49, -49],
            // Vehicle 3:
            vec![67, 42, -67, 3, -3, -42, 80, -80],
            // Vehicle 4:
            vec![15, -15, 11, -11, 61, 74, -61, 46, -74, 36, 50, -46, 14, -36, -50, -14],
            // Vehicle 5:
            vec![71, -71, 12, 72, -72, -12],
            // Vehicle 6:
            vec![22, -22, 34, 59, -34, 17, -59, 27, -27, -17, 33, -33],
            // Vehicle 7:
            vec![8, -8, 20, -20, 16, -16],
            // Vehicle 8:
            vec![39, -39, 55, -55, 10, -10],
            // Vehicle 9:
            vec![53, 41, 23, -41, -23, -53, 62, 35, -62, 45, -45, 65, -65, -35, 7, -7],
            // Vehicle 10:
            vec![54, 63, -54, -63],
            // Vehicle 11:
            vec![21, 68, -68, -21],
            // Vehicle 12:
            vec![4, -4, 26, 37, -37, -26],
            // Vehicle 13:
            vec![25, -25, 9, 19, -9, -19, 52, -52],
            // Vehicle 14:
            vec![57, -57, 28, -28, 2, -2],
            // Vehicle 15:
            vec![66, 1, -66, -1],
            // Vehicle 16:
            vec![32, 29, -29, -32, 78, -78, 24, -24],
            // Vehicle 17:
            vec![30, -30, 43, -43, 5, -5, 58, 77, -58, -77, 47, -47, 6, -6],
            // Vehicle 18:
            vec![38, 51, -38, 31, -51, -31],
            // Vehicle 19:
            vec![76, 44, -44, -76],
            // Vehicle 20:
            vec![60, 40, -60, -40, 13, 79, -13, 75, -79, -75],
        ];


        // Compare actual deliveries to expected deliveries
        for (vehicle, expected_route) in expected_route.iter().enumerate() {
            let vehicle_id = vehicle as VehicleId;
            let actual_route = sol.intersperse_deliveries(vehicle_id).unwrap();
            assert_eq!(
                actual_route, *expected_route,
                "Mismatch in vehicle {} route: expected {:?}, got {:?}",
                vehicle_id, expected_route, actual_route
            );
        }
        }
}
