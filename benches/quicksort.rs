#![feature(test)]

extern crate test;
extern crate rand;

extern crate indexing;

use rand::{StdRng, Rng, SeedableRng};

use test::Bencher;


fn test_data(n: usize) -> Vec<i32> {
    test_data_max(n, 1000000)
}

fn test_data_max(n: usize, max: i32) -> Vec<i32> {
    let mut rng = StdRng::from_seed(&[]);
    let mut v = Vec::new();
    for _ in 0..n {
        v.push(rng.gen_range(0, max));
    }
    v
}

const N: usize = 10240;
const MAX: i32 = 10240;

#[bench]
fn quicksort_branded(b: &mut Bencher) {
    let data = test_data_max(N, MAX);
    b.iter(|| {
        let mut v = data.clone();
        indexing::algorithms::quicksort(&mut v);
        v
    });
}

#[bench]
fn quicksort_bounds(b: &mut Bencher) {
    let data = test_data_max(N, MAX);
    b.iter(|| {
        let mut v = data.clone();
        indexing::algorithms::quicksort_bounds(&mut v);
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
