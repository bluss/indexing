extern crate indexing;

use indexing::indices;

fn main() {
    let mut arr1 = [1, 2, 3, 4, 5];

    indices(&mut arr1[..], |mut arr1, r1| {
        let (a, b) = r1.split_in_half();
        for i in a {
            for j in b {
                &mut arr1[i];
                &mut arr1[j];

                let x = &arr1[i];
                let y = &mut arr1[j]; //~ ERROR: as mutable because it is also borrowed
            }
        }
    });
}

