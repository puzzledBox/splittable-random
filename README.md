# Splittable Random RNG
### For Games & Stuff

## What's a splitting RNG?

A splitting RNG is a random number generator with the interesting property that, in addition to providing random numbers, it provide a new random number generator 'split' from itself. The split child RNG has a deterministic starting state, created from the output of the original RNG. So long as all the number generation in a codebase descends from a single root generator\* the randomness is *deterministic* - the program will behave the same if the inital seed of the inital RNG is the same. This allows for users to share seeds in procedural games - generating the same world each time on different devices, or the same item drops in a speed run.

## Example

```
// A good general RNG
use rand_xoshiro::Xoshiro256StarStar;

// Create a new splittable RNG with a seed
// (For a random seed, try the timestamp millis)
let mut rng = SplittingRng::<Xoshiro256StarStar>::new(12345);

// An ordered vector
let input: Vec<_> = (0..5).collect(); //[0, 1, 2, 3, 4]

// A random vector
let random_vec = rng.shuffle(&input); //[3, 4, 0, 1, 2]

// Produces a fast but biased roll on a 6 sided die
// With six sides, biased toward 1 and 2 by 1/715827882
// Gets more biased as the number of sides goes up
let biased_roll = rng.biased_roll(6); // 4

// Produces a fair roll. On a 6 sided die,
// each 1/715827882 rolls it will reroll
// Rerolls more often as the number of sides goes up
let fair_roll = rng.fair_roll(6); // 5

// Produces a fair coin flip
let flip = rng.get_bool(); // false

// Make a new rng
let mut rng_b = rng.split();

// Some number
let a = rng.get_u64(); //2947371003896198809

// Produces different results, but still predictable
// based on the initial seed
let b = rng_b.get_u64(); //6870051922617725499
```

## What isn't a splitting RNG?
Secure, thread-safe, or a perfect solution to reproducable runs. While it is a valuable part of a toolkit, care must be taken in how the randomeness is used & combined with user input, and with threads.\*

At the moment, the rng is not serializable and can't be quickly fast-forwarded to a much later state, so it is recommended to avoid situations where a heavily-used RNG is saved and resumed later, except during a single execution of a program.

## \* Additional Caveats
Determinism also requires - the same i/o responses when using files on disk or a network, the same inputs at the same time, and that if the game uses threads, the same RNG is provided to the same thread and does the same amount of number generation each run.

This probably requires thread-affinity for the generator, the tasks being executed, and the i/o. In addition, you may need global barriers that block & sync threads after each batch of work to ensure that when randomness in data from different threads interacts, there's no change in the ordering of events.
