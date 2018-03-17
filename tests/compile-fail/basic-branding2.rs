extern crate indexing;

use indexing::scope;

fn main() {
    let arr1 = [1, 2, 3, 4, 5];

    // can hold onto the indices for later, as long they stay in the closure
    let _a = scope(&arr1[..], |arr| {
        let r = arr.range();        //~ ERROR borrowed data cannot be stored outside of its closure
        let r = r.nonempty().unwrap();      
        let i = r.first();
        println!("{}", &arr[i]);

        // should be invalid to return an index
        i
    });
}


