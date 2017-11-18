extern crate indexing;

use indexing::scope;
use indexing::{Container, Range};
use indexing::container_traits::Trustworthy;

fn indices<Array, F, Out, T>(arr: Array, f: F) -> Out
    where F: for<'id> FnOnce(Container<'id, Array>, Range<'id>) -> Out,
          Array: Trustworthy<Item=T>,
{
    scope(arr, move |v| { let range = v.range(); f(v, range) })
}

fn main() {
    let arr1 = [1, 2, 3, 4, 5];
    let arr2 = [10, 20, 30];

    indices(&arr1[..], |arr1, r1| {
        indices(&arr2[..], move |arr2, r2| {
            &arr2[r1]; //~ ERROR cannot infer an appropriate lifetime
            &arr1[r2]; //~ ERROR cannot infer an appropriate lifetime
        });
    });
}

