#![feature(test)]

extern crate test;
extern crate rand;

extern crate indexing;

use std::cmp::Ordering;
use std::iter::FromIterator;

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use test::Bencher;
use test::black_box;

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
const SHORT_LEN_MIN: usize = 2;
const SHORT_LEN_MAX: usize = 32;

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
fn bench_binary_search_std(b: &mut Bencher) {
    let mut data = test_data_max(N, MAX);
    let elements = [0, 1, 2, 7, 29, MAX/3, MAX/2, MAX];
    data.sort();
    b.iter(|| {
        for elt in &elements {
            let _ = black_box(data.binary_search(&elt));
        }
    });
}

#[bench]
fn bench_binary_search_std_unchecked(b: &mut Bencher) {
    let mut data = test_data_max(N, MAX);
    let elements = [0, 1, 2, 7, 29, MAX/3, MAX/2, MAX];
    data.sort();
    b.iter(|| {
        for elt in &elements {
            let _ = black_box(core_binary_search_by(&data, move |x| x.cmp(elt)));
        }
    });
}

#[bench]
fn bench_binary_search_range(b: &mut Bencher) {
    let mut data = test_data_max(N, MAX);
    let elements = [0, 1, 2, 7, 29, MAX/3, MAX/2, MAX];
    data.sort();
    b.iter(|| {
        for elt in &elements {
            let _ = black_box(binary_search(&data, elt));
        }
    });
}

#[cfg(feature="experimental_pointer_ranges")]
#[bench]
fn bench_binary_search_prange(b: &mut Bencher) {
    let mut data = test_data_max(N, MAX);
    let elements = [0, 1, 2, 7, 29, MAX/3, MAX/2, MAX];
    data.sort();
    b.iter(|| {
        for elt in &elements {
            let _ = black_box(binary_search_by_prange(&data, |x| x.cmp(elt)));
        }
    });
}

#[cfg(feature="experimental_pointer_ranges")]
#[bench]
fn bench_binary_search_pslice(b: &mut Bencher) {
    let mut data = test_data_max(N, MAX);
    let elements = [0, 1, 2, 7, 29, MAX/3, MAX/2, MAX];
    data.sort();
    b.iter(|| {
        for elt in &elements {
            let _ = black_box(binary_search_by_pslice(&data, move |x| x.cmp(elt)));
        }
    });
}

fn get(r: Result<usize, usize>) -> usize {
    match r {
        Ok(x) => x,
        Err(x) => x,
    }
}

#[bench]
fn std_binary_search_short(b: &mut Bencher) {
    let mut data = test_data_max(N, MAX);
    let elements = [0, 1, 2, 7, 29, MAX/3, MAX/2, MAX];
    data.sort();
    b.iter(|| {
        let mut sum = 0;
        for chunk_sz in SHORT_LEN_MIN..SHORT_LEN_MAX {
            for elt in &elements {
                for chunk in data.chunks(chunk_sz) {
                    sum += get(chunk.binary_search(elt));
                }
            }
        }
        sum
    });
}

#[bench]
fn indexing_binary_search_short(b: &mut Bencher) {
    let mut data = test_data_max(N, MAX);
    let elements = [0, 1, 2, 7, 29, MAX/3, MAX/2, MAX];
    data.sort();
    b.iter(|| {
        let mut sum = 0;
        for chunk_sz in SHORT_LEN_MIN..SHORT_LEN_MAX {
            for elt in &elements {
                for chunk in data.chunks(chunk_sz) {
                    sum += get(binary_search(chunk, elt));
                }
            }
        }
        sum
    });
}

#[bench]
fn indexing_lower_bound_fake(b: &mut Bencher) {
    let mut data = test_data_max(N, MAX);
    let elements = [0, 1, 2, 7, 29, MAX/3, MAX/2, MAX];
    data.sort();
    b.iter(|| {
        for elt in &elements {
            let _ = black_box(indexing::algorithms::binary_search_by(&data,
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
fn indexing_lower_bound_many_duplicate_raw_ptr(b: &mut Bencher) {
    let max = N as i32 / 5;
    let mut data = test_data_max(N, max);
    let elements = [0, 1, 2, 7, 29, max/3, max/2, max];
    data.sort();
    b.iter(|| {
        for elt in &elements {
            black_box(lower_bound_raw_ptr(&data, elt));
        }
    });
}

#[cfg(feature="experimental_pointer_ranges")]
#[bench]
fn indexing_lower_bound_many_duplicate_prange(b: &mut Bencher) {
    let max = N as i32 / 5;
    let mut data = test_data_max(N, max);
    let elements = [0, 1, 2, 7, 29, max/3, max/2, max];
    data.sort();
    b.iter(|| {
        for elt in &elements {
            black_box(lower_bound_prange(&data, elt));
        }
    });
}

#[bench]
fn indexing_lower_bound_few_duplicate_raw_ptr(b: &mut Bencher) {
    let max = N as i32 * 10;
    let mut data = test_data_max(N, max);
    let elements = [0, 1, 2, 7, 29, max/3, max/2, max];
    data.sort();
    b.iter(|| {
        for elt in &elements {
            black_box(lower_bound_raw_ptr(&data, elt));
        }
    });
}

#[cfg(feature="experimental_pointer_ranges")]
#[bench]
fn indexing_lower_bound_few_duplicate_prange(b: &mut Bencher) {
    let max = N as i32 * 10;
    let mut data = test_data_max(N, max);
    let elements = [0, 1, 2, 7, 29, max/3, max/2, max];
    data.sort();
    b.iter(|| {
        for elt in &elements {
            black_box(lower_bound_prange(&data, elt));
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

#[bench]
fn indexing_lower_bound_few_duplicate_string_raw_ptr(b: &mut Bencher) {
    let max = N as i32 * 10;
    let numeric_data = test_data_max(N, max);
    let mut data = Vec::from_iter(numeric_data.iter().map(<_>::to_string));
    let elements = Vec::from_iter([0, 1, 2, 7, 29, max/3, max/2, max].iter().map(<_>::to_string));
    data.sort();
    b.iter(|| {
        for elt in &elements {
            black_box(lower_bound_raw_ptr(&data, elt));
        }
    });
}

#[bench]
fn short_lower_bound_indexing(b: &mut Bencher) {
    let mut data = test_data_max(N, MAX);
    let elements = [0, 1, 2, 7, 29, MAX/3, MAX/2, MAX];
    data.sort();
    b.iter(|| {
        let mut sum = 0;
        for chunk_sz in SHORT_LEN_MIN..SHORT_LEN_MAX {
            for elt in &elements {
                for chunk in data.chunks(chunk_sz) {
                    sum += lower_bound(chunk, elt);
                }
            }
        }
        sum
    });
}

#[bench]
fn short_lower_bound_std(b: &mut Bencher) {
    let mut data = test_data_max(N, MAX);
    let elements = [0, 1, 2, 7, 29, MAX/3, MAX/2, MAX];
    data.sort();
    b.iter(|| {
        let mut sum = 0;
        for chunk_sz in SHORT_LEN_MIN..SHORT_LEN_MAX {
            for elt in &elements {
                for chunk in data.chunks(chunk_sz) {
                    sum += get(chunk.binary_search_by(
                        move |x| if x >= elt {
                            Ordering::Greater
                        } else {
                            Ordering::Less
                        }));
                }
            }
        }
        sum
    });
}

#[cfg(feature="experimental_pointer_ranges")]
#[bench]
fn short_lower_bound_prange(b: &mut Bencher) {
    let mut data = test_data_max(N, MAX);
    let elements = [0, 1, 2, 7, 29, MAX/3, MAX/2, MAX];
    data.sort();
    b.iter(|| {
        let mut sum = 0;
        for chunk_sz in SHORT_LEN_MIN..SHORT_LEN_MAX {
            for elt in &elements {
                for chunk in data.chunks(chunk_sz) {
                    sum += lower_bound_prange(chunk, elt);
                }
            }
        }
        sum
    });
}

#[cfg(feature="experimental_pointer_ranges")]
#[bench]
fn short_lower_bound_pslice(b: &mut Bencher) {
    let mut data = test_data_max(N, MAX);
    let elements = [0, 1, 2, 7, 29, MAX/3, MAX/2, MAX];
    data.sort();
    b.iter(|| {
        let mut sum = 0;
        for chunk_sz in SHORT_LEN_MIN..SHORT_LEN_MAX {
            for elt in &elements {
                for chunk in data.chunks(chunk_sz) {
                    sum += lower_bound_pslice(chunk, |x| *x < *elt);
                }
            }
        }
        sum
    });
}

#[bench]
fn short_lower_bound_raw_ptr(b: &mut Bencher) {
    let mut data = test_data_max(N, MAX);
    let elements = [0, 1, 2, 7, 29, MAX/3, MAX/2, MAX];
    data.sort();
    b.iter(|| {
        let mut sum = 0;
        for chunk_sz in SHORT_LEN_MIN..SHORT_LEN_MAX {
            for elt in &elements {
                for chunk in data.chunks(chunk_sz) {
                    sum += lower_bound_raw_ptr(chunk, elt);
                }
            }
        }
        sum
    });
}


unsafe fn split_at_unchecked<T>(data: &[T], i: usize) -> (&[T], &[T])
{
    use std::slice::from_raw_parts;
    debug_assert!(i <= data.len());
    let ptr = data.as_ptr();
    (from_raw_parts(ptr, i), from_raw_parts(ptr.offset(i as isize), data.len() - i))
}


#[inline(never)]
fn core_binary_search_by<'a, T, F>(data: &'a [T], mut compare: F) -> Result<usize, usize>
    where F: FnMut(&'a T) -> Ordering
{
    use std::cmp::Ordering::*;
    let mut s = data;
    let base = s.as_ptr();
    let mut steps = 0;

    loop {
        let (head, tail) = unsafe { split_at_unchecked(s, s.len() / 2) };
        //let (head, tail) = s.split_at(s.len() / 2);
        if tail.is_empty() {
            break;
        }
        let mid_element = &tail[0];
        match compare(mid_element) {
            Equal => {
                steps += head.len();
                return Ok(ptr_distance_to(base, mid_element).unwrap_or(steps));
            }
            Greater => s = head,
            Less => {
                s = &tail[1..];
                steps += head.len() + 1;
            }
        }
    }
    Err(ptr_distance_to(base, s.as_ptr()).unwrap_or(steps))
}

use std::mem;

/// return the number of steps in elements from a to b (b must be larger)
fn ptr_distance_to<T>(a: *const T, b: *const T) -> Option<usize> {
    if mem::size_of::<T>() == 0 {
        None
    } else {
        Some((b as usize - a as usize) / mem::size_of::<T>())
    }
}
