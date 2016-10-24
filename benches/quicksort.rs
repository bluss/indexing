#![feature(test)]

extern crate test;
extern crate rand;

extern crate indexing;

use std::cmp::Ordering;
use std::iter::FromIterator;

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
fn std_binary_search_is_ok(b: &mut Bencher) {
    let mut data = test_data_max(N, MAX);
    let elements = [0, 1, 2, 7, 29, MAX/3, MAX/2, MAX];
    data.sort();
    b.iter(|| {
        for elt in &elements {
            black_box(data.binary_search(elt).is_ok());
        }
    });
}

#[bench]
fn indexing_binary_search_is_ok(b: &mut Bencher) {
    let mut data = test_data_max(N, MAX);
    let elements = [0, 1, 2, 7, 29, MAX/3, MAX/2, MAX];
    data.sort();
    b.iter(|| {
        for elt in &elements {
            black_box(indexing::algorithms::binary_search(&data, elt).is_ok());
        }
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
            black_box(indexing::algorithms::binary_search(&data, elt));
        }
    });
}

#[bench]
fn indexing_lower_bound_fake(b: &mut Bencher) {
    let mut data = test_data_max(N, MAX);
    let elements = [0, 1, 2, 7, 29, MAX/3, MAX/2, MAX];
    data.sort();
    b.iter(|| {
        for elt in &elements {
            black_box(indexing::algorithms::binary_search_by(&data,
                move |x| if x >= elt {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }));
        }
    });
}

#[bench]
fn indexing_lower_bound_same(b: &mut Bencher) {
    let mut data = test_data_max(N, MAX);
    let elements = [0, 1, 2, 7, 29, MAX/3, MAX/2, MAX];
    data.sort();
    b.iter(|| {
        for elt in &elements {
            black_box(indexing::algorithms::lower_bound(&data, elt));
        }
    });
}

#[bench]
fn indexing_lower_bound_many_duplicate(b: &mut Bencher) {
    let max = N as i32 / 5;
    let mut data = test_data_max(N, max);
    let elements = [0, 1, 2, 7, 29, max/3, max/2, max];
    data.sort();
    b.iter(|| {
        for elt in &elements {
            black_box(indexing::algorithms::lower_bound(&data, elt));
        }
    });
}

#[bench]
fn indexing_lower_bound_few_duplicate(b: &mut Bencher) {
    let max = N as i32 * 10;
    let mut data = test_data_max(N, max);
    let elements = [0, 1, 2, 7, 29, max/3, max/2, max];
    data.sort();
    b.iter(|| {
        for elt in &elements {
            black_box(indexing::algorithms::lower_bound(&data, elt));
        }
    });
}

#[bench]
fn indexing_lower_bound_few_duplicate_string(b: &mut Bencher) {
    let max = N as i32 * 10;
    let numeric_data = test_data_max(N, max);
    let mut data = Vec::from_iter(numeric_data.iter().map(<_>::to_string));
    let elements = Vec::from_iter([0, 1, 2, 7, 29, max/3, max/2, max].iter().map(<_>::to_string));
    data.sort();
    b.iter(|| {
        for elt in &elements {
            black_box(indexing::algorithms::lower_bound(&data, elt));
        }
    });
}
