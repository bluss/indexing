

// Modules

use std::cmp;
use std::ops;
use std::ptr;
use std::mem;

use std::fmt::{self, Debug};

use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use pointer::PIndex;
use index_error::IndexingError;
use index_error::index_error;
use std;
use super::Id;
use prelude::*;
use base::ProofAdd;

/// A marker trait for collections where we can safely vet indices
pub unsafe trait Buffer : Deref {
}

unsafe impl<'a, T> Buffer for &'a [T] { }
unsafe impl<'a, T> Buffer for &'a mut [T] { }

pub unsafe trait BufferMut : Buffer + DerefMut { }
unsafe impl<X: ?Sized> BufferMut for X where X: Buffer + DerefMut { }


/// A branded container, that allows access only to indices and ranges with
/// the exact same brand in the `'id` parameter.
pub struct Container<'id, Array, Mode = ()> {
    id: Id<'id>,
    arr: Array,
    mode: PhantomData<Mode>,
}

#[derive(Debug, Copy, Clone)]
pub enum OnlyIndex { }

impl<'id, Array, Mode> Debug for Container<'id, Array, Mode>
    where Array: Debug
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.arr.fmt(f)
    }
}

impl<'id, Array, Mode> Clone for Container<'id, Array, Mode>
    where Array: Clone
{
    fn clone(&self) -> Self {
        Container {
            id: self.id,
            arr: self.arr.clone(),
            mode: self.mode,
        }
    }
}

/// A branded index.
///
/// `Index<'id>` only indexes the container instantiated with the exact same
/// particular lifetime for the parameter `'id` at its inception from
/// the `indices()` constructor.
///
/// The type parameter `Proof` determines if the index is dereferenceable.
///
/// A `NonEmpty` index points to a valid element. An `Unknown` index is unknown,
/// or it points to an edge index (just past the end).
#[derive(Copy, Clone, Eq, PartialOrd)]
pub struct Index<'id, Proof = NonEmpty> {
    id: Id<'id>,
    index: usize,
    /// NonEmpty or Unknown
    proof: PhantomData<Proof>,
}

impl<'id, P> Index<'id, P> {
    #[inline(always)]
    unsafe fn new(index: usize) -> Index<'id, P> {
        debug_assert!(index as isize >= 0);
        Index { id: Id::default(), index: index, proof: PhantomData }
    }

    // FIXME: Is this a good idea? Incompatible with pointer representation.
    #[inline]
    pub fn integer(&self) -> usize { self.index }
}

impl<'id> Index<'id, NonEmpty> {
    /// Return the index directly after.
    pub fn after(self) -> Index<'id, Unknown> {
        unsafe {
            Index::new(self.index + 1)
        }
    }
}


impl<'id, P> Debug for Index<'id, P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Index({})", self.index)
    }
}

/// Index can only be compared with other indices of the same branding
impl<'id, P, Q> PartialEq<Index<'id, Q>> for Index<'id, P> {
    #[inline(always)]
    fn eq(&self, other: &Index<'id, Q>) -> bool {
        self.index == other.index
    }
}


impl<'id, Array, T, Mode> Container<'id, Array, Mode>
    where Array: Buffer<Target=[T]>
{
    #[inline]
    pub fn len(&self) -> usize {
        self.arr.len()
    }

    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.arr.as_ptr()
    }

    /// Convert the container into an only-indexing container.
    ///
    /// The container no longer allows pointer access. This unlocks
    /// some features.
    pub fn only_index(self) -> Container<'id, Array, OnlyIndex> {
        Container {
            id: self.id,
            arr: self.arr,
            mode: PhantomData,
        }
    }

    // Is this a good idea?
    /// Return the range [0, 0)
    #[inline]
    pub fn empty_range(&self) -> Range<'id> {
        unsafe {
            Range::from(0, 0)
        }
    }

    /// Return the full range of the Container.
    #[inline]
    pub fn range(&self) -> Range<'id> {
        unsafe {
            Range::from(0, self.len())
        }
    }

    #[inline]
    pub fn vet(&self, index: usize) -> Result<Index<'id>, IndexingError> {
        if index < self.len() {
            unsafe {
                Ok(Index::new(index))
            }
        } else {
            Err(index_error())
        }
    }

    #[inline]
    pub fn vet_range(&self, r: ops::Range<usize>) -> Result<Range<'id>, IndexingError> {
        if r.start <= r.end && r.end <= self.len() {
            unsafe {
                Ok(Range::from(r.start, r.end))
            }
        } else {
            Err(index_error())
        }
    }

    #[inline]
    pub fn split_at<P>(&self, index: Index<'id, P>) -> (Range<'id>, Range<'id, P>) {
        unsafe {
            (Range::from(0, index.index), Range::from_any(index.index, self.len()))
        }
    }

    /// Split in two ranges, where the first includes the `index` and the second
    /// starts with the following index.
    #[inline]
    pub fn split_after(&self, index: Index<'id>) -> (Range<'id, NonEmpty>, Range<'id>) {
        let mid = index.index + 1; // must be <= len since `index` is in bounds
        unsafe {
            (Range::from_ne(0, mid), Range::from(mid, self.len()))
        }
    }

    /// Split around the Range `r`: Return ranges corresponding to `0..r.start`
    /// and `r.end..`.
    ///
    /// So that input `r` and return values `(s, t)` cover the whole container
    /// in the order `s`, `r`, `t`.
    #[inline]
    pub fn split_around<P>(&self, r: Range<'id, P>) -> (Range<'id>, Range<'id>) {
        unsafe {
            (Range::from(0, r.start), Range::from(r.end, self.len()))
        }
    }


    /// Return the range before (not including) the index itself
    #[inline]
    pub fn before<P>(&self, index: Index<'id, P>) -> Range<'id> {
        unsafe {
            Range::from(0, index.index)
        }
    }

    /// Return the range after (not including) the index itself
    #[inline]
    pub fn after(&self, index: Index<'id>) -> Range<'id> {
        // in bounds because index + 1 is <= .len()
        unsafe {
            Range::from(index.index + 1, self.len())
        }
    }

    /// Increment `index`, if doing so would still be in bounds.
    ///
    /// Return `true` if the index was incremented.
    #[inline]
    pub fn forward(&self, index: &mut Index<'id>) -> bool {
        let i = index.index + 1;
        if i < self.len() {
            index.index = i;
            true
        } else { false }
    }

    /// Increment `index`, if doing so would still be in bounds.
    ///
    /// Return `true` if the index was incremented.
    #[inline]
    pub fn forward_by(&self, index: &mut Index<'id>, offset: usize) -> bool {
        let i = index.index + offset;
        if i < self.len() {
            index.index = i;
            true
        } else { false }
    }

    /// Increment `r`, clamping to the end of the Container.
    #[inline]
    pub fn forward_range_by<P>(&self, r: Range<'id, P>, offset: usize) -> Range<'id> {
        let start = r.start.saturating_add(offset);
        let end = r.end.saturating_add(offset);
        let len = self.len();
        unsafe {
            Range::from(cmp::min(len, start), cmp::min(len, end))
        }
    }

    /// Decrement `index`, if doing so would still be in bounds.
    ///
    /// Return `true` if the index was decremented.
    #[inline]
    pub fn backward(&self, index: &mut Index<'id>) -> bool {
        let i = index.index;
        if i > 0 {
            index.index = i - 1;
            true
        } else { false }
    }

    /// Examine the elements after `index` in order from lower indices towards higher.
    /// While the closure returns `true`, continue scan and include the scanned
    /// element in the range.
    ///
    /// Result always includes `index` in the range
    #[inline]
    pub fn scan_from<'b, F>(&'b self, index: Index<'id>, mut f: F) -> Range<'id, NonEmpty>
        where F: FnMut(&'b T) -> bool, T: 'b,
    {
        let mut end = index;
        for elt in &self[self.after(index)] {
            if !f(elt) {
                break;
            }
            end.index += 1;
        }
        end.index += 1;
        unsafe {
            Range::from_ne(index.index, end.index)
        }
    }

    /// Examine the elements before `index` in order from higher indices towards lower.
    /// While the closure returns `true`, continue scan and include the scanned
    /// element in the range.
    ///
    /// Result always includes `index` in the range.
    #[inline]
    pub fn scan_from_rev<'b, F>(&'b self, index: Index<'id>, mut f: F) -> Range<'id, NonEmpty>
        where F: FnMut(&'b T) -> bool, T: 'b
    {
        unsafe {
            let mut start = index;
            for elt in self[..index].iter().rev() {
                if !f(elt) {
                    break;
                }
                start.index -= 1;
            }
            Range::from_ne(start.index, index.index + 1)
        }
    }

    /// Examine the elements `range` in order from lower indices towards higher.
    /// While the closure returns `true`, continue scan and include the scanned
    /// element in the range.
    #[inline]
    pub fn scan_range<'b, F, P>(&'b self, range: Range<'id, P>, mut f: F)
        -> (Range<'id>, Range<'id>)
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
            (Range::from(range.start, end),
             Range::from(end, range.end))
        }
    }

    /// Swap elements at `i` and `j` (they may be equal).
    #[inline]
    pub fn swap(&mut self, i: Index<'id>, j: Index<'id>)
        where Array: BufferMut<Target=[T]>,
    {
        // ptr::swap is ok with equal pointers
        unsafe {
            ptr::swap(&mut self[i], &mut self[j])
        }
    }

    /// Rotate elements in the range `r` by one step to the right (towards higher indices)
    #[inline]
    pub fn rotate1_up<R>(&mut self, r: R)
        where Array: BufferMut<Target=[T]>,
              R: IntoCheckedRange<'id>
    {
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

    /// Rotate elements in the range `r` by one step to the left (towards lower indices)
    #[inline]
    pub fn rotate1_down<R>(&mut self, r: R)
        where Array: BufferMut<Target=[T]>,
              R: IntoCheckedRange<'id>
    {
        if let Ok(r) = r.into() {
            if r.first() != r.last() {
                unsafe {
                    let last_ptr = &mut self[r.last()] as *mut _;
                    let first_ptr = &mut self[r.first()] as *mut _;
                    let tmp = ptr::read(first_ptr);
                    ptr::copy(first_ptr.offset(1),
                              first_ptr,
                              r.len() - 1);
                    ptr::copy_nonoverlapping(&tmp, last_ptr, 1);
                    mem::forget(tmp);
                }
            }
        }
    }

    /// Index by two nonoverlapping ranges, where `r` is before `s`.
    #[inline]
    pub fn index_twice<P, Q>(&mut self, r: Range<'id, P>, s: Range<'id, Q>)
        -> Result<(&mut [T], &mut [T]), IndexingError>
        where Array: BufferMut<Target=[T]>,
    {
        if r.end <= s.start {
            let self_mut = self as *mut Self;
            unsafe {
                Ok((&mut (*self_mut)[r], &mut (*self_mut)[s]))
            }
        } else {
            Err(index_error())
        }
    }

    /// Zip by raw pointer (will be indentical if ranges have same starting point)
    pub fn zip_mut_raw<P, Q, F>(&mut self, r: Range<'id, P>, s: Range<'id, Q>, mut f: F)
        where F: FnMut(*mut T, *mut T),
              Array: BufferMut<Target=[T]>,
    {
        let len = cmp::min(r.len(), s.len());
        for i in 0..len {
            unsafe {
                f(
                    self.arr.get_unchecked_mut(r.start + i),
                    self.arr.get_unchecked_mut(s.start + i)
                )
            }
        }
    }
}


/// `&self[i]` where `i` is an `Index<'id>`.
impl<'id, T, Array, M> ops::Index<Index<'id>> for Container<'id, Array, M>
    where Array: Buffer<Target=[T]>
{
    type Output = T;
    #[inline(always)]
    fn index(&self, index: Index<'id>) -> &T {
        unsafe {
            self.arr.get_unchecked(index.index)
        }
    }
}

/// `&mut self[i]` where `i` is an `Index<'id>`.
impl<'id, T, Array, M> ops::IndexMut<Index<'id>> for Container<'id, Array, M>
    where Array: BufferMut<Target=[T]>,
{
    #[inline(always)]
    fn index_mut(&mut self, index: Index<'id>) -> &mut T {
        unsafe {
            self.arr.get_unchecked_mut(index.index)
        }
    }
}

/// `&self[r]` where `r` is a `Range<'id>`.
impl<'id, T, Array, P, M> ops::Index<Range<'id, P>> for Container<'id, Array, M>
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

/// `&mut self[r]` where `r` is a `Range<'id>`.
impl<'id, T, Array, P, M> ops::IndexMut<Range<'id, P>> for Container<'id, Array, M>
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

/// `&self[i..]` where `i` is an `Index<'id, P>` which may be an edge index.
impl<'id, T, P, Array, M> ops::Index<ops::RangeFrom<Index<'id, P>>> for Container<'id, Array, M>
    where Array: Buffer<Target=[T]>,
{
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: ops::RangeFrom<Index<'id, P>>) -> &[T] {
        let i = r.start.index;
        unsafe {
            std::slice::from_raw_parts(
                self.arr.as_ptr().offset(i as isize),
                self.len() - i)
        }
    }
}

/// `&mut self[i..]` where `i` is an `Index<'id, P>` which may be an edge index.
impl<'id, T, P, Array, M> ops::IndexMut<ops::RangeFrom<Index<'id, P>>> for Container<'id, Array, M>
    where Array: BufferMut<Target=[T]>,
{
    #[inline(always)]
    fn index_mut(&mut self, r: ops::RangeFrom<Index<'id, P>>) -> &mut [T] {
        let i = r.start.index;
        unsafe {
            std::slice::from_raw_parts_mut(
                self.arr.as_mut_ptr().offset(i as isize),
                self.len() - i)
        }
    }
}

/// `&self[..i]` where `i` is an `Index<'id, P>`, which may be an edge index.
impl<'id, T, P, Array, M> ops::Index<ops::RangeTo<Index<'id, P>>> for Container<'id, Array, M>
    where Array: Buffer<Target=[T]>,
{
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: ops::RangeTo<Index<'id, P>>) -> &[T] {
        let i = r.end.index;
        unsafe {
            std::slice::from_raw_parts(self.arr.as_ptr(), i)
        }
    }
}

/// `&mut self[..i]` where `i` is an `Index<'id, P>`, which may be an edge index.
impl<'id, T, P, Array, M> ops::IndexMut<ops::RangeTo<Index<'id, P>>> for Container<'id, Array, M>
    where Array: BufferMut<Target=[T]>
{
    #[inline(always)]
    fn index_mut(&mut self, r: ops::RangeTo<Index<'id, P>>) -> &mut [T] {
        let i = r.end.index;
        unsafe {
            std::slice::from_raw_parts_mut(self.arr.as_mut_ptr(), i)
        }
    }
}

/// `&self[..]`
impl<'id, T, Array, M> ops::Index<ops::RangeFull> for Container<'id, Array, M>
    where Array: Buffer<Target=[T]>,
{
    type Output = [T];
    #[inline(always)]
    fn index(&self, _: ops::RangeFull) -> &[T] {
        &self.arr[..]
    }
}

/// `&mut self[..]`
impl<'id, T, Array> ops::IndexMut<ops::RangeFull> for Container<'id, Array>
    where Array: BufferMut<Target=[T]>,
{
    #[inline(always)]
    fn index_mut(&mut self, _: ops::RangeFull) -> &mut [T] {
        &mut self.arr[..]
    }
}


/*
// ###### Bounds checking impls #####
impl<'id, 'a, T> ops::Index<ops::Range<usize>> for Container<'id, &'a mut [T]> {
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: ops::Range<usize>) -> &[T] {
        &self.arr[r]
    }
}

impl<'id, 'a, T> ops::Index<ops::RangeFrom<usize>> for Container<'id, &'a mut [T]> {
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: ops::RangeFrom<usize>) -> &[T] {
        &self.arr[r]
    }
}

impl<'id, 'a, T> ops::Index<ops::RangeTo<usize>> for Container<'id, &'a mut [T]> {
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: ops::RangeTo<usize>) -> &[T] {
        &self.arr[r]
    }
}

impl<'id, 'a, T> ops::IndexMut<ops::Range<usize>> for Container<'id, &'a mut [T]> {
    #[inline(always)]
    fn index_mut(&mut self, r: ops::Range<usize>) -> &mut [T] {
        &mut self.arr[r]
    }
}

impl<'id, 'a, T> ops::IndexMut<ops::RangeFrom<usize>> for Container<'id, &'a mut [T]> {
    #[inline(always)]
    fn index_mut(&mut self, r: ops::RangeFrom<usize>) -> &mut [T] {
        &mut self.arr[r]
    }
}

impl<'id, 'a, T> ops::IndexMut<ops::RangeTo<usize>> for Container<'id, &'a mut [T]> {
    #[inline(always)]
    fn index_mut(&mut self, r: ops::RangeTo<usize>) -> &mut [T] {
        &mut self.arr[r]
    }
}
// ####
*/

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

impl<'id, 'a, T, Array> ops::Index<PIndex<'id, T>> for Container<'id, Array>
    where Array: Buffer<Target=[T]>,
{
    type Output = T;
    #[inline(always)]
    fn index(&self, r: PIndex<'id, T>) -> &T {
        ptr_iselement(&self.arr[..], r.ptr());
        unsafe {
            &*r.ptr()
        }
    }
}

impl<'id, T, P, Array> ops::Index<ops::RangeTo<PIndex<'id, T, P>>> for Container<'id, Array>
    where Array: Buffer<Target=[T]>,
{
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: ops::RangeTo<PIndex<'id, T, P>>) -> &[T] {
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
    /// NonEmpty or Unknown
    proof: PhantomData<Proof>,
}

impl<'id, P> Copy for Range<'id, P> { }
impl<'id, P> Clone for Range<'id, P> {
    #[inline]
    fn clone(&self) -> Self { *self }
}

impl<'id, P, Q> PartialEq<Range<'id, Q>> for Range<'id, P> {
    fn eq(&self, other: &Range<'id, Q>) -> bool {
        self.start == other.start && self.end == other.end
    }
}

impl<'id, P> Eq for Range<'id, P> { }

impl<'id> Range<'id> {
    #[inline(always)]
    unsafe fn from(start: usize, end: usize) -> Range<'id> {
        debug_assert!(start <= end);
        Range { id: Id::default(), start: start, end: end, proof: PhantomData }
    }
}

impl<'id> Range<'id, NonEmpty> {
    #[inline(always)]
    unsafe fn from_ne(start: usize, end: usize) -> Range<'id, NonEmpty> {
        debug_assert!(start < end);
        Range { id: Id::default(), start: start, end: end, proof: PhantomData }
    }
}

impl<'id, P> Range<'id, P> {
    #[inline(always)]
    unsafe fn from_any(start: usize, end: usize) -> Range<'id, P> {
        debug_assert!(start <= end);
        Range { id: Id::default(), start: start, end: end, proof: PhantomData }
    }

    /// Return the length of the range.
    #[inline]
    pub fn len(&self) -> usize { self.end - self.start }

    /// Return `true` if the range is empty.
    #[inline]
    pub fn is_empty(&self) -> bool { self.start >= self.end }

    /// Try to create a proof that the Range is nonempty; return
    /// a `Result` where the `Ok` branch carries a non-empty Range.
    #[inline]
    pub fn nonempty(&self) -> Result<Range<'id, NonEmpty>, IndexingError> {
        unsafe {
            if !self.is_empty() {
                Ok(mem::transmute(*self))
            } else {
                Err(index_error())
            }
        }
    }

    /// Return the start index.
    #[inline]
    pub fn start(&self) -> usize { self.start }

    /// Return the end index.
    #[inline]
    pub fn end(&self) -> usize { self.end }

    /// Split the range in half, with the upper middle index landing in the
    /// latter half. Proof of length `P` transfers to the latter half.
    #[inline]
    pub fn split_in_half(self) -> (Range<'id>, Range<'id, P>) {
        let mid = (self.end - self.start) / 2 + self.start;
        unsafe {
            (Range::from(self.start, mid), Range::from_any(mid, self.end))
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

    /// `abs_index` is an absolute index
    #[inline]
    pub fn contains(&self, abs_index: usize) -> Option<Index<'id>> {
        unsafe {
            if abs_index >= self.start && abs_index < self.end {
                Some(Index::new(abs_index))
            } else { None }
        }
    }

    /// Return an iterator that divides the range in `n` parts, in as
    /// even length chunks as possible.
    #[inline]
    pub fn subdivide(&self, n: usize) -> Subdivide<'id> {
        unsafe {
            Subdivide {
                fs: FracStep::new(self.start, self.end, n),
                range: Range::from(self.start, self.end),
            }
        }
    }

    /// Join together two adjacent ranges (they must be exactly touching, and
    /// in left to right order).
    pub fn join<Q>(&self, other: Range<'id, Q>) -> Result<Range<'id, <(P, Q) as ProofAdd>::Sum>, IndexingError>
        where (P, Q): ProofAdd
    {
        // FIXME: type algebra, use P + Q in return type
        if self.end == other.start {
            unsafe {
                Ok(Range::from_any(self.start, other.end))
            }
        } else {
            Err(index_error())
        }
    }

    /// Extend the range to the end of `other`, including any space in between
    pub fn join_cover<Q>(&self, other: Range<'id, Q>) -> Range<'id, <(P, Q) as ProofAdd>::Sum>
        where (P, Q): ProofAdd,
    {
        let end = cmp::max(self.end, other.end);
        unsafe {
            Range::from_any(self.start, end)
        }
    }

    /// Extend the range to start and end of `other`, including any space in between
    pub fn join_cover_both<Q>(&self, other: Range<'id, Q>) -> Range<'id, <(P, Q) as ProofAdd>::Sum>
        where (P, Q): ProofAdd,
    {
        let start = cmp::min(self.start, other.start);
        let end = cmp::max(self.end, other.end);
        unsafe {
            Range::from_any(start, end)
        }
    }

    #[inline]
    pub fn as_range(&self) -> std::ops::Range<usize> { self.start..self.end }

    /// Return two empty ranges, at the front and the back of the range respectively
    #[inline]
    pub fn frontiers(&self) -> (Range<'id>, Range<'id>) {
        let s = self.start;
        let e = self.end;
        unsafe {
            (Range::from(s, s), Range::from(e, e))
        }
    }

    /// Increment `index`, if doing so would still be before the end of the range
    ///
    /// Return `true` if the index was incremented.
    #[inline]
    pub fn forward_by(&self, index: &mut Index<'id>, offset: usize) -> bool {
        let i = index.index + offset;
        if i < self.end {
            index.index = i;
            true
        } else { false }
    }

    /// Increment `r`, clamping to the end of `self`.
    #[inline]
    pub fn forward_range_by<Q>(&self, r: Range<'id, Q>, offset: usize) -> Range<'id> {
        // XXX saturating_add is faster in real use, for some reason
        let max = self.end;
        let start = cmp::min(r.start.saturating_add(offset), max);
        let end = cmp::min(r.end.saturating_add(offset), max);
        unsafe {
            Range::from(start, end)
        }
    }

    #[inline]
    pub fn no_proof(&self) -> Range<'id> {
        unsafe {
            mem::transmute(*self)
        }
    }
}

impl<'id, P> Debug for Range<'id, P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Range({}, {})", self.start, self.end)
    }
}

pub trait IntoCheckedRange<'id> : Sized {
    fn into(self) -> Result<Range<'id, NonEmpty>, IndexingError>;
}

impl<'id> IntoCheckedRange<'id> for Range<'id> {
    #[inline]
    fn into(self) -> Result<Range<'id, NonEmpty>, IndexingError> {
        self.nonempty()
    }
}

impl<'id> IntoCheckedRange<'id> for Range<'id, NonEmpty> {
    #[inline]
    fn into(self) -> Result<Range<'id, NonEmpty>, IndexingError> {
        Ok(self)
    }
}

impl<'id, P> Range<'id, P> {
    /// Return the first index in the range (The index is accessible if the range
    /// is `NonEmpty`).
    #[inline(always)]
    pub fn first(&self) -> Index<'id, P> {
        unsafe {
            Index::new(self.start)
        }
    }

    /// Return the middle index, rounding up.
    ///
    /// Produces `mid` where `mid = start + len / 2`.
    #[inline]
    pub fn upper_middle(&self) -> Index<'id, P> {
        let mid = self.len() / 2 + self.start;
        unsafe {
            Index::new(mid)
        }
    }

    /// Return the index past the end of the range.
    #[inline]
    pub fn past_the_end(self) -> Index<'id, Unknown> {
        unsafe {
            Index::new(self.end)
        }
    }
}

impl<'id> Range<'id, NonEmpty> {
    /// Return the middle index, rounding down.
    ///
    /// Produces `mid` where `mid = start + (len - 1)/ 2`.
    #[inline]
    pub fn lower_middle(&self) -> Index<'id> {
        // nonempty, so len - 1 >= 0
        let mid = (self.len() - 1) / 2 + self.start;
        unsafe {
            Index::new(mid)
        }
    }


    #[inline]
    pub fn last(&self) -> Index<'id> {
        unsafe {
            Index::new(self.end - 1)
        }
    }

    #[inline]
    pub fn tail(self) -> Range<'id> {
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
    pub fn advance_(&self) -> Result<Range<'id, NonEmpty>, IndexingError>
    {
        let mut next = *self;
        next.start += 1;
        if next.start < next.end {
            Ok(next)
        } else {
            Err(index_error())
        }
    }

    /// Increase the range's start, if the result is still a non-empty range.
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

    /// Increase the range's start, if the result is still a non-empty range.
    ///
    /// Return `true` if stepped successfully, `false` if the range would be empty.
    #[inline]
    pub fn advance_by(&mut self, offset: usize) -> bool
    {
        let mut next = *self;
        next.start = next.start.saturating_add(offset);
        if next.start < next.end {
            *self = next;
            true
        } else {
            false
        }
    }

    /// Decrease the range's end, if the result is still a non-empty range.
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

/// An iterator over the indices in a range.
///
/// Iterator element type is `Index<'id>`.
#[derive(Copy, Clone, Debug)]
pub struct RangeIter<'id> {
    id: Id<'id>,
    start: usize,
    end: usize,
}

impl<'id> RangeIter<'id> {
    #[inline]
    pub fn into_range(&self) -> Range<'id> {
        unsafe {
            Range::from(self.start, self.end)
        }
    }
}

impl<'id> Iterator for RangeIter<'id> {
    type Item = Index<'id>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.end {
            let index = self.start;
            self.start += 1;
            unsafe {
                Some(Index::new(index))
            }
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
            unsafe {
                Some(Index::new(self.end))
            }
        } else {
            None
        }
    }
}

/// Create an indexing scope for a container.
///
/// The indexing scope is a closure that is passed a unique lifetime for
/// the parameter `'id`; this lifetime brands the container and its indices
/// and ranges, so that they are trusted to be in bounds.
///
/// Indices and ranges branded with `'id` can not leave the closure. The
/// container can only be accessed and mutated through the `Container` wrapper
/// passed as the first argument to the indexing scope.
#[inline]
pub fn indices<Array, F, Out, T>(arr: Array, f: F) -> Out
    where F: for<'id> FnOnce(Container<'id, Array>, Range<'id>) -> Out,
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
    let indexer = Container { id: Id::default(), arr: arr, mode: PhantomData };
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

/// `Subdivide` is an iterator of evenly sized nonempty, nonoverlapping ranges
#[derive(Copy, Clone, Debug)]
pub struct Subdivide<'id> {
    range: Range<'id>,
    fs: FracStep,
}

impl<'id> Iterator for Subdivide<'id> {
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
        for (index, part) in r.subdivide(3).enumerate() {
            for elt in &mut data[part] {
                *elt = index;
            }
        }
    });
    assert_eq!(&data[..], &[0, 0, 1, 1, 1, 2, 2, 2]);
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


#[test]
fn test_scan() {
    let mut data = [0, 0, 0, 1, 2];
    indices(&mut data[..], |data, r| {
        let r = r.nonempty().unwrap();
        let range = data.scan_from(r.first(), |elt| *elt == 0);
        assert_eq!(&data[range], &[0, 0, 0]);
        let range = data.scan_from(range.last(), |elt| *elt != 0);
        assert_eq!(&data[range], &[0, 1, 2]);
    });
}

#[test]
fn test_nonempty() {
    let mut data = [0, 1, 2, 3, 4, 5];
    indices(&mut data[..], |data, r| {
        let mut r = r.nonempty().unwrap();
        assert_eq!(data[r.first()], 0);
        assert_eq!(data[r.lower_middle()], 2);
        assert_eq!(data[r.upper_middle()], 3);
        assert_eq!(data[r.last()], 5);

        assert!(r.advance());
        assert_eq!(data[r.first()], 1);
        assert_eq!(data[r.lower_middle()], 3);
        assert_eq!(data[r.upper_middle()], 3);
        assert_eq!(data[r.last()], 5);

        assert!(r.advance());
        assert_eq!(data[r.first()], 2);
        assert_eq!(data[r.lower_middle()], 3);
        assert_eq!(data[r.upper_middle()], 4);
        assert_eq!(data[r.last()], 5);

        // skip to end
        while r.advance() { }
        assert_eq!(data[r.first()], 5);
        assert_eq!(data[r.lower_middle()], 5);
        assert_eq!(data[r.upper_middle()], 5);
        assert_eq!(data[r.last()], 5);
    });
}

#[test]
fn test_contains() {
    let mut data = [0, 1, 2, 3, 4, 5];
    indices(&mut data[..], |data, r| {
        for i in 0..data.len() {
            assert!(r.contains(i).is_some());
            assert_eq!(r.contains(i).unwrap(), data.vet(i).unwrap());
        }
        assert!(r.contains(r.len()).is_none());
        assert!(data.vet(r.len()).is_err());
    });
}

#[test]
fn test_is_send_sync() {
    fn _is_send_sync<T: Send + Sync>() { }

    fn _test<'id>() {
        _is_send_sync::<Id<'id>>();
        _is_send_sync::<Index<'id>>();
        _is_send_sync::<Range<'id>>();
    }
}
