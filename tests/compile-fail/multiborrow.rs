extern crate indexing;

use indexing::scope;

fn main() {
    let mut arr1 = [1, 2, 3, 4, 5];

    scope(&mut arr1[..], |mut arr1| {
        let (a, b) = arr1.range().split_in_half();
        for i in a {
            for j in b {
                let _ = &mut arr1[i];
                let _ = &mut arr1[j];

                let xi2 = &arr1[i];
                let yi2 = &mut arr1[j]; //~ ERROR: as mutable because it is also borrowed
                *yi2 = *xi2;
            }
        }
    });
}

