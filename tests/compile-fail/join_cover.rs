extern crate indexing;

use indexing::scope;

fn main() {
    // Bug from https://github.com/bluss/indexing/issues/12
    let array = [0, 1, 2, 3, 4, 5];
    let ix = scope(&array[..], |arr| {
        let left = arr.vet_range(0..2).unwrap();
        let left = left.nonempty().unwrap();
        let (_, right) = arr.range().frontiers();

        let joined = right.join_cover(left);
        let ix = joined.first();
        arr[ix]; //~ ERROR: cannot be indexed by
        ix.integer()
    });
    dbg!(array[ix]);
}


