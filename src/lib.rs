use fnv::FnvHasher;
use std::hash::Hasher;
use rand_xoshiro::Xoshiro512StarStar;
use rand::{SeedableRng, RngCore};

const LARGEST_SAFE_INDEX: u8 = 61;

///This is the opposite of secure,
///but it is highly difficult to predict
///as long as the time when the rng is split
///is itself determined by the rng output.
pub struct SplittingRng {
    origin: u64,
    steps: u64,
    prng: Xoshiro512StarStar,
    bool_pool: BooleanList,
}

impl SplittingRng {
    pub fn new(origin: u64) -> Self {
        let mut root_rng: Xoshiro512StarStar = SeedableRng::seed_from_u64(origin);
        let bool_p = BooleanList::new(root_rng.next_u64());
        SplittingRng {
            origin: origin,
            steps: 0,
            prng: root_rng,
            bool_pool: bool_p,
        }
    }
    /// Catch up this rng to a certain number of steps in the future
    /// Possibly slow
    pub fn fast_forward_from_origin(origin: u64, steps: u64, bools: (u64, u8)) -> Self {
        let mut result = Self::new(origin);
        for _ in 0..steps {
            result.step();
        }
        result.bool_pool = BooleanList::new(bools.0);
        result.bool_pool.last = bools.1;
        result
    }
    /// Dump this rng and its current state to disk
    pub fn to_raw(&self) -> (u64, u64, u64, u8) {
        (self.origin, self.steps, self.bool_pool.inner, self.bool_pool.last)
    }
    pub fn child(&mut self) -> SplittingRng {
        SplittingRng::new(self.step())
    }
    pub fn get_bool(&mut self) -> bool {
        if let Some(r) = self.bool_pool.next() {
            return r;
        }
        self.bool_pool = BooleanList::new(self.step());
        return self.bool_pool
            .next()
            .expect("Failed to use brand new boolean pool");
    }
    pub fn get_u32(&mut self) -> u32 {
        //Try to shift away the lowest bits
        return (self.step()  >> 32) as u32;
    }
    pub fn get_u64(&mut self) -> u64 {
        self.step()
    }
    //lowest 3 bits are low entropy
    //Note - distribution is not even, because the possible values are probably not perfectly
    //divisible by the number of sides
    pub fn roll(&mut self, sides: u32) -> u32 {
        ((self.step() >> 3) % (sides as u64)) as u32
    }
    pub fn shuffle<T>(&mut self, list: &[T]) -> Vec<T>
    where
        T: Copy,
    {
        let item_ct = list.len();
        let mut intermediate = Vec::with_capacity(item_ct);
        let item_ct = item_ct as u64;

        //FIXME: salt may not be contributing here
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
    #[test]
    fn test_shuffle_uniformity() {
        let mut rng = SplittingRng::new(123456);
        let mut input = vec![];
        for i in 0..100 {
            input.push(i);
        }
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
