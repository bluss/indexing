//!
//! Respository of some indexing-implemented algorithms so we can dissect them
//! and their codegen.
//!
//!

use std::fmt::Debug;

use super::indices;

/// Convenience trait -- debugging
pub trait Data : Ord + Debug { }
impl<T: Ord + Debug> Data for T { }

/// Simple quicksort implemented using `indexing`,
fn quicksort<T: Data>(v: &mut [T]) {
    // I think this is similar to Hoare’s version?
    indices(v, |mut v, range| {
        if let Ok(range) = range.nonempty() {

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

            println!("v={:?}, pivot={:?}, {:?}", &v[..], pivot, v[pivot]);

            // partition
            if let Ok(mut scan) = range.nonempty() {
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
