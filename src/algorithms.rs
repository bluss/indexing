//!
//! Respository of some indexing-implemented algorithms so we can dissect them
//! and their codegen.
//!
//!

use std::cmp::{self, Ordering};
use std::mem::swap;

use scope;
use pointer::zip;


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

// limit for switching to insertion sort
const QS_INSERTION_SORT_THRESH: usize = 24;

/// Simple quicksort implemented using `Range`,
pub fn quicksort_range<T: Ord>(v: &mut [T]) {
    scope(v, |mut v| {
        let range = v.range();
        if let Ok(range) = range.nonempty() {
            // Fall back to insertion sort for short sections
            if range.len() <= QS_INSERTION_SORT_THRESH {
                insertion_sort_ranges(&mut v[..], |x, y| x < y);
                return;
            }

            let (l, m, r) = (range.first(), range.upper_middle(), range.last());

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
            let mut scan = range;
            v.swap(scan.first(), pivot);
            pivot = scan.first();
            'main: loop {
                if v[scan.first()] >= v[pivot] {
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
                    break;
                }
            }

            // ok split at pivot location and recurse
            let (a, b) = v.split_at(scan.first());
            //puts!("a={:?}, pivot={:?}, b={:?}", &v[a], &v[scan.first()], &v[b]);
            quicksort_range(&mut v[a]);
            quicksort_range(&mut v[b]);
        }
    });
}

/// Simple quicksort implemented using `indexing`’s PRange.
pub fn quicksort_prange<T: Ord>(v: &mut [T]) {
    scope(v, |mut v| {
        let range = v.pointer_range();
        if let Ok(range) = range.nonempty() {
            // Fall back to insertion sort for short sections
            if range.len() <= QS_INSERTION_SORT_THRESH {
                insertion_sort_ranges(&mut v[..], |x, y| x < y);
                return;
            }

            let (l, m, r) = (range.first(), range.upper_middle(), range.last());

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
            let mut scan = range;
            v.swap_ptr(scan.first(), pivot);
            pivot = scan.first();
            'main: loop {
                if v[scan.first()] >= v[pivot] {
                    loop {
                        if v[scan.last()] <= v[pivot] {
                            v.swap_ptr(scan.first(), scan.last());
                            break;
                        }
                        if !scan.advance_back() {
                            break 'main;
                        }
                    }
                }
                if !scan.advance() {
                    break;
                }
            }

            // ok split at pivot location and recurse
            let (a, b) = v.split_at_pointer(scan.first());
            //puts!("a={:?}, pivot={:?}, b={:?}", &v[a], &v[scan.first()], &v[b]);
            quicksort_prange(&mut v[a]);
            quicksort_prange(&mut v[b]);
        }
    });
}


/// quicksort implemented using regular bounds checked indexing
pub fn quicksort_bounds<T: Ord>(v: &mut [T]) {
    // Fall back to insertion sort for short sections
    if v.len() <= QS_INSERTION_SORT_THRESH {
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
    v.swap(0, pivot);
    pivot = 0;
    let mut i = 0;
    let mut j = v.len() - 1;
    'main: loop {
        if v[i] >= v[pivot] {
            loop {
                if v[j] <= v[pivot] {
                    v.swap(i, j);
                    break;
                }
                j -= 1;
                if i >= j { break 'main; }
            }
        }
        if i >= j {
            break;
        }
        i += 1;
    }

    // ok split at pivot location and recurse
    let (a, b) = v.split_at_mut(i);
    quicksort_bounds(a);
    quicksort_bounds(b);
}

pub fn zip_dot_i32(xs: &[i32], ys: &[i32]) -> i32 {
    xs.iter().zip(ys).map(|(x, y)| x * y).sum()
}

pub fn zip_dot_i32_prange(xs: &[i32], ys: &[i32]) -> i32 {
    scope(xs, move |v| {
        scope(ys, move |u| {
            let mut sum = 0;
            zip(v.pointer_range(), &v,
                u.pointer_range(), &u,
                |&x, &y| sum += x * y);
            sum
        })
    })
}

pub fn copy<T: Copy>(xs: &[T], ys: &mut [T]) {
    for (&x, y) in xs.iter().zip(ys) {
        *y = x;
    }
}

pub fn copy_prange<T: Copy>(xs: &[T], ys: &mut [T]) {
    scope(xs, move |v| {
        scope(ys, move |mut u| {
            zip(v.pointer_range(), &v,
                u.pointer_range(), &mut u,
                |&x, y| *y = x);
        })
    })
}


#[test]
fn test_quicksort() {
    let mut data = [1, 0];
    quicksort_range(&mut data);
    assert_eq!(&data, &[0, 1]);

    let mut data = [1, 2, 2, 1, 3, 3, 2, 3];
    quicksort_range(&mut data);
    assert_eq!(&data, &[1, 1, 2, 2, 2, 3, 3, 3]);

    let mut data = [1, 4, 2, 0, 3];
    quicksort_range(&mut data);
    assert_eq!(&data, &[0, 1, 2, 3, 4]);

    let mut data = [4, 3, 2, 1, 0];
    quicksort_range(&mut data);
    assert_eq!(&data, &[0, 1, 2, 3, 4]);

    let mut data = [0, 1, 2, 3, 4];
    quicksort_range(&mut data);
    assert_eq!(&data, &[0, 1, 2, 3, 4]);

    let mut data = [0, 1, 5, 2, 3, 4];
    quicksort_range(&mut data);
    assert_eq!(&data, &[0, 1, 2, 3, 4, 5]);

    let mut data = [0, 1, 1, -1, 0, -1];
    quicksort_range(&mut data);
    assert_eq!(&data, &[-1, -1, 0, 0, 1, 1]);
}

pub fn insertion_sort_indexes<T, F>(v: &mut [T], mut less_than: F) where F: FnMut(&T, &T) -> bool {
    scope(v, move |mut v| {
        for i in v.range() {
            let jtail = v.scan_from_rev(i, |j_elt| less_than(&v[i], j_elt));
            v.rotate1_up(jtail);
        }
    });
}

pub fn insertion_sort_ranges<T, F>(v: &mut [T], mut less_than: F) where F: FnMut(&T, &T) -> bool {
    scope(v, move |mut v| {
        if let Ok(mut i) = v.range().nonempty() {
            while i.advance() {
                let jtail = v.scan_from_rev(i.first(), |j_elt| less_than(&v[i.first()], j_elt));
                v.rotate1_up(jtail);
            }
        }
    });
}

/// Insertion sort using lower_bound to find the place to insert; which
/// makes it scale better (still restricted to just a smallish number of
/// elements).
pub fn insertion_sort_prange_lower<T, F>(v: &mut [T], mut less_than: F)
    where F: FnMut(&T, &T) -> bool,
{
    scope(v, |mut v| {
        for i in v.pointer_range() {
            let up_to = v.pointer_range_of(..i);
            let lb = lower_bound_prange_(up_to, &v, |x| less_than(x, &v[i]));
            // FIXME: There's a less than check here (`lb < i.after()`).
            if let Ok(lb_range) = v.nonempty_range(lb, i.after()) {
                v.rotate1_prange(lb_range);
            }
        }
    });
}

pub fn insertion_sort_pointerindex<T, F>(v: &mut [T], mut less_than: F) where F: FnMut(&T, &T) -> bool {
    scope(v, move |mut v| {
        for i in v.pointer_range() {
            let jtail = v.scan_tail_(i, |j_elt| less_than(&v[i], j_elt));
            v.rotate1_prange(jtail);
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

#[derive(Clone, Debug)]
pub struct AlgorithmError(pub &'static str);

impl From<&'static str> for AlgorithmError {
    fn from(s: &'static str) -> Self { AlgorithmError(s) }
}

/// Merge internal: Merge inside data while using `buffer` as a swap space
///
/// `data` is in two sections, each part in sorted order, divided by `left_end`.
///
/// Merge the left and right half of data in place, while using buffer for swap space.
///
/// This is panic safe by ensuring all values are swapped and not duplicated,
/// so it is all present in either `data` or `buffer` at all times.
pub fn merge_internal_indices<T: Ord>(data: &mut [T], left_end: usize, buffer: &mut [T])
    -> Result<(), AlgorithmError>
{
    debug_assert!(data.len() >= 1);
    if left_end > data.len() || left_end > buffer.len() {
        try!(Err("merge_internal: data or buffer too short"));
    }
    scope(data, move |mut data| {
        let r = data.range();
        scope(&mut buffer[..left_end], |mut buffer| {
            let rb = buffer.range();
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
    });
    Ok(())
}

/// Merge internal: Merge inside data while using `buffer` as a swap space
///
/// `data` is in two sections, each part in sorted order, divided by `left_end`.
///
/// Merge the left and right half of data in place, while using buffer for swap space.
///
/// This is panic safe by ensuring all values are swapped and not duplicated,
/// so it is all present in either `data` or `buffer` at all times.
pub fn merge_internal_ranges<T: Ord>(data: &mut [T], left_end: usize, buffer: &mut [T])
    -> Result<(), AlgorithmError>
{
    debug_assert!(data.len() >= 1);
    if left_end > data.len() || left_end > buffer.len() {
        try!(Err("merge_internal: data or buffer too short"));
    }
    scope(data, |mut data| {
        let r = data.range();
        scope(&mut buffer[..left_end], |mut buffer| {
            let rb = buffer.range();
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
    });
    Ok(())
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

pub fn heapify<T: Ord>(v: &mut [T]) {
    scope(v, |mut v| {
        // for 0-indexed element k, children are:
        // 2k + 1, 2k + 2
        let (left, _right) = v.range().split_in_half();
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
                if v[pos] <= v[child] {
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

pub fn binary_search<T: Ord>(v: &[T], elt: &T) -> Result<usize, usize> {
    binary_search_by(v, |x| x.cmp(elt))
}

/// `f` is a closure that is passed `x` from the slice and should return the
/// result of `x` compared with *something*.
pub fn binary_search_by<T, F>(v: &[T], mut f: F) -> Result<usize, usize>
    where F: FnMut(&T) -> Ordering,
{
    scope(v, move |v| {
        let mut range = v.range();
        loop {
            /* NOTE: This is sometimes a benefit. But how do we do this cleanly?
            if range.len() < 4 {
                for i in range {
                    match f(&v[i]) {
                        Ordering::Equal => return Ok(i.integer()),
                        Ordering::Greater => return Err(i.integer()),
                        Ordering::Less => { }
                    }
                }
                return Err(range.end());
            }
            */
            let (a, b) = range.split_in_half();
            if let Ok(b_) = b.nonempty() {
                let mid = b_.first();
                match f(&v[mid]) {
                    Ordering::Equal => return Ok(mid.integer()),
                    Ordering::Greater => range = a,
                    Ordering::Less => range = b_.tail(),
                }
            } else {
                break;
            }
        }
        Err(range.start())
    })
}

pub fn binary_search_by_prange<'id, T, F>(v: &[T], compare: F)
    -> Result<usize, usize>
    where F: FnMut(&T) -> Ordering,
{
    scope(v, move |v| {
        match binary_search_by_prange_(v.pointer_range(), &v, compare) {
            Ok(p) => Ok(v.distance_to(p)),
            Err(p) => Err(v.distance_to(p)),
        }
    })
}

/// Binary search using comparison `compare`.
///
/// Return a valid trusted pointer to the element if it is found, otherwise
/// return an edge pointer to where the item could be inserted.
///
/// `compare` is a closure that is passed `x` from the slice and should return
/// the result of `x` compared with the element whose position is sought.
pub fn binary_search_by_prange_<'id, T, P, Array, F>(range: PRange<'id, T, P>,
                                                     v: &Container<'id, Array>,
                                                     mut compare: F)
    -> Result<PIndex<'id, T>, PIndex<'id, T, Unknown>>
    where F: FnMut(&T) -> Ordering,
          Array: Contiguous<Item=T>,
{
    let mut range = range.no_proof();
    loop {
        let (a, b) = range.split_in_half();
        if let Ok(b_) = b.nonempty() {
            let mid = b_.first();
            match compare(&v[mid]) {
                Ordering::Equal => return Ok(mid),
                Ordering::Greater => range = a,
                Ordering::Less => range = b_.tail(),
            }
        } else {
            break;
        }
    }
    Err(range.first())
}

pub fn binary_search_by_pslice<'id, T, F>(v: &[T], compare: F)
    -> Result<usize, usize>
    where F: FnMut(&T) -> Ordering,
{
    scope(v, move |v| {
        match binary_search_by_pslice_(v.pointer_slice(), &v, compare) {
            Ok(p) => Ok(v.distance_to(p)),
            Err(p) => Err(v.distance_to(p)),
        }
    })
}

/// Binary search using comparison `compare`.
///
/// Return a valid trusted pointer to the element if it is found, otherwise
/// return an edge pointer to where the item could be inserted.
///
/// `compare` is a closure that is passed `x` from the slice and should return
/// the result of `x` compared with the element whose position is sought.
pub fn binary_search_by_pslice_<'id, T, P, Array, F>(range: PSlice<'id, T, P>,
                                                     v: &Container<'id, Array>,
                                                     mut compare: F)
    -> Result<PIndex<'id, T>, PIndex<'id, T, Unknown>>
    where F: FnMut(&T) -> Ordering,
          Array: Contiguous<Item=T>,
{
    let mut range = range.no_proof();
    loop {
        let (a, b) = range.split_in_half();
        if let Ok(b_) = b.nonempty() {
            let mid = b_.first();
            match compare(&v[mid]) {
                Ordering::Equal => return Ok(mid),
                Ordering::Greater => range = a,
                Ordering::Less => range = b_.tail(),
            }
        } else {
            break;
        }
    }
    Err(range.first())
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

//#[inline(never)]
pub fn lower_bound<T: PartialOrd>(v: &[T], elt: &T) -> usize {
    scope(v, move |v| {
        let mut range = v.range();
        while let Ok(range_) = range.nonempty() {
            let (a, b) = range_.split_in_half();
            if v[b.first()] < *elt {
                range = b.tail();
            } else {
                range = a;
            }
        }
        range.start()
    })
}

/// Using PRange (pointer-based safe API)
pub fn lower_bound_prange<T: PartialOrd>(v: &[T], elt: &T) -> usize {
    scope(v, move |v| {
        let mut range = v.pointer_range();
        while let Ok(range_) = range.nonempty() {
            let (a, b) = range_.split_in_half();
            if v[b.first()] < *elt {
                range = b.tail();
            } else {
                range = a;
            }
        }
        v.distance_to(range.first())
    })
}


use Container;
use container_traits::Contiguous;
use pointer::{PIndex, PRange, PSlice};
use Unknown;
use proof::Provable;

pub fn lower_bound_prange_<'id, T, P, Array, F>(range: PRange<'id, T, P>,
                                                v: &Container<'id, Array>,
                                                mut less_than: F)
    -> PIndex<'id, T, Unknown>
    where Array: Contiguous<Item=T>,
          F: FnMut(&T) -> bool,
{
    let mut range = range.no_proof();
    while let Ok(range_) = range.nonempty() {
        let (a, b) = range_.split_in_half();
        if less_than(&v[b.first()]) {
            range = b.tail();
        } else {
            range = a;
        }
    }
    range.first()
}

pub fn lower_bound_pslice_<'id, T, P, Array, F>(range: PSlice<'id, T, P>,
                                                v: &Container<'id, Array>,
                                                mut less_than: F)
    -> PIndex<'id, T, Unknown>
    where Array: Contiguous<Item=T>,
          F: FnMut(&T) -> bool,
{
    let mut range = range.no_proof();
    while let Ok(range_) = range.nonempty() {
        let (a, b) = range_.split_in_half();
        if less_than(&v[b.first()]) {
            range = b.tail();
        } else {
            range = a;
        }
    }
    range.first()
}

/// Using PSlice (pointer-based safe API)
pub fn lower_bound_pslice<T, F>(v: &[T], f: F) -> usize
    where F: FnMut(&T) -> bool,
{
    scope(v, move |v| {
        let range = v.pointer_slice();
        v.distance_to(lower_bound_pslice_(range, &v, f))
    })
}

/// Raw pointer version, for comparison
/// From http://en.cppreference.com/w/cpp/algorithm/lower_bound
pub fn lower_bound_raw_ptr<T: PartialOrd>(v: &[T], elt: &T) -> usize {
    unsafe {
        let mut start = v.as_ptr();
        let end = start.offset(v.len() as isize);
        let mut count = ptrdistance(start, end);
        while count > 0 {
            let step = count / 2;
            let it = start.offset(step as isize);
            if *it < *elt {
                start = it.offset(1);
                count -= step + 1;
            } else {
                count = step;
            }
        }
        ptrdistance(v.as_ptr(), start)
    }
}

/// return the number of steps between a and b
fn ptrdistance<T>(from: *const T, to: *const T) -> usize {
    use std::mem;
    (to as usize - from as usize) / mem::size_of::<T>()
}


#[test]
fn test_lower_bound() {
    let data = [3, 7, 8, 8, 8, 11, 11, 11, 15, 22, 22, 26];
    assert_eq!(lower_bound(&data, &8), 2);
    assert_eq!(lower_bound(&data, &7), 1);

    let elts = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 25, 26, 27, 28];

    for elt in &elts {
        assert_eq!(lower_bound(&data, elt),
            data.binary_search_by(|x| if x >= elt {
                Ordering::Greater
            } else {
                Ordering::Less
            }).unwrap_err());
    }
}

#[test]
fn test_lower_bound_ptr() {
    let data = [3, 7, 8, 8, 8, 11, 11, 11, 15, 22, 22, 26];
    assert_eq!(lower_bound_raw_ptr(&data, &8), 2);
    assert_eq!(lower_bound_raw_ptr(&data, &7), 1);

    let elts = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 25, 26, 27, 28];

    for elt in &elts {
        assert_eq!(lower_bound_raw_ptr(&data, elt),
            data.binary_search_by(|x| if x >= elt {
                Ordering::Greater
            } else {
                Ordering::Less
            }).unwrap_err());
    }
}

#[test]
fn test_lower_bound_pointer() {
    let data = [3, 7, 8, 8, 8, 11, 11, 11, 15, 22, 22, 26];
    assert_eq!(lower_bound_prange(&data, &8), 2);
    assert_eq!(lower_bound_prange(&data, &7), 1);

    let elts = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 25, 26, 27, 28];

    for elt in &elts {
        assert_eq!(lower_bound_prange(&data, elt),
            data.binary_search_by(|x| if x >= elt {
                Ordering::Greater
            } else {
                Ordering::Less
            }).unwrap_err());
    }
}
