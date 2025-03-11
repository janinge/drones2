use super::solution::*;
use crate::types::*;

#[cfg(test)]
mod solution_tests {
    use super::*;
    use crate::problem::Problem;

    #[test]
    fn test_insert_and_remove_calls() {
        let mut solution = Solution::from_params(3, 5);

        let v1 = VehicleId::new(1).unwrap();
        let v2 = VehicleId::new(2).unwrap();
        let v3 = VehicleId::new(3).unwrap();

        let c1 = CallId::new_pickup(1).unwrap();
        let c2 = CallId::new_pickup(2).unwrap();
        let c3 = CallId::new_pickup(3).unwrap();
        let c4 = CallId::new_pickup(4).unwrap();
        let c5 = CallId::new_pickup(5).unwrap();

        // Insert calls in a certain order
        solution.insert_call(v1, c1, 0, 1).unwrap();
        solution.insert_call(v1, c2, 1, 2).unwrap();
        solution.insert_call(v2, c3, 0, 1).unwrap();
        solution.insert_call(v3, c4, 0, 2).unwrap();
        solution.insert_call(v3, c5, 0, 2).unwrap();

        // Check expected routes
        assert_eq!(solution.route(v1), vec![c1, c2, c1.inverse(), c2.inverse()]);
        assert_eq!(solution.route(v2), vec![c3, c3.inverse()]);
        assert_eq!(solution.route(v3), vec![c5, c4, c4.inverse(), c5.inverse()]);

        // Remove some calls and check routes
        solution.remove_call(c2).unwrap();
        assert_eq!(solution.route(v1), vec![c1, c1.inverse()]);

        solution.remove_call(c4).unwrap();
        assert_eq!(solution.route(v3), vec![c5, c5.inverse()]);

        // Reinsert in different orders
        solution.insert_call(v1, c5, 1, 2).unwrap();
        solution.insert_call(v2, c2, 0, 1).unwrap();

        // Verify final routes
        assert_eq!(solution.route(v1), vec![c1, c5, c1.inverse(), c5.inverse()]);
        assert_eq!(solution.route(v2), vec![c2, c3, c2.inverse(), c3.inverse()]);
        assert_eq!(solution.route(v3), vec![]);
    }

    #[test]
    fn test_invalid_insertions() {
        let mut solution = Solution::from_params(2, 3);

        let v1 = VehicleId::new(1).unwrap();
        let c1 = CallId::new_pickup(1).unwrap();

        // Attempt to insert with delivery before pickup (should fail)
        assert!(solution.insert_call(v1, c1, 2, 1).is_err());

        // Attempt to remove a call that hasn't been inserted (should fail)
        assert!(solution.remove_call(c1).is_err());
    }

    #[test]
    fn test_reassign_call() {
        let mut solution = Solution::from_params(2, 3);

        let v1 = VehicleId::new(1).unwrap();
        let v2 = VehicleId::new(2).unwrap();
        let c1 = CallId::new_pickup(1).unwrap();

        // Insert a call in one vehicle, then reassign it
        solution.insert_call(v1, c1, 0, 1).unwrap();
        assert_eq!(solution.route(v1), vec![c1, c1.inverse()]);

        solution.insert_call(v2, c1, 0, 1).unwrap();
        assert_eq!(solution.route(v1), vec![]); // Should be removed from v1
        assert_eq!(solution.route(v2), vec![c1, c1.inverse()]);
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

    pub fn route_to_plain(route: Vec<CallId>) -> Vec<isize> {
        route.iter().map(|&call| call.raw() as isize).collect()
    }

    #[test]
    fn test_pylist_feasible() {
        let mut sol = setup_pylist();

        let problem = Problem::load("data/Call_80_Vehicle_20.txt").unwrap();

        assert!(sol.feasible(&problem).is_ok(), "Solution is not feasible");
    }

    #[test]
    fn test_pylist_cost() {
        let mut sol = setup_pylist();

        let problem = Problem::load("data/Call_80_Vehicle_20.txt").unwrap();

        assert_eq!(sol.cost(&problem), 10705457);
    }

    #[ignore]
    #[test]
    fn test_pylist_match_pickups() {
        // The Python-like list string, exactly as given:
        let sol = setup_pylist();

        // Expected routes for each vehicle
        let expected_pickups: Vec<Vec<isize>> = vec![
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
            let vehicle_id = VehicleId::new((vehicle + 1) as u8).unwrap();
            let actual_route = route_to_plain(sol.route(vehicle_id));
            assert_eq!(
                actual_route, *expected_route,
                "Mismatch in vehicle {:?} delivery: expected {:?}, got {:?}",
                vehicle_id, expected_route, actual_route
            );
        }
    }

    #[test]
    fn test_pylist_match_expected() {
        // The Python-like list string, exactly as given:
        let sol = setup_pylist();

        // Expected routes for each vehicle
        let expected_route: Vec<Vec<isize>> = vec![
            // Vehicle 1:
            vec![70, 18, -18, -70, 69, 48, 73, -69, -48, 56, -56, -73],
            // Vehicle 2:
            vec![64, -64, 49, -49],
            // Vehicle 3:
            vec![67, 42, -67, 3, -3, -42, 80, -80],
            // Vehicle 4:
            vec![
                15, -15, 11, -11, 61, 74, -61, 46, -74, 36, 50, -46, 14, -36, -50, -14,
            ],
            // Vehicle 5:
            vec![71, -71, 12, 72, -72, -12],
            // Vehicle 6:
            vec![22, -22, 34, 59, -34, 17, -59, 27, -27, -17, 33, -33],
            // Vehicle 7:
            vec![8, -8, 20, -20, 16, -16],
            // Vehicle 8:
            vec![39, -39, 55, -55, 10, -10],
            // Vehicle 9:
            vec![
                53, 41, 23, -41, -23, -53, 62, 35, -62, 45, -45, 65, -65, -35, 7, -7,
            ],
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
            let vehicle_id = VehicleId::new((vehicle + 1) as u8).unwrap();
            let actual_route = route_to_plain(sol.route(vehicle_id));
            assert_eq!(
                actual_route, *expected_route,
                "Mismatch in vehicle {:?} route: expected {:?}, got {:?}",
                vehicle_id, expected_route, actual_route
            );
        }
    }
}
