use bevy::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

/// Simple LCG random number generator.
#[derive(Resource)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn from_entropy() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos() as u64);
        Self::new(seed ^ seed.rotate_left(29))
    }
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Returns a value in [0, max).
    pub fn next(&mut self, max: u32) -> u32 {
        if max == 0 {
            return 0;
        }
        // LCG parameters from Numerical Recipes
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.state >> 33) as u32) % max
    }

    pub fn unit_f32(&mut self) -> f32 {
        self.next(u32::MAX) as f32 / u32::MAX as f32
    }

    pub fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.unit_f32()
    }
}
