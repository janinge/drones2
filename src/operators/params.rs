#[derive(Clone, Copy)]
pub struct RemovalParams {
    pub selection_ratio: f32,  // Fraction of total calls to remove
    pub randomness: f32,       // Degree of randomness in selection
    pub cost_bias: f32,        // Influence of cost in selection
    pub assignment_bias: f32,  // Preference for assigned/unassigned calls
    pub min_removals: usize,   // Minimum removals
    pub max_removals: usize   // Maximum removals
}

pub enum SamplingMethod {
    Uniform,
    Gaussian,
    Exponential,
}
