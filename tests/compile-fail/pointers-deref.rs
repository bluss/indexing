extern crate indexing;

use indexing::scope;

fn main() {
    let arr1 = [1, 2, 3, 4, 5];

    // The .first() ptr is not dereferenceable when we don't have a length proof
    let _a = scope(&arr1[..], |arr| {
        let r = arr.pointer_range();
        println!("{}", &arr[r.first()]); //~ ERROR the trait bound
    });
}


