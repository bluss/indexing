extern crate indexing;

use indexing::scope;

fn main() {
    let mut arr1 = [1, 2, 3, 4, 5];

    scope(&mut arr1[..], |mut arr1| {
        let (a, b) = arr1.range().split_in_half();
        for i in a {
            for j in b {
                &mut arr1[i];
                &mut arr1[j];

                let _x = &arr1[i];
                let _y = &mut arr1[j]; //~ ERROR: as mutable because it is also borrowed
            }
        }
    });
}

