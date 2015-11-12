
//! Based on “sound unchecked indexing”/“signing” by Gankro.
//!
//! Extended to include interval (range) API

#![feature(test)]
#![feature(core_intrinsics)]

extern crate test;

// Modules
pub mod pointer;
pub mod algorithms;

#[cfg(test)]
use test::Bencher;

use std::cmp;
use std::ops;
use std::ptr;
use std::mem;

use std::fmt::{self, Debug};

use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use pointer::PIndex;

/// A marker trait for collections where we can safely vet indices
pub unsafe trait Buffer : Deref {
}

unsafe impl<'a, T> Buffer for &'a [T] { }
unsafe impl<'a, T> Buffer for &'a mut [T] { }

pub unsafe trait BufferMut : Buffer + DerefMut { }
unsafe impl<X: ?Sized> BufferMut for X where X: Buffer + DerefMut { }

// Cell<T> is invariant in T; so Cell<&'id _> makes `id` invariant.
// This means that the inference engine is not allowed to shrink or
// grow 'id to solve the borrow system.
type Id<'id> = PhantomData<::std::cell::Cell<&'id mut ()>>;

pub struct Indexer<'id, Array> {
    id: Id<'id>,
    arr: Array,
}

#[derive(Copy, Clone, Eq)]
pub struct Index<'id> {
    id: Id<'id>,
    idx: usize,
}

impl<'id> Debug for Index<'id> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Index({})", self.idx)
    }
}

/// Index can only be compared with other indices of the same branding
impl<'id> PartialEq for Index<'id> {
    #[inline(always)]
    fn eq(&self, other: &Index<'id>) -> bool {
        self.idx == other.idx
    }
}


/// Length marker for range known to not be empty.
#[derive(Copy, Clone, Debug)]
pub enum NonEmpty {}
/// Length marker for unknown length.
#[derive(Copy, Clone, Debug)]
pub enum Unknown {}

trait LengthMarker {}

impl LengthMarker for NonEmpty {}
impl LengthMarker for Unknown {}

impl<'id, 'a, Array, T> Indexer<'id, Array> where Array: Buffer<Target=[T]> {
    #[inline]
    pub fn len(&self) -> usize {
        self.arr.len()
    }

    // Is this a good idea?
    /// Return the range [0, 0)
    #[inline]
    pub fn empty_range(&self) -> Range<'id> {
        unsafe {
            Range::from(0, 0)
        }
    }

    /// Return the full range of the Indexer.
    #[inline]
    pub fn range(&self) -> Range<'id> {
        unsafe {
            Range::from(0, self.arr.len())
        }
    }

    #[inline]
    pub fn split_at(&self, index: Index<'id>) -> (Range<'id>, Range<'id, NonEmpty>) {
        unsafe {
            (Range::from(0, index.idx), Range::from_ne(index.idx, self.arr.len()))
        }
    }

    /// Split in two ranges, where the first includes the `index` and the second
    /// starts with the following index.
    #[inline]
    pub fn split_after(&self, index: Index<'id>) -> (Range<'id, NonEmpty>, Range<'id>) {
        let mid = index.idx + 1; // must be <= len since `index` is in bounds
        unsafe {
            (Range::from_ne(0, mid), Range::from(mid, self.arr.len()))
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

impl<'id, T, Array> Indexer<'id, Array>
    where Array: BufferMut<Target=[T]>,
{
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
            if r.first() != r.last() {
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
    }

    /// Examine the elements after `index` in order from lower indices towards higher
    /// While the closure returns `true`, continue scan and include the scanned
    /// element in the range.
    ///
    /// Result always includes `index` in the range
    #[inline]
    pub fn scan_head<'b, F>(&'b self, index: Index<'id>, mut f: F) -> Range<'id, NonEmpty>
        where F: FnMut(&'b T) -> bool, T: 'b,
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
            Range::from_ne(index.idx, end.idx)
        }
    }

    /// Examine the elements `range` in order from lower indices towards higher
    /// While the closure returns `true`, continue scan and include the scanned
    /// element in the range.
    ///
    /// Result always includes `index` in the range
    #[inline]
    pub fn scan_range<'b, F, P>(&'b self, range: Range<'id, P>, mut f: F) -> Range<'id>
        where F: FnMut(&'b T) -> bool, T: 'b,
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
    pub fn scan_tail<'b, F>(&'b self, index: Index<'id>, mut f: F) -> Range<'id, NonEmpty>
        where F: FnMut(&'b T) -> bool, T: 'b
    {
        unsafe {
            let mut start = index;
            for elt in self[..index].iter().rev() {
                if !f(elt) {
                    break;
                }
                start.idx -= 1;
            }
            Range::from_ne(start.idx, index.idx + 1)
        }
    }
}


impl<'id, T, Array> ops::Index<Index<'id>> for Indexer<'id, Array>
    where Array: Buffer<Target=[T]>
{
    type Output = T;
    #[inline(always)]
    fn index(&self, idx: Index<'id>) -> &T {
        unsafe {
            self.arr.get_unchecked(idx.idx)
        }
    }
}

impl<'id, T, Array, P> ops::Index<Range<'id, P>> for Indexer<'id, Array>
    where Array: Buffer<Target=[T]>,
{
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: Range<'id, P>) -> &[T] {
        unsafe {
            std::slice::from_raw_parts(
                self.arr.as_ptr().offset(r.start as isize),
                r.len())
        }
    }
}

impl<'id, T, Array> ops::IndexMut<Index<'id>> for Indexer<'id, Array>
    where Array: BufferMut<Target=[T]>,
{
    #[inline(always)]
    fn index_mut(&mut self, idx: Index<'id>) -> &mut T {
        unsafe {
            self.arr.get_unchecked_mut(idx.idx)
        }
    }
}

impl<'id, T, Array, P> ops::IndexMut<Range<'id, P>> for Indexer<'id, Array>
    where Array: BufferMut<Target=[T]>,
{
    #[inline(always)]
    fn index_mut(&mut self, r: Range<'id, P>) -> &mut [T] {
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

impl<'id, T, Array> ops::Index<ops::RangeTo<Index<'id>>> for Indexer<'id, Array>
    where Array: Buffer<Target=[T]>,
{
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: ops::RangeTo<Index<'id>>) -> &[T] {
        let i = r.end.idx;
        unsafe {
            std::slice::from_raw_parts(self.arr.as_ptr(), i)
        }
    }
}

impl<'id, T, Array> ops::IndexMut<ops::RangeTo<Index<'id>>> for Indexer<'id, Array>
    where Array: BufferMut<Target=[T]>
{
    #[inline(always)]
    fn index_mut(&mut self, r: ops::RangeTo<Index<'id>>) -> &mut [T] {
        let i = r.end.idx;
        unsafe {
            std::slice::from_raw_parts_mut(self.arr.as_mut_ptr(), i)
        }
    }
}

impl<'id, T, Array> ops::Index<ops::RangeFull> for Indexer<'id, Array>
    where Array: Buffer<Target=[T]>,
{
    type Output = [T];
    #[inline(always)]
    fn index(&self, _: ops::RangeFull) -> &[T] {
        &self.arr[..]
    }
}

impl<'id, T, Array> ops::IndexMut<ops::RangeFull> for Indexer<'id, Array>
    where Array: BufferMut<Target=[T]>,
{
    #[inline(always)]
    fn index_mut(&mut self, _: ops::RangeFull) -> &mut [T] {
        &mut self.arr[..]
    }
}


// ###### Bounds checking impls #####
impl<'id, 'a, T> ops::Index<ops::Range<usize>> for Indexer<'id, &'a mut [T]> {
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: ops::Range<usize>) -> &[T] {
        &self.arr[r]
    }
}

impl<'id, 'a, T> ops::Index<ops::RangeFrom<usize>> for Indexer<'id, &'a mut [T]> {
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: ops::RangeFrom<usize>) -> &[T] {
        &self.arr[r]
    }
}

impl<'id, 'a, T> ops::Index<ops::RangeTo<usize>> for Indexer<'id, &'a mut [T]> {
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: ops::RangeTo<usize>) -> &[T] {
        &self.arr[r]
    }
}

impl<'id, 'a, T> ops::IndexMut<ops::Range<usize>> for Indexer<'id, &'a mut [T]> {
    #[inline(always)]
    fn index_mut(&mut self, r: ops::Range<usize>) -> &mut [T] {
        &mut self.arr[r]
    }
}

impl<'id, 'a, T> ops::IndexMut<ops::RangeFrom<usize>> for Indexer<'id, &'a mut [T]> {
    #[inline(always)]
    fn index_mut(&mut self, r: ops::RangeFrom<usize>) -> &mut [T] {
        &mut self.arr[r]
    }
}

impl<'id, 'a, T> ops::IndexMut<ops::RangeTo<usize>> for Indexer<'id, &'a mut [T]> {
    #[inline(always)]
    fn index_mut(&mut self, r: ops::RangeTo<usize>) -> &mut [T] {
        &mut self.arr[r]
    }
}
// ####

/// return the number of steps between a and b
fn ptrdistance<T>(a: *const T, b: *const T) -> usize {
    (a as usize - b as usize) / mem::size_of::<T>()
}

#[inline(always)]
fn ptr_iselement<T>(arr: &[T], ptr: *const T) {
    unsafe {
        let end = arr.as_ptr().offset(arr.len() as isize);
        debug_assert!(ptr >= arr.as_ptr() && ptr < end);
    }
}

impl<'id, 'a, T> ops::Index<PIndex<'id, T>> for Indexer<'id, &'a mut [T]> {
    type Output = T;
    #[inline(always)]
    fn index(&self, r: PIndex<'id, T>) -> &T {
        ptr_iselement(self.arr, r.ptr());
        unsafe {
            &*r.ptr()
        }
    }
}

impl<'id, T, Array> ops::Index<ops::RangeTo<PIndex<'id, T>>> for Indexer<'id, Array>
    where Array: Buffer<Target=[T]>,
{
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: ops::RangeTo<PIndex<'id, T>>) -> &[T] {
        let len = ptrdistance(r.end.ptr(), self.arr.as_ptr());
        unsafe {
            std::slice::from_raw_parts(self.arr.as_ptr(), len)
        }
    }
}

/// A branded range.
///
/// `Range<'id>` only indexes the container instantiated with the exact same
/// particular lifetime for the parameter `'id` at its inception from
/// the `indices()` constructor.
///
/// The `Range` may carry a proof of nonemptiness (type parameter `Proof`),
/// which enables further methods.
pub struct Range<'id, Proof=Unknown> {
    id: Id<'id>,
    start: usize,
    end: usize,
    /// NonEmpty or Not
    proof: PhantomData<Proof>,
}

impl<'id, P> Copy for Range<'id, P> { }
impl<'id, P> Clone for Range<'id, P> {
    #[inline]
    fn clone(&self) -> Self { *self }
}

impl<'id> Range<'id> {
    #[inline(always)]
    unsafe fn from(start: usize, end: usize) -> Range<'id> {
        debug_assert!(start <= end);
        Range { id: PhantomData, start: start, end: end, proof: PhantomData }
    }
}

impl<'id> Range<'id, NonEmpty> {
    #[inline(always)]
    unsafe fn from_ne(start: usize, end: usize) -> Range<'id, NonEmpty> {
        debug_assert!(start < end);
        Range { id: PhantomData, start: start, end: end, proof: PhantomData }
    }
}

impl<'id, P> Range<'id, P> {
    /// Return the length of the range.
    #[inline]
    pub fn len(&self) -> usize { self.end - self.start }

    /// Return `true` if the range is empty.
    #[inline]
    pub fn is_empty(&self) -> bool { self.start == self.end }

    /// Try to create a proof that the Range is nonempty; return
    /// a `Result` where the `Ok` branch carries a non-empty Range.
    #[inline]
    pub fn nonempty(&self) -> Result<Range<'id, NonEmpty>, Range<'id>> {
        unsafe {
            if !self.is_empty() {
                Ok(mem::transmute(*self))
            } else {
                Err(mem::transmute(*self))
            }
        }
    }

    #[inline]
    pub fn split_in_half(&self) -> (Range<'id>, Range<'id>) {
        let mid = (self.end - self.start) / 2 + self.start;
        unsafe {
            (Range::from(self.start, mid), Range::from(mid, self.end))
        }
    }

    /// If `abs_index` is past the end, clamp it at the end
    ///
    /// `abs_index` is an absolute index
    #[inline]
    pub fn split_at_clamp(&self, abs_index: usize) -> (Range<'id>, Range<'id>) {
        let mid = cmp::min(abs_index, self.end);
        unsafe {
            (Range::from(self.start, mid), Range::from(mid, self.end))
        }
    }

    /// Split to length `index`; if past the end, return false and clamp to the end
    ///
    /// `index` is a relative index.
    #[inline]
    pub fn split_at(&self, index: usize) -> (Range<'id>, Range<'id>, bool) {
        let mid = if index > self.len() {
             self.end
        } else { self.start + index };
        unsafe {
            (Range::from(self.start, mid), Range::from(mid, self.end),
             index <= self.len())
        }
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

    /// `abs_index` is an absolute index
    #[inline]
    pub fn contains(&self, abs_index: usize) -> Option<Index<'id>> {
        if abs_index >= self.start && abs_index <= self.end {
            Some(Index { id: self.id, idx: abs_index })
        } else { None }
    }

    /// Return an iterator that divides the range in `n` parts, in as
    /// even length chunks as possible.
    #[inline]
    pub fn even_chunks(&self, n: usize) -> Intervals<'id> {
        unsafe {
            Intervals {
                fs: FracStep::new(self.start, self.end, n),
                range: Range::from(self.start, self.end),
            }
        }
    }

    #[inline]
    pub fn as_range(&self) -> std::ops::Range<usize> { self.start..self.end }
}

impl<'id, P> Debug for Range<'id, P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Range({}, {})", self.start, self.end)
    }
}

pub trait IntoCheckedRange<'id> : Sized {
    fn into(self) -> Result<Range<'id, NonEmpty>, Range<'id>>;
}

impl<'id> IntoCheckedRange<'id> for Range<'id> {
    #[inline]
    fn into(self) -> Result<Range<'id, NonEmpty>, Range<'id>> {
        self.nonempty()
    }
}

impl<'id> IntoCheckedRange<'id> for Range<'id, NonEmpty> {
    #[inline]
    fn into(self) -> Result<Range<'id, NonEmpty>, Range<'id>> {
        Ok(self)
    }
}

/// Type alias for **N**on **E**mpty Range.
pub type NeRange<'id> = Range<'id, NonEmpty>;

impl<'id> Range<'id, NonEmpty> {
    #[inline(always)]
    pub fn first(&self) -> Index<'id> {
        Index { id: self.id, idx: self.start }
    }

    /// Return the middle index, rounding up.
    ///
    /// Produces `mid` where `mid = start + len / 2`.
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
    pub fn advance_(&self) -> Result<Range<'id, NonEmpty>, ()>
    {
        let mut next = *self;
        next.start += 1;
        if next.start < next.end {
            Ok(next)
        } else {
            Err(())
        }
    }

    /// Step this range forward, if the result is still a non-empty range.
    ///
    /// Return `true` if stepped successfully, `false` if the range would be empty.
    #[inline]
    pub fn advance(&mut self) -> bool
    {
        let mut next = *self;
        next.start += 1;
        if next.start < next.end {
            *self = next;
            true
        } else {
            false
        }
    }

    /// Step the range's end, if the result is still a non-empty range.
    ///
    /// Return `true` if stepped successfully, `false` if the range would be empty.
    #[inline]
    pub fn advance_back(&mut self) -> bool
    {
        let mut next = *self;
        next.end -= 1;
        if next.start < next.end {
            *self = next;
            true
        } else {
            false
        }
    }
}

impl<'id, P> IntoIterator for Range<'id, P> {
    type Item = Index<'id>;
    type IntoIter = RangeIter<'id>;
    #[inline]
    fn into_iter(self) -> RangeIter<'id> {
        RangeIter {
            id: self.id,
            start: self.start,
            end: self.end,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RangeIter<'id> {
    id: Id<'id>,
    start: usize,
    end: usize,
}

impl<'id> Iterator for RangeIter<'id> {
    type Item = Index<'id>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.end {
            let index = self.start;
            self.start += 1;
            Some(Index { id: PhantomData, idx: index })
        } else {
            None
        }
    }
}

impl<'id> DoubleEndedIterator for RangeIter<'id> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start < self.end {
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
          Array: Buffer<Target=[T]>,
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
    let indexer = Indexer { id: PhantomData, arr: arr };
    let indices = indexer.range();
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
    type Item = Range<'id, NonEmpty>;
    #[inline]
    fn next(&mut self) -> Option<Range<'id, NonEmpty>> {
        self.fs.next().map(|(i, j)| {
            debug_assert!(self.range.contains(i).is_some());
            debug_assert!(self.range.contains(j).is_some() || j == self.range.end);
            debug_assert!(i != j);
            unsafe {
                Range::from_ne(i, j)
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
            for elt in &mut data[part] {
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
            for (i, j) in it1.into_iter().zip(it2) {
                println!("{} {}", arr1[i], arr2[j]);
                //
                // println!("{} ", arr2.get(i));    // should be invalid to idx wrong source
                // println!("{} ", arr1.get(j));    // should be invalid to idx wrong source
            }
        });
    });

    // can hold onto the indices for later, as long they stay in the closure
    let _a = indices(arr1, |arr, mut r| {
        let mut it = r.into_iter();
        let a = it.next().unwrap();
        let b = it.next_back().unwrap();
        println!("{} {}", arr[a], arr[b]);
        // a    // should be invalid to return an index
    });
    //

    /*
    // can get references out, just not indices
    let (x, y) = indices(arr1, |arr, mut r| {
        println!("{:?}", arr.slice(r));
        let a = r.next().unwrap();
        let b = r.next_back().unwrap();
        (arr.get(a), arr.get(b))
    });
    println!("{} {}", x, y);
    */
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

#[cfg(test)]
fn range_insertion_sort<T, F>(v: &mut [T], mut less_than: F) where F: FnMut(&T, &T) -> bool {
    indices(v, move |mut v, r| {
        if let Ok(mut i) = r.nonempty() {
            while i.advance() {
                let jtail = v.scan_tail(i.first(), |j_elt| less_than(&v[i.first()], j_elt));
                v.rotate1(jtail);
            }
        }
    });
}

#[cfg(test)]
fn pointerindexing_insertion_sort<T, F>(v: &mut [T], mut less_than: F) where F: FnMut(&T, &T) -> bool {
    indices(v, move |mut v, _r| {
        for i in v.pointer_range() {
            let jtail = v.scan_tail_(i, |j_elt| less_than(&v[i], j_elt));
            v.rotate1_(jtail);
        }
    });
}

/*
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
*/

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
fn bench_range_insertion_sort_1024(b: &mut Bencher) {
    let mut data = [0; 1024];
    bench_data(&mut data);

    b.iter(|| {
        let mut d = data;
        range_insertion_sort(&mut d, |a, b| a < b);
    });
    b.bytes = mem::size_of_val(&data) as u64;
}

#[bench]
fn bench_range_insertion_sort_100(b: &mut Bencher) {
    let mut data = [0; 100];
    bench_data(&mut data);

    b.iter(|| {
        let mut d = data;
        range_insertion_sort(&mut d, |a, b| a < b);
    });
    b.bytes = mem::size_of_val(&data) as u64;
}

#[bench]
fn bench_pointer_insertion_sort_1024(b: &mut Bencher) {
    let mut data = [0; 1024];
    bench_data(&mut data);

    b.iter(|| {
        let mut d = data;
        pointerindexing_insertion_sort(&mut d, |a, b| a < b);
    });
    b.bytes = mem::size_of_val(&data) as u64;
}

#[bench]
fn bench_pointer_insertion_sort_100(b: &mut Bencher) {
    let mut data = [0; 100];
    bench_data(&mut data);

    b.iter(|| {
        let mut d = data;
        pointerindexing_insertion_sort(&mut d, |a, b| a < b);
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
        assert_eq!(&data[range], &[0, 0, 0]);
        let range = data.scan_head(range.last(), |elt| *elt != 0);
        assert_eq!(&data[range], &[0, 1, 2]);
    });
}
