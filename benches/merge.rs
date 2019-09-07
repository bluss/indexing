#![feature(test)]

extern crate test;
extern crate indexing;

fn bench_data(data: &mut [i32]) {
    let len = data.len();
    for (index, elt) in data.iter_mut().enumerate() {
        *elt = ((index * 123) % len) as i32;
    }
}

macro_rules! bench_insertion_sort {
    ($($name:ident, $n:expr;)*) => {
        $(
        mod $name {
            use indexing::algorithms::*;
            use crate::bench_data;
            use test::Bencher;
            use std::mem;
            #[bench]
            fn indexes(b: &mut Bencher) {
                let mut data = [0; $n];
                bench_data(&mut data);

                b.iter(|| {
                    let mut d = data;
                    insertion_sort_indexes(&mut d, |a, b| a < b);
                });
                b.bytes = mem::size_of_val(&data) as u64;
            }

            #[bench]
            fn ranges(b: &mut Bencher) {
                let mut data = [0; $n];
                bench_data(&mut data);

                b.iter(|| {
                    let mut d = data;
                    insertion_sort_ranges(&mut d, |a, b| a < b);
                });
                b.bytes = mem::size_of_val(&data) as u64;
            }

            #[cfg(feature="experimental_pointer_ranges")]
            #[bench]
            fn ranges_lower_bound(b: &mut Bencher) {
                let mut data = [0; $n];
                bench_data(&mut data);

                b.iter(|| {
                    let mut d = data;
                    insertion_sort_prange_lower(&mut d, |a, b| a < b);
                });
                b.bytes = mem::size_of_val(&data) as u64;
            }

            #[cfg(feature="experimental_pointer_ranges")]
            #[bench]
            fn prange(b: &mut Bencher) {
                let mut data = [0; $n];
                bench_data(&mut data);

                b.iter(|| {
                    let mut d = data;
                    insertion_sort_pointerindex(&mut d, |a, b| a < b);
                });
                b.bytes = mem::size_of_val(&data) as u64;
            }

            #[bench]
            fn raw_ptr(b: &mut Bencher) {
                let mut data = [0; $n];
                bench_data(&mut data);

                b.iter(|| {
                    let mut d = data;
                    insertion_sort_rust(&mut d, |a, b| a < b);
                });
                b.bytes = mem::size_of_val(&data) as u64;
            }

            #[bench]
            fn libstd_sort(b: &mut Bencher) {
                let mut data = [0; $n];
                bench_data(&mut data);

                b.iter(|| {
                    let mut d = data;
                    d.sort();
                });
                b.bytes = mem::size_of_val(&data) as u64;
            }
        }
        )*
    }
}

bench_insertion_sort!(
    insertion_sort_004, 4;
    insertion_sort_010, 10;
    insertion_sort_016, 16;
    insertion_sort_032, 32;
    insertion_sort_050, 50;
    insertion_sort_100, 100;
    insertion_sort_300, 300;
    insertion_sort_700, 700;
);
