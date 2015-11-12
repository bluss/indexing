extern crate indexing;

use indexing::indices;

fn main() {
    let arr1 = [1, 2, 3, 4, 5];

    // can hold onto the indices for later, as long they stay in the closure
    let _a = indices(&arr1[..], |arr, mut it| {
        let a = it.next().unwrap();          //~ ERROR cannot infer an appropriate lifetime
        let b = it.next_back().unwrap();
        println!("{} {}", arr[a], arr[b]);

        // should be invalid to return an index
        a
    });
}


