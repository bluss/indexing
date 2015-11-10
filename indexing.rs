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

//! Based on “sound unchecked indexing”/“signing” by Gankro.
//!
//! Extended to include interval (range) API

#![feature(test)]

extern crate test;

#[cfg(test)]
use test::Bencher;

use std::cmp;
use std::ops;
use std::ptr;
use std::mem;

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

#[derive(Copy, Clone, Debug, Eq)]
pub struct Index<'id> {
    id: Id<'id>,
    idx: usize,
}

/// Index can only be compared with other indices of the same branding
impl<'id> PartialEq for Index<'id> {
    fn eq(&self, other: &Index<'id>) -> bool {
        self.idx == other.idx
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Checked<X, L> {
    item: X,
    proof: PhantomData<L>,
}

impl<X, L> Checked<X, L> {
    #[inline]
    unsafe fn new(item: X) -> Self {
        Checked { item: item, proof: PhantomData }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum NonEmpty {}
#[derive(Copy, Clone, Debug)]
pub enum Empty {}
#[derive(Copy, Clone, Debug)]
pub enum Unknown {}

trait LengthMarker {}

impl LengthMarker for NonEmpty {}
impl LengthMarker for Empty {}
impl LengthMarker for Unknown {}

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

    /// Rotate elements in the range by one step to the right (towards higher indices)
    #[inline]
    pub fn rotate1<R>(&mut self, r: R) where R: IntoCheckedRange<'id> {
        if let Ok(r) = r.into() {
            unsafe {
                let last_ptr = &self[r.last()] as *const _;
                let first_ptr = &mut self[r.first()] as *mut _;
                let tmp = ptr::read(last_ptr);
                ptr::copy(first_ptr,
                          first_ptr.offset(1),
                          r.len() - 1);
                ptr::copy_nonoverlapping(&tmp, first_ptr, 1);
                mem::forget(tmp);
            }
        }
    }

    /// Examine the elements after `index` in order from lower indices towards higher
    /// While the closure returns `true`, continue scan and include the scanned
    /// element in the range.
    ///
    /// Result always includes `index` in the range
    #[inline]
    pub fn scan_head<'b, F>(&'b self, index: Index<'id>, mut f: F) -> Checked<Range<'id>, NonEmpty>
        where F: FnMut(&'b T) -> bool
    {
        let mut end = index;
        for elt in &self[self.after(index)] {
            if !f(elt) {
                break;
            }
            end.idx += 1;
        }
        end.idx += 1;
        unsafe {
            Checked::new(Range::from(index.idx, end.idx))
        }
    }

    /// Examine the elements `range` in order from lower indices towards higher
    /// While the closure returns `true`, continue scan and include the scanned
    /// element in the range.
    ///
    /// Result always includes `index` in the range
    #[inline]
    pub fn scan_range<'b, F>(&'b self, range: Range<'id>, mut f: F) -> Range<'id>
        where F: FnMut(&'b T) -> bool
    {
        let mut end = range.start;
        for elt in &self[range] {
            if !f(elt) {
                break;
            }
            end += 1;
        }
        unsafe {
            Range::from(range.start, end)
        }
    }

    /// Examine the elements before `index` in order from higher indices towards lower.
    /// While the closure returns `true`, continue scan and include the scanned
    /// element in the range.
    ///
    /// Result always includes `index` in the range
    #[inline]
    pub fn scan_tail<F>(&self, index: Index<'id>, mut f: F) -> Checked<Range<'id>, NonEmpty>
        where F: FnMut(&T) -> bool
    {
        unsafe {
            let mut start = index;
            for elt in self[..index].iter().rev() {
                if !f(elt) {
                    break;
                }
                start.idx -= 1;
            }
            Checked::new(Range::from(start.idx, index.idx + 1))
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

impl<'id, 'a, T> ops::Index<ops::RangeTo<Index<'id>>> for Indexer<'id, &'a mut [T]> {
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: ops::RangeTo<Index<'id>>) -> &[T] {
        let i = r.end.idx;
        unsafe {
            std::slice::from_raw_parts(self.arr.as_ptr(), i)
        }
    }
}

impl<'id, 'a, T> ops::IndexMut<ops::RangeTo<Index<'id>>> for Indexer<'id, &'a mut [T]> {
    #[inline(always)]
    fn index_mut(&mut self, r: ops::RangeTo<Index<'id>>) -> &mut [T] {
        let i = r.end.idx;
        unsafe {
            std::slice::from_raw_parts_mut(self.arr.as_mut_ptr(), i)
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

    /// Check if the range is empty. Nonempty ranges have extra methods.
    #[inline]
    pub fn nonempty(&self) -> Result<Checked<Self, NonEmpty>, Checked<Self, Empty>> {
        unsafe {
            if self.len() > 0 {
                Ok(Checked::new(*self))
            } else {
                Err(Checked::new(*self))
            }
        }
    }

    #[inline]
    pub fn len(&self) -> usize { self.end - self.start }
    #[inline]
    pub fn is_empty(&self) -> bool { self.start == self.end }

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
    pub fn clamp_end_at(&mut self, end: usize) {
        self.end = cmp::min(cmp::max(self.start, end), self.end);
    }

    #[inline]
    pub fn clamp_len(&mut self, len: usize) {
        let diff = cmp::min(self.len(), len);
        self.end -= diff;
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

    /// Return an iterator that divides the range in `n` parts, in as
    /// eaven length chunks as possible.
    #[inline]
    pub fn even_chunks(&self, n: usize) -> Intervals<'id> {
        Intervals {
            fs: FracStep::new(self.start, self.end, n),
            range: *self,
        }
    }
}

pub trait IntoCheckedRange<'id> : Sized {
    fn into(self) -> Result<Checked<Range<'id>, NonEmpty>, Checked<Range<'id>, Empty>>;
}

impl<'id> IntoCheckedRange<'id> for Range<'id> {
    #[inline]
    fn into(self) -> Result<Checked<Range<'id>, NonEmpty>, Checked<Range<'id>, Empty>> {
        self.nonempty()
    }
}

impl<'id> IntoCheckedRange<'id> for Checked<Range<'id>, NonEmpty> {
    #[inline]
    fn into(self) -> Result<Checked<Range<'id>, NonEmpty>, Checked<Range<'id>, Empty>> {
        Ok(self)
    }
}

impl<'id> IntoCheckedRange<'id> for Checked<Range<'id>, Empty> {
    #[inline]
    fn into(self) -> Result<Checked<Range<'id>, NonEmpty>, Checked<Range<'id>, Empty>> {
        Err(self)
    }
}

impl<'id> IntoCheckedRange<'id> for Checked<Range<'id>, Unknown> {
    #[inline]
    fn into(self) -> Result<Checked<Range<'id>, NonEmpty>, Checked<Range<'id>, Empty>> {
        (*self).nonempty()
    }
}

impl<'id> Checked<Range<'id>, NonEmpty> {
    #[inline]
    pub fn first(&self) -> Index<'id> {
        Index { id: self.id, idx: self.start }
    }

    /// Return the middle index, rounding up on even
    #[inline]
    pub fn upper_middle(&self) -> Index<'id> {
        let mid = (self.end - self.start) / 2 + self.start;
        Index { id: self.id, idx: mid }
    }

    #[inline]
    pub fn tail(&self) -> Range<'id> {
        // in bounds since it's nonempty
        unsafe {
            Range::from(self.start + 1, self.end)
        }
    }

    #[inline]
    pub fn init(&self) -> Range<'id> {
        // in bounds since it's nonempty
        unsafe {
            Range::from(self.start, self.end - 1)
        }
    }

    #[inline]
    pub fn last(&self) -> Index<'id> {
        Index { id: self.id, idx: self.end - 1 }
    }

    #[inline]
    pub fn advance(&self) -> Result<Checked<Range<'id>, NonEmpty>,
                                    Checked<Range<'id>, Empty>>
    {
        unsafe {
            let next = Range::from(self.start + 1, self.end);
            if !next.is_empty() {
                Ok(Checked::new(next))
            } else {
                Err(Checked::new(next))
            }
        }
    }

    #[inline]
    pub fn advance_back(&self) -> Result<Checked<Range<'id>, NonEmpty>,
                                         Checked<Range<'id>, Empty>>
    {
        unsafe {
            let next = Range::from(self.start, self.end - 1);
            if !next.is_empty() {
                Ok(Checked::new(next))
            } else {
                Err(Checked::new(next))
            }
        }
    }
}

/// Deref to the inner range
// NOTE: immutable deref is ok, mutable would be unsound
impl<'id, X, L> Deref for Checked<X, L> {
    type Target = X;
    fn deref(&self) -> &X {
        &self.item
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

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
/// decimal, numerator, denominator
struct Frac(usize, usize, usize);

impl Frac {
    // Add decimal and fractional part, return decimal result
    #[inline]
    fn next_interval(&mut self, dec: usize, frac: usize) -> (usize, usize) {
        let start = self.0;
        self.0 += dec;
        self.1 += frac;
        if self.1 >= self.2 {
            self.1 -= self.2;
            self.0 += 1;
        }
        (start, self.0)
    }
}

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
struct FracStep {
    f: Frac,
    frac_step: usize,
    decimal_step: usize,
    start: usize,
    end: usize,
}

impl FracStep {
    #[inline]
    fn new(start: usize, mut end: usize, divisor: usize) -> Self {
        debug_assert!(start <= end);
        // decimal_step * divisor + frac_step == len
        let len = end - start;
        let decimal_step = len / divisor;
        let frac_step = len % divisor;
        if decimal_step == 0 {
            end = start;
        }
        FracStep {
            f: Frac(start, 0, divisor),
            frac_step: frac_step,
            decimal_step: decimal_step,
            start: start,
            end: end,
        }
    }

    /// Return the next interval / index range
    #[inline]
    fn next(&mut self) -> Option<(usize, usize)> {
        if self.f.0 >= self.end {
            None
        } else {
            let (ds, fs) = (self.decimal_step, self.frac_step);
            Some(self.f.next_interval(ds, fs))
        }
    }
}

/// `Intervals` is an iterator of evenly sized nonempty, nonoverlapping ranges
#[derive(Copy, Clone, Debug)]
pub struct Intervals<'id> {
    range: Range<'id>,
    fs: FracStep,
}

impl<'id> Intervals<'id> {
    /// Reset counter and double up
    pub fn double(&mut self) {
    }
}

impl<'id> Iterator for Intervals<'id> {
    type Item = Checked<Range<'id>, NonEmpty>;
    #[inline]
    fn next(&mut self) -> Option<Checked<Range<'id>, NonEmpty>> {
        self.fs.next().map(|(i, j)| {
            debug_assert!(self.range.contains(i).is_some());
            debug_assert!(self.range.contains(j).is_some() || j == self.range.end);
            debug_assert!(i != j);
            unsafe {
                Checked::new(Range::from(i, j))
            }
        })
    }
}

#[test]
fn test_frac_step() {
    let mut f = FracStep::new(0, 8, 3);
    assert_eq!(f.next(), Some((0, 2)));
    assert_eq!(f.next(), Some((2, 5)));
    assert_eq!(f.next(), Some((5, 8)));
    assert_eq!(f.next(), None);

    let mut f = FracStep::new(1, 9, 3);
    assert_eq!(f.next(), Some((1, 3)));
    assert_eq!(f.next(), Some((3, 6)));
    assert_eq!(f.next(), Some((6, 9)));
    assert_eq!(f.next(), None);

    // Too long and it should be empty
    let mut f = FracStep::new(0, 7, 8);
    assert_eq!(f.next(), None);
    assert_eq!(f.next(), None);

    let mut f = FracStep::new(0, 3, 1);
    assert_eq!(f.next(), Some((0, 3)));
    assert_eq!(f.next(), None);
}

#[test]
fn test_intervals() {
    let mut data = [0; 8];
    indices(&mut data[..], |mut data, r| {
        for (index, part) in r.even_chunks(3).enumerate() {
            for elt in &mut data[*part] {
                *elt = index;
            }
        }
    });
    assert_eq!(&data[..], &[0, 0, 1, 1, 1, 2, 2, 2]);
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
    //
    // can get references out, just not indices
    let (x, y) = indices(arr1, |arr, mut r| {
        println!("{:?}", arr.slice(r));
        let a = r.next().unwrap();
        let b = r.next_back().unwrap();
        (arr.get(a), arr.get(b))
    });
    println!("{} {}", x, y);
    //
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

#[cfg(test)]
// Copied from rust / libcollections/slice.rs
fn rust_insertion_sort<T, F>(v: &mut [T], mut less_than: F) where F: FnMut(&T, &T) -> bool {
    use std::mem;
    use std::ptr;
    let len = v.len() as isize;
    let buf_v = v.as_mut_ptr();

    // 1 <= i < len;
    for i in 1..len {
        // j satisfies: 0 <= j <= i;
        let mut j = i;
        unsafe {
            // `i` is in bounds.
            let read_ptr = buf_v.offset(i) as *const T;

            // find where to insert, we need to do strict <,
            // rather than <=, to maintain stability.

            // 0 <= j - 1 < len, so .offset(j - 1) is in bounds.
            while j > 0 && less_than(&*read_ptr, &*buf_v.offset(j - 1)) {
                j -= 1;
            }

            // shift everything to the right, to make space to
            // insert this value.

            // j + 1 could be `len` (for the last `i`), but in
            // that case, `i == j` so we don't copy. The
            // `.offset(j)` is always in bounds.

            if i != j {
                let tmp = ptr::read(read_ptr);
                ptr::copy(&*buf_v.offset(j),
                          buf_v.offset(j + 1),
                          (i - j) as usize);
                ptr::copy_nonoverlapping(&tmp, buf_v.offset(j), 1);
                mem::forget(tmp);
            }
        }
    }
}


#[cfg(test)]
fn indexing_insertion_sort<T, F>(v: &mut [T], mut less_than: F) where F: FnMut(&T, &T) -> bool {
    indices(v, move |mut v, r| {
        for i in r {
            let jtail = v.scan_tail(i, |j_elt| less_than(&v[i], j_elt));
            v.rotate1(jtail);
        }
    });
}

#[test]
fn test_insertion_sort() {
    let mut data = [2, 1];
    indexing_insertion_sort(&mut data, |a, b| a < b);
    assert_eq!(data, [1, 2]);

    let mut data = [2, 1, 3];
    indexing_insertion_sort(&mut data, |a, b| a < b);
    assert_eq!(data, [1, 2, 3]);

    let mut data = [2, 0, 2, 3, 4, 1, 0];
    indexing_insertion_sort(&mut data, |a, b| a < b);
    assert_eq!(data, [0, 0, 1, 2, 2, 3, 4]);

    let mut data = [0; 100];
    bench_data(&mut data);
    let mut data2 = data;
    indexing_insertion_sort(&mut data, |a, b| a < b);
    rust_insertion_sort(&mut data2, |a, b| a < b);
    assert_eq!(&data[..], &data2[..]);
}

#[cfg(test)]
fn bench_data(data: &mut [i32]) {
    let len = data.len();
    for (index, elt) in data.iter_mut().enumerate() {
        *elt = ((index * 123) % len) as i32;
    }
}

#[bench]
fn bench_insertion_sort_1024(b: &mut Bencher) {
    let mut data = [0; 1024];
    bench_data(&mut data);

    b.iter(|| {
        let mut d = data;
        indexing_insertion_sort(&mut d, |a, b| a < b);
    });
    b.bytes = mem::size_of_val(&data) as u64;
}

#[bench]
fn bench_insertion_sort_100(b: &mut Bencher) {
    let mut data = [0; 100];
    bench_data(&mut data);

    b.iter(|| {
        let mut d = data;
        indexing_insertion_sort(&mut d, |a, b| a < b);
    });
    b.bytes = mem::size_of_val(&data) as u64;
}


#[bench]
fn bench_rust_insertion_sort_1024(b: &mut Bencher) {
    let mut data = [0; 1024];
    bench_data(&mut data);

    b.iter(|| {
        let mut d = data;
        rust_insertion_sort(&mut d, |a, b| a < b);
    });
    b.bytes = mem::size_of_val(&data) as u64;
}

#[bench]
fn bench_rust_insertion_sort_100(b: &mut Bencher) {
    let mut data = [0; 100];
    bench_data(&mut data);

    b.iter(|| {
        let mut d = data;
        rust_insertion_sort(&mut d, |a, b| a < b);
    });
    b.bytes = mem::size_of_val(&data) as u64;
}

#[test]
fn test_scan() {
    let mut data = [0, 0, 0, 1, 2];
    indices(&mut data[..], |data, r| {
        let r = r.nonempty().unwrap();
        let range = data.scan_head(r.first(), |elt| *elt == 0);
        assert_eq!(&data[*range], &[0, 0, 0]);
        let range = data.scan_head(range.last(), |elt| *elt != 0);
        assert_eq!(&data[*range], &[0, 1, 2]);
    });
}
