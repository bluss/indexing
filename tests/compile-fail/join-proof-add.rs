extern crate indexing;

use indexing::indices;

fn main() {
    let arr1 = [1, 2, 3, 4, 5];

    indices(&arr1[..], |_, r| {
        if let Ok(r) = r.nonempty() {
            let (front, back) = r.frontiers();

            r.first();
            front.join(r).unwrap().first();
            r.join(back).unwrap().first();
            front.join_cover(r).first();
            r.join_cover(back).first();
            r.join_cover(r).first();

            front.join_cover(back).first();
            //~^ ERROR no method named

            let (a, b) = r.split_in_half();
            assert_eq!(a.join_cover(back), r);

            a.join_cover(back).first();
            //~^ ERROR no method named

            b.join_cover(back).first();
        }
    });
}

