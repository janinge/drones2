use rand::prelude::*;
use rand::rng;
use rand::seq::index::sample;

use crate::solution::Solution;
use crate::types::CallId;

pub(crate) fn random_calls(solution: &Solution, amount: usize) -> Vec<CallId> {
    let n = solution.call_assignments().len();
    let mut thread_rng = rng();
    sample(&mut thread_rng, n, amount)
        .iter()
        .map(|idx| {
            (idx + 1)
                .try_into()
                .expect("Out of range value generated for CallId")
        })
        .collect::<Vec<CallId>>()
}
