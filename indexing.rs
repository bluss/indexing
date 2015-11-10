// This program demonstrates sound unchecked indexing
// by having slices generate valid indices, and "signing"
// them with an invariant lifetime. These indices cannot be used on another
// slice, nor can they be stored until the array is no longer valid 
// (consider adapting this to Vec, and then trying to use indices after a push).
//
// This represents a design "one step removed" from iterators, providing greater
// control to the consumer of the API. Instead of getting references to elements
// we get indices, from which we can get references or hypothetically perform
// any other "index-related" operation (slicing?). Normally, these operations
// would need to be checked at runtime to avoid indexing out of bounds, but
// because the array knows it personally minted the indices, it can trust them.
// This hypothetically enables greater composition. Using this technique
// one could also do "only once" checked indexing (let idx = arr.validate(idx)).
// 
// The major drawback of this design is that it requires a closure to
// create an environment that the signatures are bound to, complicating
// any logic that flows between the two (e.g. moving values in/out and try!).
// In principle, the compiler could be "taught" this trick to eliminate the
// need for the closure, as far as I know. Although how one would communicate
// that they're trying to do this to the compiler is another question.
// It also relies on wrapping the structure of interest to provide a constrained
// API (again, consider applying this to Vec -- need to prevent `push` and `pop`
// being called). This is the same principle behind Entry and Iterator.
//
// It also produces terrible compile errors (random lifetime failures),
// because we're hacking novel semantics on top of the borrowchecker which
// it doesn't understand.
//
// This technique was first pioneered by gereeter to enable safely constructing
// search paths in BTreeMap. See Haskell's ST Monad for a related design.
//
// The example isn't maximally generic or fleshed out because I got bored trying
// to express the bounds necessary to handle &[T] and &mut [T] appropriately.

/// Based on “sound unchecked indexing”/“signing” by Gankro.
///
/// Extended to include interval (range) API
use std::cmp;
use std::ops;
use std::ptr;

use std::marker::PhantomData;
use std::ops::Deref;

// Cell<T> is invariant in T; so Cell<&'id _> makes `id` invariant.
// This means that the inference engine is not allowed to shrink or
// grow 'id to solve the borrow system. 
type Id<'id> = PhantomData<::std::cell::Cell<&'id mut ()>>;

pub struct Indexer<'id, Array> {
    id: Id<'id>,
    arr: Array,
}

#[derive(Copy, Clone, Debug)]
pub struct Index<'id> {
    id: Id<'id>,
    idx: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct Range<'id> {
    id: Id<'id>,
    start: usize,
    end: usize,
}

impl<'id, 'a, T> Indexer<'id, &'a [T]> {
    #[inline]
    pub fn get(&self, idx: Index<'id>) -> &'a T {
        unsafe {
            self.arr.get_unchecked(idx.idx)
        }
    }

    #[inline]
    pub fn slice(&self, r: Range<'id>) -> &'a [T] {
        unsafe {
            std::slice::from_raw_parts(
                self.arr.as_ptr().offset(r.start as isize),
                r.end - r.start)
        }
    }
}

impl<'id, 'a, Array, T> Indexer<'id, Array> where Array: Deref<Target=[T]> {
    #[inline]
    pub fn len(&self) -> usize {
        self.arr.len()
    }

    // Is this a good idea?
    /// Return the range [0, 0)
    #[inline]
    pub fn empty_range(&self) -> Range<'id> {
        Range { id: PhantomData, start: 0, end: 0 }
    }

    #[inline]
    pub fn split_at(&self, index: Index<'id>) -> (Range<'id>, Range<'id>) {
        unsafe {
            (Range::from(0, index.idx), Range::from(index.idx, self.arr.len()))
        }
    }

    /// Return the range before (not including) the index itself
    #[inline]
    pub fn before(&self, index: Index<'id>) -> Range<'id> {
        unsafe {
            Range::from(0, index.idx)
        }
    }

    /// Return the range after (not including) the index itself
    #[inline]
    pub fn after(&self, index: Index<'id>) -> Range<'id> {
        // in bounds because idx + 1 is <= .len()
        unsafe {
            Range::from(index.idx + 1, self.arr.len())
        }
    }

    /// Return true if the index is still in bounds
    #[inline]
    pub fn forward(&self, index: &mut Index<'id>) -> bool {
        let i = index.idx + 1;
        if i < self.arr.len() {
            index.idx = i;
            true
        } else { false }
    }
}

impl<'id, 'a, T> Indexer<'id, &'a mut [T]> {
    #[inline]
    pub fn swap(&mut self, i: Index<'id>, j: Index<'id>) {
        // ptr::swap is ok with equal pointers
        unsafe {
            ptr::swap(&mut self[i], &mut self[j])
        }
    }
}

impl<'id, 'a, T> ops::Index<Index<'id>> for Indexer<'id, &'a [T]> {
    type Output = T;
    #[inline(always)]
    fn index(&self, idx: Index<'id>) -> &T {
        unsafe {
            self.arr.get_unchecked(idx.idx)
        }
    }
}

impl<'id, 'a, T> ops::Index<Range<'id>> for Indexer<'id, &'a [T]> {
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: Range<'id>) -> &[T] {
        unsafe {
            std::slice::from_raw_parts(
                self.arr.as_ptr().offset(r.start as isize),
                r.len())
        }
    }
}

impl<'id, 'a, T> ops::Index<Index<'id>> for Indexer<'id, &'a mut [T]> {
    type Output = T;
    #[inline(always)]
    fn index(&self, idx: Index<'id>) -> &T {
        unsafe {
            self.arr.get_unchecked(idx.idx)
        }
    }
}

impl<'id, 'a, T> ops::IndexMut<Index<'id>> for Indexer<'id, &'a mut [T]> {
    #[inline(always)]
    fn index_mut(&mut self, idx: Index<'id>) -> &mut T {
        unsafe {
            self.arr.get_unchecked_mut(idx.idx)
        }
    }
}

impl<'id, 'a, T> ops::Index<Range<'id>> for Indexer<'id, &'a mut [T]> {
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: Range<'id>) -> &[T] {
        unsafe {
            std::slice::from_raw_parts(
                self.arr.as_ptr().offset(r.start as isize),
                r.len())
        }
    }
}

impl<'id, 'a, T> ops::IndexMut<Range<'id>> for Indexer<'id, &'a mut [T]> {
    #[inline(always)]
    fn index_mut(&mut self, r: Range<'id>) -> &mut [T] {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.arr.as_mut_ptr().offset(r.start as isize),
                r.len())
        }
    }
}

impl<'id, 'a, T> ops::Index<ops::RangeFrom<Index<'id>>> for Indexer<'id, &'a mut [T]> {
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: ops::RangeFrom<Index<'id>>) -> &[T] {
        let i = r.start.idx;
        unsafe {
            std::slice::from_raw_parts(
                self.arr.as_ptr().offset(i as isize),
                self.len() - i)
        }
    }
}

impl<'id, 'a, T> ops::IndexMut<ops::RangeFrom<Index<'id>>> for Indexer<'id, &'a mut [T]> {
    #[inline(always)]
    fn index_mut(&mut self, r: ops::RangeFrom<Index<'id>>) -> &mut [T] {
        let i = r.start.idx;
        unsafe {
            std::slice::from_raw_parts_mut(
                self.arr.as_mut_ptr().offset(i as isize),
                self.len() - i)
        }
    }
}

impl<'id> Range<'id> {
    #[inline(always)]
    unsafe fn from(start: usize, end: usize) -> Self {
        Range { id: PhantomData, start: start, end: end }
    }

    #[inline]
    pub fn as_range(&self) -> std::ops::Range<usize> { self.start..self.end }

    #[inline]
    pub fn len(&self) -> usize { self.end - self.start }

    #[inline]
    pub fn split_in_half(&self) -> (Range<'id>, Range<'id>) {
        let mid = (self.end - self.start) / 2 + self.start;
        (Range { id: self.id, start: self.start, end: mid },
         Range { id: self.id, start: mid, end: self.start })
    }

    /// If `i` is past the end, clamp it at the end
    #[inline]
    pub fn split_at_clamp(&self, i: usize) -> (Range<'id>, Range<'id>) {
        let mid = cmp::min(i, self.end);
        (Range { id: self.id, start: self.start, end: mid },
         Range { id: self.id, start: mid, end: self.end })
    }

    #[inline]
    pub fn increase_start(&mut self, offset: usize) {
        // FIXME saturating?
        self.start = cmp::min(self.start.saturating_add(offset), self.end);
    }

    #[inline]
    pub fn clamp_after(&mut self, end: usize) {
        self.end = cmp::min(self.start.saturating_add(end), self.end);
    }

    #[inline]
    pub fn decrease_end(&mut self, offset: usize) {
        self.end = cmp::max(self.start, self.end.saturating_sub(offset));
    }

    #[inline]
    pub fn contains(&self, index: usize) -> Option<Index<'id>> {
        if index >= self.start && index <= self.end {
            Some(Index { id: self.id, idx: index })
        } else { None }
    }
}

impl<'id> Iterator for Range<'id> {
    type Item = Index<'id>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.start != self.end {
            let index = self.start;
            self.start += 1;
            Some(Index { id: PhantomData, idx: index })
        } else {
            None
        }
    }
}

impl<'id> DoubleEndedIterator for Range<'id> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start != self.end {
            self.end -= 1;
            Some(Index { id: PhantomData, idx: self.end })
        } else {
            None
        }
    }
}

#[inline]
pub fn indices<Array, F, Out, T>(arr: Array, f: F) -> Out
    where F: for<'id> FnOnce(Indexer<'id, Array>, Range<'id>) -> Out,
          Array: Deref<Target = [T]>,
{
    // This is where the magic happens. We bind the indexer and indices
    // to the same invariant lifetime (a constraint established by F's
    // definition). As such, each call to `indices` produces a unique
    // signature that only these two values can share.
    //
    // Within this function, the borrow solver can choose literally any lifetime,
    // including `'static`, but we don't care what the borrow solver does in
    // *this* function. We only need to trick the solver in the caller's
    // scope. Since borrowck doesn't do interprocedural analysis, it
    // sees every call to this function produces values with some opaque
    // fresh lifetime and can't unify any of them.
    //
    // In principle a "super borrowchecker" that does interprocedural
    // analysis would break this design, but we could go out of our way
    // to somehow bind the lifetime to the inside of this function, making
    // it sound again. Borrowck will never do such analysis, so we don't
    // care.
    let len = arr.len();
    let indexer = Indexer { id: PhantomData, arr: arr };
    let indices = Range { id: PhantomData, start: 0, end: len };
    f(indexer, indices)
}

#[test]
fn main() {
    let arr1: &[u32] = &[1, 2, 3, 4, 5];
    let arr2: &[u32] = &[10, 20, 30];

    // concurrent iteration (hardest thing to do with iterators)
    indices(arr1, |arr1, it1| {
        indices(arr2, move |arr2, it2| {
            for (i, j) in it1.zip(it2) {
                println!("{} {}", arr1.get(i), arr2.get(j));
                //
                // println!("{} ", arr2.get(i));    // should be invalid to idx wrong source
                // println!("{} ", arr1.get(j));    // should be invalid to idx wrong source
            }
        });
    });
    
    // can hold onto the indices for later, as long they stay in the closure
    let _a = indices(arr1, |arr, mut it| {
        let a = it.next().unwrap();
        let b = it.next_back().unwrap();
        println!("{} {}", arr.get(a), arr.get(b));
        // a    // should be invalid to return an index
    });
    
    // can get references out, just not indices
    let (x, y) = indices(arr1, |arr, mut r| {
        println!("{:?}", arr.slice(r));
        let a = r.next().unwrap();
        let b = r.next_back().unwrap();
        (arr.get(a), arr.get(b))
    });
    println!("{} {}", x, y);
    
    // Excercise to the reader: sound multi-index mutable indexing!?
    // (hint: it would be unsound with the current design)
}

#[test]
fn intervals() {
    let mut data = [0; 16];
    indices(&mut data[..], |mut arr, r| {
        for elt in &mut arr[r] {
            *elt += 1;
        }
        println!("{:?}", &mut arr[r]);
    });
}
