extern crate indexing;

use indexing::indices;

fn main() {
    let mut arr1 = [1, 2, 3, 4, 5];

    indices(&mut arr1[..], |mut arr1, r1| {
        let (mut a, mut b) = r1.split_in_half();
        let i = a.next().unwrap();
        let j = a.next().unwrap();
        &mut arr1[i];
        &mut arr1[j];

        let x = &arr1[i];
        let y = &mut arr1[j]; //~ ERROR: as mutable because it is also borrowed
    });
}

