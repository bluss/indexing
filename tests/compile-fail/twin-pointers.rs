extern crate indexing;

use indexing::scope;

fn main() {
    let mut arr1 = [1, 2, 3, 4, 5i64];
    let mut arr2 = [6, 7, 8, 9, 0];

    // can hold onto the pointers for later, as long they stay in the closure
    let _a = scope(&mut arr1[..], |arr| {
        let r = arr.pointer_range();
        let r = r.nonempty().unwrap();      
        let i = r.first();
        let (a, b) = arr.split_at_pointer(i);

        let twin = arr.make_twin(&mut arr2[..]).unwrap();
        let elt = twin[i]; //~ ERROR: cannot be indexed by
        twin.split_at_pointer(i); //~ ERROR: no method named
    });
}


