use crate::operators::mutate::mutate;
use crate::problem::Problem;
use crate::solution::Solution;
use crate::types::Cost;

pub fn local_search(
    problem: &Problem,
    mut initial_solution: Solution,
    max_iter: usize,
) -> (Cost, Solution) {
    let mut best_cost = initial_solution.cost(problem);
    let mut best_solution = initial_solution;
    let mut current_solution;

    let mut _infeasible_count = 0;

    for _ in 0..max_iter {
        current_solution = best_solution.clone();
        mutate(&mut current_solution, problem);

        if current_solution.feasible(problem).is_err() {
            _infeasible_count += 1;
            continue;
        }

        let current_cost = current_solution.cost(problem);

        if current_cost < best_cost {
            best_cost = current_cost;
            best_solution = current_solution.clone();
        }
    }

    (best_cost, best_solution)
}
