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
