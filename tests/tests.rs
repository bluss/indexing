extern crate indexing;
#[macro_use]
extern crate quickcheck;

use indexing::indices;
use indexing::algorithms::*;

use std::cmp::Ordering;


#[test]
fn join_add_proof() {
    let data = [1, 2, 3];
    indices(&data[..], move |_, r| {
        if let Ok(r) = r.nonempty() {
            let (front, back) = r.frontiers();

            r.first();
            // test nonempty range methods
            front.join(r).unwrap().first();
            r.join(back).unwrap().first();
            front.join_cover(r).first();
            r.join_cover(back).first();
            r.join_cover(r).first();

            assert_eq!(front.join(r).unwrap(), r);
            assert_eq!(front.join_cover(back), r);
            assert_eq!(back.join_cover(front), back);

            let (a, b) = r.split_in_half();
            assert_eq!(a.join(b), Ok(r));
            assert_eq!(a.join_cover(back), r);
            assert_eq!(front.join_cover(a), a);
            assert_eq!(front.join_cover(b), r);
        }
    });
}

#[test]
fn range_split_nonempty() {
    let data = [1, 2, 3, 4, 5];
    indices(&data[..], move |v, _| {
        for i in 0..v.len() {
            let r = v.vet_range(0..i).unwrap();
            if let Ok(r) = r.nonempty() {
                let (a, b) = r.split_in_half();
                assert!(b.len() > 0);
                assert_eq!(a.len() + b.len(), r.len());
                assert!(b.first().integer() < r.len());
            } else {
                let (a, b) = r.split_in_half();
                assert_eq!(a.len(), 0);
                assert_eq!(b.len(), 0);
                assert_eq!(a.start(), b.start());
            }
        }
    });
}
    

fn is_sorted<T: Clone + Ord>(v: &[T]) -> bool {
    let mut vec = v.to_vec();
    vec.sort();
    vec == v
}

#[test]
fn qc_quicksort() {
    fn prop(mut v: Vec<i32>) -> bool {
        indexing::algorithms::quicksort(&mut v);
        is_sorted(&v)
    }

    quickcheck::quickcheck(prop as fn(_) -> bool);
}

#[test]
fn qc_quicksort_bounds() {
    fn prop(mut v: Vec<i32>) -> bool {
        indexing::algorithms::quicksort_bounds(&mut v);
        is_sorted(&v)
    }

    quickcheck::quickcheck(prop as fn(_) -> bool);
}

// check the heap property
fn is_minheap<T: Ord>(v: &[T]) -> bool {
    // minheap:  parent is less or equal to child
    // k -> 2k + 1, 2k + 2
    for (index, parent) in v.iter().enumerate() {
        let child = 2 * index + 1;
        if child < v.len() && &v[child] < parent {
            return false;
        }
        if child + 1 < v.len() && &v[child + 1] < parent {
            return false;
        }
    }
    true
}

#[test]
fn qc_heapify() {
    fn prop(mut v: Vec<i32>) -> bool {
        indexing::algorithms::heapify(&mut v);
        is_minheap(&v)
    }
    quickcheck::quickcheck(prop as fn(_) -> bool);
}

#[cfg(test)]
fn bench_data(data: &mut [i32]) {
    let len = data.len();
    for (index, elt) in data.iter_mut().enumerate() {
        *elt = ((index * 123) % len) as i32;
    }
}
#[test]
fn test_insertion_sort() {
    let mut data = [2, 1];
    insertion_sort_indexes(&mut data, |a, b| a < b);
    assert_eq!(data, [1, 2]);

    let mut data = [2, 1, 3];
    insertion_sort_indexes(&mut data, |a, b| a < b);
    assert_eq!(data, [1, 2, 3]);

    let mut data = [2, 0, 2, 3, 4, 1, 0];
    insertion_sort_indexes(&mut data, |a, b| a < b);
    assert_eq!(data, [0, 0, 1, 2, 2, 3, 4]);

    let mut data = [0; 100];
    bench_data(&mut data);
    let mut data2 = data;
    insertion_sort_indexes(&mut data, |a, b| a < b);
    insertion_sort_rust(&mut data2, |a, b| a < b);
    assert_eq!(&data[..], &data2[..]);
}

quickcheck! {
    fn test_lower_bound_1(data: Vec<u8>, find: u8) -> bool {
        lower_bound(&data, &find) == lower_bound_raw_ptr(&data, &find)
    }

    fn test_lower_bound_2(data: Vec<u8>, find: u8) -> bool {
        lower_bound(&data, &find) ==
            data.binary_search_by(|x|
                if *x >= find {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }).unwrap_err()
    }

    fn test_lower_bound_3(data: Vec<u8>, find: u8) -> bool {
        lower_bound(&data, &find) == lower_bound_prange(&data, &find)
    }

    fn test_insertion_sort_prange(data: Vec<u8>) -> bool {
        let mut data = data;
        let mut ans = data.clone();
        ans.sort();
        insertion_sort_pointerindex(&mut data, |a, b| a < b);
        assert_eq!(ans, data);
        true
    }
}
