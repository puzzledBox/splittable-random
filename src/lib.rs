use fnv::FnvHasher;
use rand::{RngCore, SeedableRng};
use std::hash::Hasher;

const LARGEST_SAFE_INDEX: u8 = 61;

/// A splitting rng which provides
/// several types of random value
/// and also produces seeded child
/// RNGs.
///
/// This is the opposite of secure,
/// but it is highly difficult to predict
/// as long as the time when the rng is split
/// is itself determined by the rng output.
pub struct SplittingRng<T: RngCore + SeedableRng> {
    origin: u64,
    steps: u64,
    prng: T,
    bool_pool: BooleanList,
}

impl<T: RngCore + SeedableRng> SplittingRng<T> {
    /// Create a new RNG using the origin RNG
    pub fn new(origin: u64) -> Self {
        let mut root_rng: T = SeedableRng::seed_from_u64(origin);
        let bool_p = BooleanList::new(root_rng.next_u64());
        SplittingRng {
            origin,
            steps: 0,
            prng: root_rng,
            bool_pool: bool_p,
        }
    }

    /// Catch up this rng to a certain number of steps in the future
    /// Possibly slow, as the underlying implementation is not able to jump ahead
    /// Prefer to same the interior state
    fn fast_forward_from_origin(origin: u64, steps: u64, bools: (u64, u8)) -> Self {
        let mut result = Self::new(origin);
        for _ in 0..steps {
            result.step();
        }
        result.bool_pool = BooleanList::new(bools.0);
        result.bool_pool.last = bools.1;
        result
    }

    /// Dump this rng and its current state to numbers
    pub fn to_raw(&self) -> (u64, u64, u64, u8) {
        (
            self.origin,
            self.steps,
            self.bool_pool.inner,
            self.bool_pool.last,
        )
    }

    /// Load an rng and its current state to numbers
    /// Note that the same T type must be used
    /// Gets slower the more the generator was used
    pub fn from_raw(raw: (u64, u64, u64, u8)) -> Self {
        let (origin, steps, inner, last) = raw;
        Self::fast_forward_from_origin(origin, steps, (inner, last))
    }

    /// Split this rng into itself and a child
    /// Advances the internal state of this
    /// rng as well as creating the new instance,
    /// so multiple sequential calls to `child`
    /// will produce distinct RNGs
    pub fn split(&mut self) -> SplittingRng<T> {
        SplittingRng::new(self.step())
    }

    /// Provide a random boolean
    pub fn get_bool(&mut self) -> bool {
        if let Some(r) = self.bool_pool.next() {
            return r;
        }
        self.bool_pool = BooleanList::new(self.step());
        self.bool_pool
            .next()
            .expect("Failed to use new boolean pool")
    }

    /// Provide an unsigned 32-bit integer
    pub fn get_u32(&mut self) -> u32 {
        // Shift away the lowest bits,
        // which are not usable
        (self.step() >> 32) as u32
    }
    /// Provide an unsigned 64-bit integer
    ///
    /// Note that the three lowest-significance bits
    /// are not as entropic as expected due to the
    /// underlying implementation
    pub fn get_u64(&mut self) -> u64 {
        self.step()
    }

    /// Roll a die with up to 2^32 sides
    ///
    /// Note that the  distribution is not even, because the possible values are probably
    /// not perfectly divisible by the number of sides. This inaccuracy grows with the
    /// number of sides.
    pub fn biased_roll(&mut self, sides: u32) -> u32 {
        if sides == 0 {
            return 0;
        }
        // lowest 3 bits are low entropy, shift away
        ((self.step() >> 3) % (sides as u64)) as u32
    }

    /// Roll a die with up to 2^32 sides
    /// and guarantee that that roll is fair
    /// Quite fast, but slower than the fast
    /// roll and can in principle run a very long
    /// time.
    ///
    /// Note that this slows down more when the number of sides
    /// is very large.
    pub fn fair_roll(&mut self, sides: u32) -> u32 {
        if sides == 0 {
            return 0;
        }
        // Roll first
        let mut step = self.step() >> 3;
        loop {
            // Find the largest number under which our roll will be fair
            let biggest = (sides as u64) * (u64::MAX / (sides as u64));
            if step > biggest {
                // the roll would not be fair
                // roll again
                step = self.step() >> 3;
            } else {
                return (step % (sides as u64)) as u32;
            }
        }
    }

    /// Shuffle a list of N items
    ///
    /// Unlike rolling, this shuffle is theoretically perfect
    /// Therefore, when rolling without replacement, this implementation
    /// is superior to rolling if you can tolerate the use of ~64 bits of
    /// temporary allocation per item in the input slice.
    ///
    /// When rolling on a list with replacement, it is suggested
    /// to shuffle that list at intervals if using `biased_roll`.
    pub fn shuffle<L>(&mut self, list: &[L]) -> Vec<L>
    where
        L: Copy,
    {
        let item_ct = list.len();
        let mut intermediate = Vec::with_capacity(item_ct);
        let item_ct = item_ct as u64;

        // Use up a little extra randomness on a salt here
        // though it should make no difference
        // TODO: Add prop tests to ensure there's no change
        let salt = self.step();
        let mut hasher = FnvHasher::with_key(self.step());
        for (idx, item) in list.iter().enumerate() {
            let salted = idx as u64 + salt;
            hasher.write_u64(salted);
            //lowest bits are low entropy
            //Reduce width to 32 bits with XOR to improve behavior
            let unsmushed = hasher.finish();
            let naive_dest =
                ((unsmushed & (u32::max_value() as u64)) | (unsmushed >> 32)) % item_ct;
            intermediate.push((naive_dest, *item));
            intermediate.sort_unstable_by(|(lhash, _), (rhash, _)| lhash.cmp(rhash));
        }
        intermediate.iter().map(|(_, item)| *item).collect()
    }

    fn step(&mut self) -> u64 {
        self.steps += 1;
        self.prng.next_u64()
    }
}

#[doc(hidden)]
/// A helper structure to generate 61 random bools
/// from each 64-bit output of an RngCore
struct BooleanList {
    inner: u64,
    last: u8,
}

impl BooleanList {
    fn new(base: u64) -> BooleanList {
        BooleanList {
            inner: base,
            last: 0,
        }
    }

    fn next(&mut self) -> Option<bool> {
        if self.last < LARGEST_SAFE_INDEX {
            //We should avoid the last 3 bits because they aren't really random
            let result = ((0x1000 << self.last) & self.inner) == 0;
            self.last += 1;
            return Some(result);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_xoshiro::Xoshiro256StarStar;
    #[test]
    fn test_shuffle_uniformity() {
        // This is a silly prop-test style exercise
        // Some fraction of the time this would fail but with a fixed
        // seed we know it works
        let mut rng = SplittingRng::<Xoshiro256StarStar>::new(12345);
        let input: Vec<_> = (0..100).collect();
        let mut acc = 0;
        let iter = 1000;
        for _ in 0..iter {
            acc += rng.shuffle(&input)[8];
        }
        let avg = acc as f64 / iter as f64;
        println!("Avg {}", avg);
        assert!(avg > 49.5);
        assert!(avg < 50.5);
    }
}
