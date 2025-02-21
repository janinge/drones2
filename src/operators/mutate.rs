use crate::operators::insertion::random_placement;
use crate::operators::removal::random_calls;
use crate::problem::Problem;
use crate::solution::Solution;

pub fn mutate(solution: &mut Solution, problem: &Problem) -> (usize, usize) {
    let calls = random_calls(solution, 1);

    random_placement(solution, problem, calls)
}
