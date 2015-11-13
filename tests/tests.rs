extern crate indexing;
extern crate quickcheck;

use indexing::algorithms::*;

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
