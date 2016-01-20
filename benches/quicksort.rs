#![feature(test)]

extern crate test;
extern crate rand;

extern crate indexing;

use rand::{StdRng, Rng, SeedableRng};

use test::Bencher;
use test::black_box;


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

#[bench]
fn std_binary_search(b: &mut Bencher) {
    let mut data = test_data_max(N, MAX);
    let elements = [0, 1, 2, 7, 29, MAX/3, MAX/2, MAX];
    data.sort();
    b.iter(|| {
        for elt in &elements {
            black_box(data.binary_search(&elt));
        }
    });
}

#[bench]
fn indexing_binary_search(b: &mut Bencher) {
    let mut data = test_data_max(N, MAX);
    let elements = [0, 1, 2, 7, 29, MAX/3, MAX/2, MAX];
    data.sort();
    b.iter(|| {
        for elt in &elements {
            black_box(indexing::algorithms::binary_search(&data, &elt));
        }
    });
}
