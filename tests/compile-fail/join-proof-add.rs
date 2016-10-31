extern crate indexing;

use indexing::scope;

fn main() {
    let arr1 = [1, 2, 3, 4, 5];

    scope(&arr1[..], |v| {
        let r = v.range();
        if let Ok(r) = r.nonempty() {
            let (front, back) = r.frontiers();

            r.first();
            front.join(r).unwrap().first();
            r.join(back).unwrap().first();
            front.join_cover(r).first();
            r.join_cover(back).first();
            r.join_cover(r).first();

            r.last();
            front.join(r).unwrap().last();
            r.join(back).unwrap().last();
            front.join_cover(r).last();
            r.join_cover(back).last();
            r.join_cover(r).last();

            front.join_cover(back).first();
            front.join_cover(back).last();
            //~^ ERROR no method named

            let (a, b) = r.split_in_half();
            assert_eq!(a.join_cover(back), r);

            a.join_cover(back).last();
            //~^ ERROR no method named

            b.join_cover(back).last();
        }
    });
}

