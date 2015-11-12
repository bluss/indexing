//!
//! Respository of some indexing-implemented algorithms so we can dissect them
//! and their codegen.
//!
//!

use std::fmt::{self, Debug};

use super::indices;

/// Convenience trait -- debugging
pub trait Data : Ord + Debug { }
impl<T: Ord + Debug> Data for T { }


// for debugging -- like println during debugging
#[cfg(debug_assertions)]
fn print(a: fmt::Arguments) {
    print!("{}\n", a);
}

#[cfg(debug_assertions)]
macro_rules! puts {
    ($($t:tt)*) => {
        print(format_args!($($t)*))
    }
}

#[cfg(not(debug_assertions))]
macro_rules! puts {
    ($($t:tt)*) => {
    }
}


/// Simple quicksort implemented using `indexing`,
pub fn quicksort<T: Data>(v: &mut [T]) {
    indices(v, |mut v, range| {
        if let Ok(range) = range.nonempty() {
            // Fall back to insertion short sections
            if range.len() <= 16 {
                insertion_sort_indexes(&mut v[..], |x, y| x < y);
                return;
            }

            let (r, m, l) = (range.first(), range.upper_middle(), range.last());
            // return, if the range is too short to sort
            if r == l {
                return;
            }
            // simple pivot
            // let pivot = m;
            //
            // smart pivot -- use median of three
            let pivot = if v[l] <= v[m] && v[m] <= v[r] {
                m
            } else if v[l] >= v[m] && v[l] <= v[r] {
                l
            } else {
                r
            };

            puts!("v={:?}, pivot={:?}, {:?}", &v[..], pivot, v[pivot]);

            // partition
            // I think this is similar to Hoare’s version, wikipedia
            let mut scan = range;
            'main: loop {
                if v[scan.first()] > v[pivot] {
                    loop {
                        if v[scan.last()] <= v[pivot] {
                            v.swap(scan.first(), scan.last());
                            break;
                        }
                        if !scan.advance_back() {
                            break 'main;
                        }
                    }
                }
                if !scan.advance() {
                    v.swap(pivot, scan.first());
                    break;
                }
            }

            // ok split at pivot location and recurse
            let (a, b) = v.split_at(scan.first());
            quicksort(&mut v[a]);
            quicksort(&mut v[b]);
        }
    });
}


#[test]
fn test_quicksort() {
    let mut data = [1, 0];
    quicksort(&mut data);
    assert_eq!(&data, &[0, 1]);

    let mut data = [1, 2, 2, 1, 3, 3, 2, 3];
    quicksort(&mut data);
    assert_eq!(&data, &[1, 1, 2, 2, 2, 3, 3, 3]);

    let mut data = [1, 4, 2, 0, 3];
    quicksort(&mut data);
    assert_eq!(&data, &[0, 1, 2, 3, 4]);

    let mut data = [4, 3, 2, 1, 0];
    quicksort(&mut data);
    assert_eq!(&data, &[0, 1, 2, 3, 4]);

    let mut data = [0, 1, 2, 3, 4];
    quicksort(&mut data);
    assert_eq!(&data, &[0, 1, 2, 3, 4]);

    let mut data = [0, 1, 5, 2, 3, 4];
    quicksort(&mut data);
    assert_eq!(&data, &[0, 1, 2, 3, 4, 5]);
}

pub fn insertion_sort_indexes<T, F>(v: &mut [T], mut less_than: F) where F: FnMut(&T, &T) -> bool {
    indices(v, move |mut v, r| {
        for i in r {
            let jtail = v.scan_tail(i, |j_elt| less_than(&v[i], j_elt));
            v.rotate1(jtail);
        }
    });
}

pub fn insertion_sort_ranges<T, F>(v: &mut [T], mut less_than: F) where F: FnMut(&T, &T) -> bool {
    indices(v, move |mut v, r| {
        if let Ok(mut i) = r.nonempty() {
            while i.advance() {
                let jtail = v.scan_tail(i.first(), |j_elt| less_than(&v[i.first()], j_elt));
                v.rotate1(jtail);
            }
        }
    });
}
