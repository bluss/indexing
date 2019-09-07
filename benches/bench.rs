#![feature(test)]

extern crate test;
extern crate rand;

extern crate indexing;


use test::Bencher;




use rand::{XorShiftRng, Rng, SeedableRng};

use indexing::algorithms::*;

fn test_data_max(n: usize, max: i32) -> Vec<i32> {
    let mut rng = XorShiftRng::from_seed([0; 16]);
    let mut v = Vec::new();
    for _ in 0..n {
        v.push(rng.gen_range(0, max));
    }
    v
}

const ZIPLEN: usize = 256;

#[cfg(feature="experimental_pointer_ranges")]
#[bench]
fn zip_1(bench: &mut Bencher) {
    let xs = test_data_max(ZIPLEN, 21);
    let ys = test_data_max(ZIPLEN, 21);
    bench.iter(|| {
        zip_dot_i32(&xs, &ys)
    });
}

#[cfg(feature="experimental_pointer_ranges")]
#[bench]
fn zip_2(bench: &mut Bencher) {
    let xs = test_data_max(ZIPLEN, 21);
    let ys = test_data_max(ZIPLEN, 21);
    bench.iter(|| {
        zip_dot_i32_prange(&xs, &ys)
    });
}

#[cfg(feature="experimental_pointer_ranges")]
#[bench]
fn copy_1(bench: &mut Bencher) {
    let xs = test_data_max(ZIPLEN, 21);
    let mut ys = test_data_max(ZIPLEN, 21);
    bench.iter(|| {
        copy(&xs, &mut ys)
    });
}

#[cfg(feature="experimental_pointer_ranges")]
#[bench]
fn copy_2(bench: &mut Bencher) {
    let xs = test_data_max(ZIPLEN, 21);
    let mut ys = test_data_max(ZIPLEN, 21);
    bench.iter(|| {
        copy_prange(&xs, &mut ys)
    });
}
