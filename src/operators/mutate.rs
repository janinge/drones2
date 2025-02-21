use crate::operators::insertion::random_placement;
use crate::operators::removal::random_calls;
use crate::problem::Problem;
use crate::solution::Solution;

pub fn mutate(mut solution: Solution, problem: &Problem) -> Solution {
    let calls = random_calls(&solution, 1);

    let solution = random_placement(solution, problem, calls);

    solution
}
