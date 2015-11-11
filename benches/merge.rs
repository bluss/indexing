#![feature(test)]

extern crate test;
extern crate indexing;

use test::Bencher;

use indexing::indices;
use std::cmp;
use std::mem::swap;

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
fn merge_internal_indices<T: Ord>(data: &mut [T], left_end: usize, buffer: &mut [T]) {
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
fn merge_internal_ranges<T: Ord>(data: &mut [T], left_end: usize, buffer: &mut [T]) {
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

fn main() { }
