use bevy::prelude::*;

/// Simple LCG random number generator.
#[derive(Resource)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Returns a value in [0, max).
    pub fn next(&mut self, max: u32) -> u32 {
        // LCG parameters from Numerical Recipes
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.state >> 33) as u32) % max
    }
}
