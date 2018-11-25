#![cfg(feature="use_std")]

extern crate indexing;

use indexing::scope;


#[test]
fn test_vec() {
    let mut v = vec![0, 1];
    scope(&mut v, |v| {
        let mut v = v.only_index();
        v.push(1);
        let end = v.push(2);
        assert_eq!(v[end], 2);
    });
    assert_eq!(&v, &[0, 1, 1, 2]);
}
