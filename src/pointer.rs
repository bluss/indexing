//! Pointer-based inbounds intervals and element references.
//!
//! These are safe abstractions built upon raw pointers instead of indices.
//! Which choice of Range, PRange or PSlice generates best code depends on
//! the algorithm.
//!
//! All element read/write access still goes through a nominal borrow of the
//! container with the correct brand; this ensures the usual & vs &mut borrowing
//! rules apply.
//!
//! The pointer type `PIndex` has a proof parameter just like the range;
//! this allows us to represent the one-past-the-end pointer.

use std::cmp::min;
use std::marker::PhantomData;
use std::mem;
use std::ptr;
use std::ops;
use std::slice::{from_raw_parts, from_raw_parts_mut};

use scope;
use super::Id;
use super::{NonEmpty, Container};
use {Unknown};
use IndexingError;
use index_error::index_error;

use pointer_ext::PointerExt;
use proof::Provable;
use container_traits::*;
use ContainerPrivate;

/// `PIndex` is a pointer to a location
///
/// It carries a proof (type parameter `Proof`) which when `NonEmpty`, means
/// it points to a valid element, `Unknown` is an unknown or edge pointer
/// (it can be a one-past-the-end pointer).
#[derive(Debug)]
pub struct PIndex<'id, T, Proof = NonEmpty> {
    id: Id<'id>,
    idx: *const T,
    proof: PhantomData<Proof>,
}

impl<'id, T, P> PIndex<'id, T, P> {
    unsafe fn new(p: *const T) -> Self {
        PIndex {
            id: Id::default(),
            idx: p,
            proof: PhantomData,
        }
    }
}

impl<'id, T> PIndex<'id, T, NonEmpty> {
    unsafe fn inbounds(p: *const T) -> Self {
        PIndex {
            id: Id::default(),
            idx: p,
            proof: PhantomData,
        }
    }
}

impl<'id, T, P> Copy for PIndex<'id, T, P> { }
impl<'id, T, P> Clone for PIndex<'id, T, P> {
    fn clone(&self) -> Self { *self }
}

impl<'id, T, P> PIndex<'id, T, P> {
    #[inline(always)]
    pub fn ptr(self) -> *const T {
        self.idx
    }

    #[inline(always)]
    pub fn ptr_mut(self) -> *mut T {
        self.idx as *mut T
    }
}

impl<'id, T> PIndex<'id, T, NonEmpty> {
    pub fn after(self) -> PIndex<'id, T, Unknown> {
        unsafe {
            PIndex::new(self.idx.offset(1))
        }
    }
}

impl<'id, T, P> PartialEq for PIndex<'id, T, P> {
    fn eq(&self, rhs: &Self) -> bool {
        self.idx == rhs.idx
    }
}
impl<'id, T, P> Eq for PIndex<'id, T, P> { }

/// `PRange` is a pointer-based valid range with start and end pointer
/// representation.
#[derive(Debug)]
pub struct PRange<'id, T, Proof = Unknown> {
    id: Id<'id>,
    start: *const T,
    end: *const T,
    /// NonEmpty or Unknown
    proof: PhantomData<Proof>,
}

impl<'id, T, P> Copy for PRange<'id, T, P> { }
impl<'id, T, P> Clone for PRange<'id, T, P> {
    fn clone(&self) -> Self { *self }
}

impl<'id, T, P, Q> PartialEq<PRange<'id, T, Q>> for PRange<'id, T, P> {
    fn eq(&self, rhs: &PRange<'id, T, Q>) -> bool {
        self.start == rhs.start && self.end == rhs.end
    }
}
impl<'id, T, P> Eq for PRange<'id, T, P> { }


/// return the number of steps between a and b
fn ptrdistance<T>(a: *const T, b: *const T) -> usize {
    debug_assert!(a as usize >= b as usize);
    (a as usize - b as usize) / mem::size_of::<T>()
}

impl<'id, T, P> PRange<'id, T, P> {
    #[inline(always)]
    unsafe fn new(start: *const T, end: *const T) -> Self {
        debug_assert!(end as usize >= start as usize);
        PRange { id: Id::default(), start: start, end: end, proof: PhantomData }
    }

    #[inline]
    pub fn len(self) -> usize { ptrdistance(self.end, self.start) }

    #[inline]
    pub fn is_empty(self) -> bool { self.start == self.end }

    /// Check if the range is empty. `NonEmpty` ranges have extra methods.
    #[inline]
    pub fn nonempty(&self) -> Result<PRange<'id, T, NonEmpty>, IndexingError>
    {
        unsafe {
            if !self.is_empty() {
                Ok(PRange::new(self.start, self.end))
            } else {
                Err(index_error())
            }
        }
    }

    /// Split the range in half, with the upper middle index landing in the
    /// latter half. Proof of nonemptiness `P` transfers to the latter half.
    #[inline]
    pub fn split_in_half(self) -> (PRange<'id, T>, PRange<'id, T, P>) {
        unsafe {
            let mid_offset = self.len() / 2;
            let mid = self.start.offset(mid_offset as isize);
            (PRange::new(self.start, mid), PRange::new(mid, self.end))
        }
    }
}

impl<'id, T, P> PRange<'id, T, P> {
    #[inline]
    pub fn first(self) -> PIndex<'id, T, P> {
        unsafe {
            PIndex::new(self.start)
        }
    }

    /// Return the middle index, rounding up on even
    #[inline]
    pub fn upper_middle(self) -> PIndex<'id, T, P> {
        unsafe {
            let mid = ptrdistance(self.end, self.start) / 2;
            PIndex::new(self.start.offset(mid as isize))
        }
    }

    #[inline]
    pub fn past_the_end(self) -> PIndex<'id, T, Unknown> {
        unsafe {
            PIndex::new(self.end)
        }
    }
}

impl<'id, T> PRange<'id, T, NonEmpty> {
    #[inline]
    pub fn last(self) -> PIndex<'id, T> {
        unsafe {
            PIndex::inbounds(self.end.offset(-1))
        }
    }

    #[inline]
    pub fn tail(self) -> PRange<'id, T> {
        // in bounds since it's nonempty
        unsafe {
            PRange::new(self.start.offset(1), self.end)
        }
    }

    #[inline]
    pub fn init(self) -> PRange<'id, T> {
        // in bounds since it's nonempty
        unsafe {
            PRange::new(self.start, self.end.offset(-1))
        }
    }

    /// Increase the range's start, if the result is still a non-empty range.
    ///
    /// Return `true` if stepped successfully, `false` if the range would be empty.
    #[inline]
    pub fn advance(&mut self) -> bool
    {
        unsafe {
            // always in bounds because the range is nonempty
            let next_ptr = self.start.offset(1);
            if next_ptr != self.end {
                self.start = next_ptr;
                true
            } else {
                false
            }
        }
    }

    /// Decrease the range's end, if the result is still a non-empty range.
    ///
    /// Return `true` if stepped successfully, `false` if the range would be empty.
    #[inline]
    pub fn advance_back(&mut self) -> bool
    {
        unsafe {
            // always in bounds because the range is nonempty
            let next_end = self.end.offset(-1);
            if self.start != next_end {
                self.end = next_end;
                true
            } else {
                false
            }
        }
    }
}

impl<'id, T, Array> Container<'id, Array> where Array: Contiguous<Item=T> {
    #[inline]
    pub fn pointer_range(&self) -> PRange<'id, T> {
        unsafe {
            let start = self.begin();
            let end = self.end();
            PRange::new(start, end)
        }
    }

    pub fn pointer_slice(&self) -> PSlice<'id, T> {
        unsafe {
            let start = self.begin();
            PSlice::new(start, self.len())
        }
    }

    #[inline]
    pub fn pointer_range_of<P, R>(&self, r: R) -> PRange<'id, T>
        where R: OnePointRange<Index=PIndex<'id, T, P>>,
    {
        debug_assert!(!(r.start().is_some() && r.end().is_some()));
        unsafe {
            let start = r.start().map_or(self.begin(), PIndex::ptr);
            let end = r.end().map_or(self.end(), PIndex::ptr);
            PRange::new(start, end)
        }
    }

    #[inline]
    pub fn nonempty_range<P, Q>(&self, a: PIndex<'id, T, P>, b: PIndex<'id, T, Q>)
        -> Result<PRange<'id, T, NonEmpty>, IndexingError>
    {
        if (a.idx as usize) < b.idx as usize {
            unsafe {
                Ok(PRange::new(a.idx, b.idx))
            }
        } else {
            Err(index_error())
        }
    }

    fn begin(&self) -> *const T {
        self.array().begin()
    }
    fn end(&self) -> *const T {
        self.array().end()
    }

    /// Return the distance (in number of elements) from the 
    /// start of the container to the pointer.
    pub fn distance_to<P>(&self, ptr: PIndex<'id, T, P>) -> usize {
        ptrdistance(ptr.ptr(), self.begin())
    }

    /// Examine the elements before `index` in order from higher indices towards lower.
    /// While the closure returns `true`, continue scan and include the scanned
    /// element in the range.
    ///
    /// Result always includes `index` in the range
    #[inline]
    pub fn scan_tail_<P, F>(&self, index: PIndex<'id, T, P>, mut f: F) -> PRange<'id, T, P>
        where F: FnMut(&T) -> bool
    {
        unsafe {
            let container_start = self.pointer_range().start;
            let mut end = index.ptr();
            loop {
                if container_start == end {
                    break;
                }
                if !f(&*end.offset(-1)) {
                    break;
                }
                end.dec();
            }
            PRange::new(end, index.ptr().offset(1))
        }
    }

    /// Examine the elements `range` in order from lower towards higher.
    /// While the closure returns `true`, continue scan and include the scanned
    /// element in the range.
    #[inline]
    pub fn scan_pointer_range<'b, F, P>(&'b self, range: PRange<'id, T, P>, mut f: F)
        -> (PRange<'id, T>, PRange<'id, T>)
        where F: FnMut(&'b T) -> bool, T: 'b,
    {
        unsafe {
            let mut ptr = range.start;
            while ptr != range.end {
                if !f(&*ptr) {
                    break;
                }
                ptr.inc();
            }
            (PRange::new(range.start, ptr),
             PRange::new(ptr, range.end))
        }
    }

    #[inline]
    pub fn split_at_pointer<P>(&self, index: PIndex<'id, T, P>)
        -> (PRange<'id, T>, PRange<'id, T, P>) {
        unsafe {
            let pr = self.pointer_range();
            (PRange::new(pr.start, index.idx),
             PRange::new(index.idx, pr.end))
        }
    }

    /// Examine the elements `range` in order from higher towards lower
    /// While the closure returns `true`, continue scan and include the scanned
    /// element in the range.
    #[inline]
    pub fn scan_pointer_range_rev<'b, F, P>(&'b self, range: PRange<'id, T, P>, mut f: F)
        -> (PRange<'id, T>, PRange<'id, T>)
        where F: FnMut(&'b T) -> bool, T: 'b,
    {
        unsafe {
            let mut ptr = range.end;
            while ptr != range.start {
                if !f(&*ptr.offset(-1)) {
                    break;
                }
                ptr.dec();
            }
            (PRange::new(range.start, ptr),
             PRange::new(ptr, range.end))
        }
    }
}
impl<'id, T, Array> Container<'id, Array> where Array: Contiguous<Item=T> {
    #[inline]
    pub fn split_container_at_pointer<P, F, Out>(&mut self, index: PIndex<'id, T, P>, f: F) -> Out
        //-> (PRange<'id, T>, PRange<'id, T, P>)
        where F: for<'id1, 'id2> FnOnce(Container<'id1, &mut [T]>, PRange<'id1, T>,
                                        Container<'id2, &mut [T]>, PRange<'id2, T, P>) -> Out,
    {
        unsafe {
            let mid = self.distance_to(index);
            let pr = self.pointer_range();
            let ptr1 = pr.start as *mut _;
            let ptr2 = index.idx as *mut _;
            let s1 = from_raw_parts_mut(ptr1, mid);
            let s2 = from_raw_parts_mut(ptr2, pr.len() - mid);
            scope(s1, move |i1| {
                scope(s2, move |i2| {
                    let r1 = PRange::new(pr.start, index.idx);
                    let r2 = PRange::new(index.idx, pr.end);
                    f(i1, r1, i2, r2)
                })
            })
        }
    }
}


/// Pointer-based zip (lock step iteration) of two ranges from
/// two containers.
pub fn zip<'id1, 'id2, C1, C2, R1, R2, F>(r1: R1, c1: C1, r2: R2, c2: C2, mut f: F)
    where C1: ContainerRef<'id1>,
          R1: PointerRange<'id1, Item=C1::Item>,
          C2: ContainerRef<'id2>,
          R2: PointerRange<'id2, Item=C2::Item>,
          F: FnMut(C1::Ref, C2::Ref),
{
    let _ = c1;
    let _ = c2;
    let len = min(r1.len(), r2.len());
    unsafe {
        let end = r1.ptr().offset(len as isize);
        let mut ptr1 = r1.ptr();
        let mut ptr2 = r2.ptr();
        while ptr1 != end {
            f(C1::dereference(ptr1), C2::dereference(ptr2));
            ptr1.inc();
            ptr2.inc();
        }
    }
}

/// Unsafe because: Must only be implemented by a range branded by `'id`.
pub unsafe trait PointerRange<'id> : Copy {
    type Item;
    fn ptr(self) -> *const Self::Item;
    fn end_ptr(self) -> *const Self::Item;
    fn len(self) -> usize;
}

unsafe impl<'id, T, P> PointerRange<'id> for PRange<'id, T, P>
{
    type Item = T;
    fn ptr(self) -> *const Self::Item { self.start }
    fn end_ptr(self) -> *const Self::Item { self.end }
    fn len(self) -> usize { self.len() }
}

unsafe impl<'id, T, P> PointerRange<'id> for PSlice<'id, T, P>
{
    type Item = T;
    fn ptr(self) -> *const Self::Item { self.start }
    fn end_ptr(self) -> *const Self::Item {
        unsafe {
            self.start.offset(self.len() as isize)
        }
    }
    fn len(self) -> usize { self.len() }
}

pub trait ContainerRef<'id> {
    type Item;
    type Ref;

    unsafe fn dereference(ptr: *const Self::Item) -> Self::Ref;
}

impl<'id, 'a, Array, T: 'a> ContainerRef<'id> for &'a Container<'id, Array>
    where Array: Contiguous<Item=T>,
{
    type Item = T;
    type Ref = &'a T;

    unsafe fn dereference(ptr: *const Self::Item) -> Self::Ref {
        &*ptr
    }
}

impl<'id, 'a, Array, T: 'a> ContainerRef<'id> for &'a mut Container<'id, Array>
    where Array: ContiguousMut<Item=T>,
{
    type Item = T;
    type Ref = &'a mut T;

    unsafe fn dereference(ptr: *const Self::Item) -> Self::Ref {
        &mut *(ptr as *mut T)
    }
}

impl<'id, T, Array> Container<'id, Array> where Array: ContiguousMut<Item=T> {

    /// Rotate elements in the range by one step to the right (towards higher indices)
    #[inline]
    pub fn rotate1_prange(&mut self, r: PRange<'id, T, NonEmpty>) {
        unsafe {
            let last_ptr = r.last().ptr();
            let first_ptr = r.first().ptr_mut();
            if first_ptr as *const _ == last_ptr {
                return;
            }
            let tmp = ptr::read(last_ptr);
            ptr::copy(first_ptr,
                      first_ptr.offset(1),
                      r.len() - 1);
            ptr::copy_nonoverlapping(&tmp, first_ptr, 1);
            mem::forget(tmp);
        }
    }

    /// Swap elements at `i` and `j` (they may be equal).
    #[inline(always)]
    pub fn swap_ptr(&mut self, i: PIndex<'id, T>, j: PIndex<'id, T>) {
        // ptr::swap is ok with equal pointers
        unsafe {
            ptr::swap(i.ptr_mut(), j.ptr_mut())
        }
    }

}

pub struct PRangeIter<'id, T>(PRange<'id, T, Unknown>);

impl<'id, T, P> IntoIterator for PRange<'id, T, P> {
    type Item = PIndex<'id, T>;
    type IntoIter = PRangeIter<'id, T>;
    fn into_iter(self) -> Self::IntoIter {
        PRangeIter(self.no_proof())
    }
}

impl<'id, T> Iterator for PRangeIter<'id, T> {
    type Item = PIndex<'id, T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.0.start != self.0.end {
            let index = self.0.start;
            unsafe {
                //assume(!index.is_null());
                self.0.start = self.0.start.offset(1);
                Some(PIndex::inbounds(index))
            }
        } else {
            None
        }
    }
}

impl<'id, T> DoubleEndedIterator for PRangeIter<'id, T> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.0.start != self.0.end {
            unsafe {
                //assume(!self.end.is_null());
                self.0.end  = self.0.end.offset(-1);
                Some(PIndex::inbounds(self.0.end))
            }
        } else {
            None
        }
    }
}


/// `PSlice` is a pointer-based valid range with start pointer and length
/// representation.
#[derive(Debug)]
pub struct PSlice<'id, T, Proof = Unknown> {
    id: Id<'id>,
    start: *const T,
    len: usize,
    /// NonEmpty or Unknown
    proof: PhantomData<Proof>,
}

impl<'id, T, P> Copy for PSlice<'id, T, P> { }
impl<'id, T, P> Clone for PSlice<'id, T, P> {
    fn clone(&self) -> Self { *self }
}

impl<'id, T, P, Q> PartialEq<PSlice<'id, T, Q>> for PSlice<'id, T, P> {
    fn eq(&self, rhs: &PSlice<'id, T, Q>) -> bool {
        self.start == rhs.start && self.len == rhs.len
    }
}
impl<'id, T, P> Eq for PSlice<'id, T, P> { }


impl<'id, T, P> From<PSlice<'id, T, P>> for PRange<'id, T, P> {
    fn from(range: PSlice<'id, T, P>) -> Self {
        unsafe {
            PRange::new(range.start, range.start.offset(range.len as isize))
        }
    }
}
impl<'id, T, P> From<PRange<'id, T, P>> for PSlice<'id, T, P> {
    fn from(range: PRange<'id, T, P>) -> Self {
        unsafe {
            PSlice::new(range.start, ptrdistance(range.end, range.start))
        }
    }
}

impl<'id, T, P> PSlice<'id, T, P> {
    unsafe fn new(start: *const T, len: usize) -> Self {
        debug_assert!(len as isize >= 0);
        PSlice { id: Id::default(), start: start, len: len, proof: PhantomData }
    }

    #[inline]
    pub fn len(self) -> usize { self.len }

    #[inline]
    pub fn is_empty(self) -> bool { self.len == 0 }

    /// Check if the range is empty. `NonEmpty` ranges have extra methods.
    #[inline]
    pub fn nonempty(&self) -> Result<PSlice<'id, T, NonEmpty>, IndexingError>
    {
        unsafe {
            if !self.is_empty() {
                Ok(PSlice::new(self.start, self.len))
            } else {
                Err(index_error())
            }
        }
    }

    /// Split the range in half, with the upper middle index landing in the
    /// latter half. Proof of length `P` transfers to the latter half.
    #[inline]
    pub fn split_in_half(self) -> (PSlice<'id, T>, PSlice<'id, T, P>) {
        unsafe {
            let mid_offset = self.len() / 2;
            let mid = self.start.offset(mid_offset as isize);
            (PSlice::new(self.start, mid_offset), PSlice::new(mid, self.len() - mid_offset))
        }
    }
}

impl<'id, T, P> PSlice<'id, T, P> {
    #[inline]
    pub fn first(self) -> PIndex<'id, T, P> {
        unsafe {
            PIndex::new(self.start)
        }
    }

    /// Return the middle index, rounding up on even
    #[inline]
    pub fn upper_middle(self) -> PIndex<'id, T, P> {
        unsafe {
            let mid = self.len() / 2;
            PIndex::new(self.start.offset(mid as isize))
        }
    }

    #[inline]
    pub fn past_the_end(self) -> PIndex<'id, T, Unknown> {
        unsafe {
            PIndex::new(self.start.offset(self.len as isize))
        }
    }
}

impl<'id, T> PSlice<'id, T, NonEmpty> {
    #[inline]
    pub fn last(self) -> PIndex<'id, T> {
        unsafe {
            PIndex::inbounds(self.start.offset(self.len() as isize - 1))
        }
    }

    #[inline]
    pub fn tail(self) -> PSlice<'id, T> {
        // in bounds since it's nonempty
        unsafe {
            PSlice::new(self.start.offset(1), self.len - 1)
        }
    }

    #[inline]
    pub fn init(self) -> PSlice<'id, T> {
        // in bounds since it's nonempty
        unsafe {
            PSlice::new(self.start, self.len - 1)
        }
    }

    /// Increase the range's start, if the result is still a non-empty range.
    ///
    /// Return `true` if stepped successfully, `false` if the range would be empty.
    #[inline]
    pub fn advance(&mut self) -> bool {
        unsafe {
            // always in bounds because the range is nonempty
            if self.len() > 1 {
                self.start.inc();
                self.len -= 1;
                true
            } else {
                false
            }
        }
    }

    /// Decrease the range's end, if the result is still a non-empty range.
    ///
    /// Return `true` if stepped successfully, `false` if the range would be empty.
    #[inline]
    pub fn advance_back(&mut self) -> bool {
        // always in bounds because the range is nonempty
        if self.len() > 1 {
            self.len -= 1;
            true
        } else {
            false
        }
    }
}

/// `&self[r]` where `r` is a `PRange<'id>`.
impl<'id, T, Array, P> ops::Index<PRange<'id, T, P>> for Container<'id, Array>
    where Array: Contiguous<Item=T>,
{
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: PRange<'id, T, P>) -> &[T] {
        unsafe {
            from_raw_parts(r.start, r.len())
        }
    }
}

/// `&mut self[r]` where `r` is a `Range<'id>`.
impl<'id, T, Array, P> ops::IndexMut<PRange<'id, T, P>> for Container<'id, Array>
    where Array: ContiguousMut<Item=T>,
{
    #[inline(always)]
    fn index_mut(&mut self, r: PRange<'id, T, P>) -> &mut [T] {
        unsafe {
            from_raw_parts_mut(r.start as *mut T, r.len())
        }
    }
}

#[inline(always)]
fn ptr_iselement<T>(arr: &[T], ptr: *const T) {
    unsafe {
        let end = arr.as_ptr().offset(arr.len() as isize);
        debug_assert!(ptr >= arr.as_ptr() && ptr < end);
    }
}

impl<'id, 'a, T, Array> ops::Index<PIndex<'id, T>> for Container<'id, Array>
    where Array: Contiguous<Item=T>,
{
    type Output = T;
    #[inline(always)]
    fn index(&self, r: PIndex<'id, T>) -> &T {
        ptr_iselement(self.array().as_slice(), r.ptr());
        unsafe {
            &*r.ptr()
        }
    }
}

use std::ops::{RangeTo, RangeFrom};

macro_rules! pindex_range {
    ($index_type:ty) => {
impl<'id, T, P, Array> ops::Index<$index_type> for Container<'id, Array>
    where Array: Contiguous<Item=T>,
{
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: $index_type) -> &[T] {
        let start = r.start().map_or(self.begin(), PIndex::ptr);
        let end = r.end().map_or(self.end(), PIndex::ptr);
        let len = ptrdistance(end, start);
        unsafe {
            ::std::slice::from_raw_parts(start, len)
        }
    }
}

impl<'id, T, P, Array> ops::IndexMut<$index_type> for Container<'id, Array>
    where Array: ContiguousMut<Item=T>,
{
    #[inline(always)]
    fn index_mut(&mut self, r: $index_type) -> &mut [T] {
        let start = r.start().map_or(self.begin(), PIndex::ptr);
        let end = r.end().map_or(self.end(), PIndex::ptr);
        let len = ptrdistance(end, start);
        unsafe {
            ::std::slice::from_raw_parts_mut(start as *mut _, len)
        }
    }
}

    }
}

pindex_range!{RangeTo<PIndex<'id, T, P>>}
pindex_range!{RangeFrom<PIndex<'id, T, P>>}

