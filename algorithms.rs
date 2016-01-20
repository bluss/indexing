//!
//! Respository of some indexing-implemented algorithms so we can dissect them
//! and their codegen.
//!
//!

use std::fmt::{Debug};
use std::cmp::{self, Ordering};
use std::mem::swap;

use super::indices;

/// Convenience trait -- debugging
pub trait Data : Ord + Debug { }
impl<T: Ord + Debug> Data for T { }


// for debugging -- like println during debugging
#[cfg(debug_assertions)]
macro_rules! puts {
    ($($t:tt)*) => {
        println!($($t)*)
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
            // Fall back to insertion sort for short sections
            if range.len() <= 16 {
                insertion_sort_ranges(&mut v[..], |x, y| x < y);
                return;
            }

            let (l, m, r) = (range.first(), range.upper_middle(), range.last());
            // return, if the range is too short to sort
            if r == l {
                return;
            }
            // simple pivot
            // let pivot = m;
            //
            // smart pivot -- use median of three
            let mut pivot = if v[l] <= v[m] && v[m] <= v[r] {
                m
            } else if v[m] <= v[l] && v[l] <= v[r] {
                l
            } else {
                r
            };

            // partition
            // I think this is similar to Hoare’s version, wikipedia
            let mut scan = range;
            'main: loop {
                if v[scan.first()] > v[pivot] {
                    loop {
                        if v[scan.last()] <= v[pivot] {
                            v.swap(scan.first(), scan.last());
                            if scan.last() == pivot {
                                pivot = scan.first();
                            }
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
            //puts!("a={:?}, pivot={:?}, b={:?}", &v[a], &v[scan.first()], &v[b]);
            quicksort(&mut v[a]);
            quicksort(&mut v[b]);
        }
    });
}

/// quicksort implemented using regular bounds checked indexing
pub fn quicksort_bounds<T: Data>(v: &mut [T]) {
    // Fall back to insertion sort for short sections
    if v.len() <= 16 {
        insertion_sort_ranges(&mut v[..], |x, y| x < y);
        return;
    }

    let (l, m, r) = (0, v.len() / 2, v.len() - 1);
    let mut pivot = if v[l] <= v[m] && v[m] <= v[r] {
        m
    } else if v[m] <= v[l] && v[l] <= v[r] {
        l
    } else {
        r
    };

    // partition
    // I think this is similar to Hoare’s version, wikipedia
    let mut i = 0;
    let mut j = v.len() - 1;
    'main: loop {
        if v[i] > v[pivot] {
            loop {
                if v[j] <= v[pivot] {
                    v.swap(i, j);
                    if j == pivot {
                        pivot = i;
                    }
                    break;
                }
                j -= 1;
                if i >= j { break 'main; }
            }
        }
        if i >= j {
            v.swap(pivot, i);
            break;
        }
        i += 1;
    }

    // ok split at pivot location and recurse
    let (a, b) = v.split_at_mut(i);
    quicksort_bounds(a);
    quicksort_bounds(b);
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

    let mut data = [0, 1, 1, -1, 0, -1];
    quicksort(&mut data);
    assert_eq!(&data, &[-1, -1, 0, 0, 1, 1]);
}

pub fn insertion_sort_indexes<T, F>(v: &mut [T], mut less_than: F) where F: FnMut(&T, &T) -> bool {
    indices(v, move |mut v, r| {
        for i in r {
            let jtail = v.scan_from_rev(i, |j_elt| less_than(&v[i], j_elt));
            v.rotate1_up(jtail);
        }
    });
}

pub fn insertion_sort_ranges<T, F>(v: &mut [T], mut less_than: F) where F: FnMut(&T, &T) -> bool {
    indices(v, move |mut v, r| {
        if let Ok(mut i) = r.nonempty() {
            while i.advance() {
                let jtail = v.scan_from_rev(i.first(), |j_elt| less_than(&v[i.first()], j_elt));
                v.rotate1_up(jtail);
            }
        }
    });
}

pub fn insertion_sort_pointerindex<T, F>(v: &mut [T], mut less_than: F) where F: FnMut(&T, &T) -> bool {
    indices(v, move |mut v, _r| {
        for i in v.pointer_range() {
            let jtail = v.scan_tail_(i, |j_elt| less_than(&v[i], j_elt));
            v.rotate1_(jtail);
        }
    });
}

/// Copied from rust / libcollections/slice.rs, using raw pointers
pub fn insertion_sort_rust<T, F>(v: &mut [T], mut less_than: F) where F: FnMut(&T, &T) -> bool {
    use std::mem;
    use std::ptr;
    let len = v.len() as isize;
    let buf_v = v.as_mut_ptr();

    // 1 <= i < len;
    for i in 1..len {
        // j satisfies: 0 <= j <= i;
        let mut j = i;
        unsafe {
            // `i` is in bounds.
            let read_ptr = buf_v.offset(i) as *const T;

            // find where to insert, we need to do strict <,
            // rather than <=, to maintain stability.

            // 0 <= j - 1 < len, so .offset(j - 1) is in bounds.
            while j > 0 && less_than(&*read_ptr, &*buf_v.offset(j - 1)) {
                j -= 1;
            }

            // shift everything to the right, to make space to
            // insert this value.

            // j + 1 could be `len` (for the last `i`), but in
            // that case, `i == j` so we don't copy. The
            // `.offset(j)` is always in bounds.

            if i != j {
                let tmp = ptr::read(read_ptr);
                ptr::copy(&*buf_v.offset(j),
                          buf_v.offset(j + 1),
                          (i - j) as usize);
                ptr::copy_nonoverlapping(&tmp, buf_v.offset(j), 1);
                mem::forget(tmp);
            }
        }
    }
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
                        block_swap2(&mut buffer[i], &mut data[out]);
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

pub fn heapify<T: Data>(v: &mut [T]) {
    indices(v, |mut v, range| {
        // for 0-indexed element k, children are:
        // 2k + 1, 2k + 2
        let (left, _right) = range.split_in_half();
        for i in left.into_iter().rev() {
            // Sift down element at `i`.
            let mut pos = i;
            while let Ok(mut child) = v.vet(pos.integer() * 2 + 1) {
                // pick the smaller of the two children
                let mut right = child;
                if v.forward(&mut right) && v[child] > v[right] {
                    child = right;
                }
                // sift down is done if we are already in order
                if v[pos] < v[child] {
                    break;
                }
                //puts!("mov {:?} => {:?} (value={:?})", pos, child, &v[pos]);
                v.swap(pos, child);
                pos = child;
            }
        }
    });
}

#[test]
fn test_heapify() {
    let mut data = [8, 12, 9, 7, 22, 3, 26, 14, 11, 15, 22];
    let heap = [3, 7, 8, 11, 15, 9, 26, 14, 12, 22, 22];
    heapify(&mut data);
    assert_eq!(&data, &heap);
}

    // Sift up
    // Now sift up start..pos
        /*
    while hole.pos() > start {
        let parent = (hole.pos() - 1) / 2;
        if hole.removed() <= hole.get(parent) { break }
        hole.move_to(parent);
    }
    */
    /*
    while pos > start {
        let parent_index = (pos.index - 1) / 2;
        let parent = v.vet(parent_index).unwrap();
        if v[pos] > v[parent] { break; }
        // mov
        puts!("mov {:?} => {:?} (value={:?})", pos, parent, &v[pos]);
        v.swap(pos, parent);
        pos = parent;
        puts!("sup {:?}", &v[..]);
    }
    */

pub fn binary_search<T: Data>(v: &[T], elt: &T) -> Result<usize, usize> {
    indices(v, move |v, mut range| {
        while let Ok(r) = range.nonempty() {
            let (fst, ix) = r.split_in_half();
            match v[ix.first()].cmp(elt) {
                Ordering::Equal => return Ok(ix.first().integer()),
                Ordering::Greater => {
                    range = fst;
                }
                Ordering::Less => {
                    range = ix.tail();
                }
            }
        }
        Err(range.as_range().start)
    })
}

#[test]
fn test_binary_search() {
    let data = [3, 7, 8, 11, 15, 22, 26];
    assert_eq!(binary_search(&data, &3), Ok(0));
    assert_eq!(binary_search(&data, &2), Err(0));

    let elts = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 25, 26, 27, 28];

    for elt in &elts {
        assert_eq!(binary_search(&data, elt), data.binary_search(elt));
    }
}
