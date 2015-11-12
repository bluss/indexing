//!
//! Respository of some indexing-implemented algorithms so we can dissect them
//! and their codegen.
//!
//!

use std::fmt::{self, Debug};
use std::cmp;
use std::mem::swap;

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



// block swap between two different slices
/// Block swap to the shortest length of the two slices
fn block_swap2<T>(a: &mut [T], b: &mut [T]) {
    let count = cmp::min(a.len(), b.len());
    let a = &mut a[..count];
    let b = &mut b[..count];
    for i in 0..count {
        swap(&mut a[i], &mut b[i]);
    }
}

/// Merge internal: Merge inside data while using `buffer` as a swap space
///
/// `data` is in two sections, each part in sorted order, divided by `left_end`.
///
/// Merge the left and right half of data in place, while using buffer for swap space.
///
/// This is panic safe by ensuring all values are swapped and not duplicated,
/// so it is all present in either `data` or `buffer` at all times.
pub fn merge_internal_indices<T: Ord>(data: &mut [T], left_end: usize, buffer: &mut [T]) {
    debug_assert!(data.len() >= 1);
    if left_end > data.len() || left_end > buffer.len() {
        panic!("merge_internal: data or buffer too short");
    }
    indices(data, move |mut data, r| {
        indices(&mut buffer[..left_end], |mut buffer, rb| {
            let right_end = data.len();
            if right_end - left_end == 0 {
                return;
            }
            block_swap2(&mut data[r], &mut buffer[rb]);

            let mut i = match rb.contains(0) {
                Some(i) => i,
                None => return,
            };
            let mut out = match r.contains(0) {
                Some(i) => i,
                None => return,
            };
            let mut j = match r.contains(left_end) {
                Some(i) => i,
                None => return,
            };
            loop {
                if buffer[i] <= data[j] {
                    swap(&mut buffer[i], &mut data[out]);
                    data.forward(&mut out);
                    if !buffer.forward(&mut i) { break; }
                } else {
                    data.swap(j, out);
                    data.forward(&mut out);
                    if !data.forward(&mut j) {
                        // block swap remainder
                        block_swap2(&mut buffer[i..], &mut data[out..]);
                        break;
                    }
                }
            }
        })
    })
}

/// Merge internal: Merge inside data while using `buffer` as a swap space
///
/// `data` is in two sections, each part in sorted order, divided by `left_end`.
///
/// Merge the left and right half of data in place, while using buffer for swap space.
///
/// This is panic safe by ensuring all values are swapped and not duplicated,
/// so it is all present in either `data` or `buffer` at all times.
pub fn merge_internal_ranges<T: Ord>(data: &mut [T], left_end: usize, buffer: &mut [T]) {
    debug_assert!(data.len() >= 1);
    if left_end > data.len() || left_end > buffer.len() {
        panic!("merge_internal: data or buffer too short");
    }
    indices(data, |mut data, r| {
        indices(&mut buffer[..left_end], |mut buffer, rb| {
            let mut i = match rb.nonempty() {
                Ok(r) => r,
                Err(_) => return,
            };
            let mut out = match r.nonempty() {
                Ok(r) => r,
                Err(_) => return,
            };
            let mut j = match r.split_at(left_end).1.nonempty() {
                Ok(r) => r,
                Err(_) => return,
            };

            block_swap2(&mut data[r], &mut buffer[rb]);
            loop {
                if buffer[i.first()] <= data[j.first()] {
                    swap(&mut buffer[i.first()], &mut data[out.first()]);
                    if !out.advance() { return; }
                    if !i.advance() { return; }
                } else {
                    data.swap(j.first(), out.first());
                    if !out.advance() { return; }
                    if !j.advance() {
                        // block swap remainder
                        block_swap2(&mut buffer[*i], &mut data[*out]);
                        break;
                    }
                }
            }
        })
    })
}

#[test]
fn test_merge_internal() {
    let mut buffer = [0; 128];
    let a = (0..15).collect::<Vec<_>>();
    let b = (1..25).filter(|&x| x % 2 == 0).collect::<Vec<_>>();
    // data to sort
    let data = [&a[..], &b[..]].concat();
    let mut ans = data.clone();
    ans.sort();

    {
        let mut workspace = data.clone();
        merge_internal_indices(&mut workspace, a.len(), &mut buffer);
        assert_eq!(workspace, ans);
        assert!(buffer.iter().all(|x| *x == 0));
    }
    {
        let mut workspace = data.clone();
        merge_internal_ranges(&mut workspace, a.len(), &mut buffer);
        assert_eq!(workspace, ans);
        assert!(buffer.iter().all(|x| *x == 0));
    }
}
