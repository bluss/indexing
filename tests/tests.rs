extern crate indexing;
extern crate quickcheck;

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
