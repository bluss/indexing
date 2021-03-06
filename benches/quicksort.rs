#![feature(test)]

extern crate test;

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use test::Bencher;

use indexing::algorithms::*;

fn test_data_max(n: usize, max: i32) -> Vec<i32> {
    let mut rng = StdRng::from_seed([0; 32]);
    let mut v = Vec::new();
    for _ in 0..n {
        v.push(rng.gen_range(0, max));
    }
    v
}

const N: usize = 10240;
const MAX: i32 = 10240;

#[bench]
fn bench_quicksort_range(b: &mut Bencher) {
    let data = test_data_max(N, MAX);
    b.iter(|| {
        let mut v = data.clone();
        quicksort_range(&mut v);
        v
    });
}

#[cfg(feature="experimental_pointer_ranges")]
#[bench]
fn bench_quicksort_prange(b: &mut Bencher) {
    let data = test_data_max(N, MAX);
    b.iter(|| {
        let mut v = data.clone();
        quicksort_prange(&mut v);
        v
    });
}

#[bench]
fn bench_quicksort_bounds(b: &mut Bencher) {
    let data = test_data_max(N, MAX);
    b.iter(|| {
        let mut v = data.clone();
        quicksort_bounds(&mut v);
        v
    });
}

#[bench]
fn libstdsort(b: &mut Bencher) {
    let data = test_data_max(N, MAX);
    b.iter(|| {
        let mut v = data.clone();
        v.sort();
        v
    });
}

