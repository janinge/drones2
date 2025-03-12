use rand::{random_range, rng};
use rand::distr::weighted::WeightedIndex;
use rand::prelude::*;

use crate::operators::insertion::{random_placement_one, random_placement_all};
use crate::operators::params::RemovalParams;
use crate::operators::removal::*;
use crate::problem::Problem;
use crate::solution::Solution;
use crate::types::CallId;

const REMOVAL_OPERATORS: [fn(&Solution, &RemovalParams) -> Vec<CallId>; 3] = [
    combined_cost,
    broken_vehicle,
    global_waiting
];

const WEIGHTS: [f64; 3] = [0.3, 0.5, 0.2];

const PARAMS: RemovalParams = RemovalParams {
    selection_ratio: 0.5,
    randomness: 0.1,
    cost_bias: 0.5,
    assignment_bias: 0.5,
    min_removals: 1,
    max_removals: 7,
};

pub fn roulette_wheel_tuned(solution: &mut Solution, problem: &Problem) -> (usize, usize) {
    let mut thread_rng = rng();

    let dist = WeightedIndex::new(WEIGHTS).unwrap();
    let selected_fn = REMOVAL_OPERATORS[dist.sample(&mut thread_rng)];

    let calls = selected_fn(solution, &PARAMS);

    random_placement_all(solution, problem, calls)
}

pub fn roulette_wheel_equal(solution: &mut Solution, problem: &Problem) -> (usize, usize) {
    let calls = match random_range(0..3) {
        0 => combined_cost(solution, &PARAMS),
        1 => broken_vehicle(solution,  &PARAMS),
        2 => global_waiting(solution, &PARAMS),
        _ => unreachable!(),
    };

    random_placement_all(solution, problem, calls)
}

pub fn mutate(solution: &mut Solution, problem: &Problem) -> (usize, usize) {
    let calls = random_calls(solution, 1);

    random_placement_one(solution, problem, calls)
}
