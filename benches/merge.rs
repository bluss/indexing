#![feature(test)]

extern crate test;
extern crate indexing;

use indexing::algorithms::*;

use test::Bencher;

use std::mem;

#[cfg(test)]
fn bench_data(data: &mut [i32]) {
    let len = data.len();
    for (index, elt) in data.iter_mut().enumerate() {
        *elt = ((index * 123) % len) as i32;
    }
}

#[bench]
fn bench_insertion_sort_1024(b: &mut Bencher) {
    let mut data = [0; 1024];
    bench_data(&mut data);

    b.iter(|| {
        let mut d = data;
        insertion_sort_indexes(&mut d, |a, b| a < b);
    });
    b.bytes = mem::size_of_val(&data) as u64;
}

#[bench]
fn bench_insertion_sort_100(b: &mut Bencher) {
    let mut data = [0; 100];
    bench_data(&mut data);

    b.iter(|| {
        let mut d = data;
        insertion_sort_indexes(&mut d, |a, b| a < b);
    });
    b.bytes = mem::size_of_val(&data) as u64;
}

#[bench]
fn bench_range_insertion_sort_1024(b: &mut Bencher) {
    let mut data = [0; 1024];
    bench_data(&mut data);

    b.iter(|| {
        let mut d = data;
        insertion_sort_ranges(&mut d, |a, b| a < b);
    });
    b.bytes = mem::size_of_val(&data) as u64;
}

#[bench]
fn bench_range_insertion_sort_100(b: &mut Bencher) {
    let mut data = [0; 100];
    bench_data(&mut data);

    b.iter(|| {
        let mut d = data;
        insertion_sort_ranges(&mut d, |a, b| a < b);
    });
    b.bytes = mem::size_of_val(&data) as u64;
}

#[bench]
fn bench_pointer_insertion_sort_1024(b: &mut Bencher) {
    let mut data = [0; 1024];
    bench_data(&mut data);

    b.iter(|| {
        let mut d = data;
        insertion_sort_pointerindex(&mut d, |a, b| a < b);
    });
    b.bytes = mem::size_of_val(&data) as u64;
}

#[bench]
fn bench_pointer_insertion_sort_100(b: &mut Bencher) {
    let mut data = [0; 100];
    bench_data(&mut data);

    b.iter(|| {
        let mut d = data;
        insertion_sort_pointerindex(&mut d, |a, b| a < b);
    });
    b.bytes = mem::size_of_val(&data) as u64;
}



#[bench]
fn bench_insertion_sort_rust_1024(b: &mut Bencher) {
    let mut data = [0; 1024];
    bench_data(&mut data);

    b.iter(|| {
        let mut d = data;
        insertion_sort_rust(&mut d, |a, b| a < b);
    });
    b.bytes = mem::size_of_val(&data) as u64;
}

#[bench]
fn bench_insertion_sort_rust_100(b: &mut Bencher) {
    let mut data = [0; 100];
    bench_data(&mut data);

    b.iter(|| {
        let mut d = data;
        insertion_sort_rust(&mut d, |a, b| a < b);
    });
    b.bytes = mem::size_of_val(&data) as u64;
}

