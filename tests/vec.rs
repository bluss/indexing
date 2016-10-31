
extern crate indexing;

use indexing::indices;


#[test]
fn test_vec() {
    let mut v = vec![0; 8];
    indices(&mut v, |v, _| {
        v.push(1);
        let end = v.push(2);
        assert_eq!(v[end], 2);
    });
}
