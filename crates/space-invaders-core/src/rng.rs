//! Small, dependency-free deterministic random source.
//!
//! Determinism matters: the checks harness must be able to replay a game and
//! get the same invader fire and bunker damage. `xorshift64*` is more than good
//! enough for a 1978 arcade game and keeps the simulation free of system
//! entropy and platform crates.

/// A seedable xorshift64* generator.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Creates a generator from a non-zero seed.
    ///
    /// A zero seed is replaced with a fixed constant so the generator can never
    /// enter its degenerate all-zero state.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    /// Returns the next 32 random bits.
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    /// Returns a value in `[low, high)`, or `low` when the range is empty.
    pub fn range(&mut self, low: u32, high: u32) -> u32 {
        if high <= low {
            return low;
        }
        low + self.next_u32() % (high - low)
    }

    /// Returns `true` for roughly one in `n` calls.
    pub fn one_in(&mut self, n: u32) -> bool {
        n > 1 && self.next_u32() % n == 0
    }
}

#[cfg(test)]
mod tests {
    use super::Rng;

    #[test]
    fn is_deterministic_and_bounded() {
        let mut a = Rng::new(1978);
        let mut b = Rng::new(1978);
        for _ in 0..1_000 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
        let mut rng = Rng::new(7);
        for _ in 0..1_000 {
            assert!((3..9).contains(&rng.range(3, 9)));
        }
        assert_eq!(Rng::new(7).range(4, 4), 4);
    }
}
