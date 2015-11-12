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

#[bench]
fn bench_quicksort(b: &mut Bencher) {
    let data = test_data_max(1024, 128);
    b.iter(|| {
        let mut v = data.clone();
        indexing::algorithms::quicksort(&mut v);
        v
    });
}

#[bench]
fn bench_libstdsort(b: &mut Bencher) {
    let data = test_data_max(1024, 128);
    b.iter(|| {
        let mut v = data.clone();
        v.sort();
        v
    });
}