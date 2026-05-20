use std::fmt;

use buf_list::etna::{property_read_exact_pos_on_eof, PropertyResult};
use crabcheck::profiling::quickcheck;
use crabcheck::quickcheck::{Arbitrary, Mutate};
use rand::Rng;

// Mirror src/bin/etna.rs Chunks: outer 0..8, inner 0..16 of random bytes.
#[derive(Clone)]
struct Chunks(Vec<Vec<u8>>);
impl fmt::Debug for Chunks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<R: Rng> Arbitrary<R> for Chunks {
    fn generate(rng: &mut R, _n: usize) -> Self {
        let outer = rng.random_range(0..8u32) as usize;
        let mut out = Vec::with_capacity(outer);
        for _ in 0..outer {
            let inner = rng.random_range(0..16u32) as usize;
            let mut chunk = Vec::with_capacity(inner);
            for _ in 0..inner {
                chunk.push(rng.random_range(0..=u8::MAX));
            }
            out.push(chunk);
        }
        Chunks(out)
    }
}

impl<R: Rng> Mutate<R> for Chunks {
    fn mutate(&self, rng: &mut R, _n: usize) -> Self {
        let mut out = self.0.clone();
        match rng.random_range(0u8..4) {
            0 if !out.is_empty() => {
                let i = rng.random_range(0..out.len());
                if !out[i].is_empty() {
                    let j = rng.random_range(0..out[i].len());
                    let b = rng.random_range(0u32..8);
                    out[i][j] ^= 1u8 << b;
                }
            }
            1 if !out.is_empty() => {
                let i = rng.random_range(0..out.len());
                if out[i].len() < 16 {
                    out[i].push(rng.random_range(0..=u8::MAX));
                }
            }
            2 if out.len() < 8 => {
                out.push(vec![]);
            }
            _ if !out.is_empty() => {
                out.pop();
            }
            _ => {}
        }
        Chunks(out)
    }
}

fn to_opt(r: PropertyResult) -> Option<bool> {
    match r {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 3 {
        return;
    }
    let result = match (args[1].as_str(), args[2].as_str()) {
        ("crabcheck", "ReadExactPosOnEof") => {
            quickcheck(|(Chunks(cs), start, extra): (Chunks, usize, usize)| {
                to_opt(property_read_exact_pos_on_eof(cs, start as u32, extra as u32))
            })
        }
        (a, b) => panic!("Unknown: {a} {b}"),
    };
    println!("Result: {:?}", result);
}
