
use std::cmp::min;
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;
use std::ptr;
use std::ops;
use std::slice::{from_raw_parts, from_raw_parts_mut};
use super::Id;
use super::{NonEmpty, Buffer, BufferMut, Container};
use {Unknown};
use IndexingError;
use index_error::index_error;

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
    unsafe fn from(p: *const T) -> Self {
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
            PIndex::from(self.idx.offset(1))
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
    pub unsafe fn from(start: *const T, end: *const T) -> Self {
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
                Ok(PRange::from(self.start, self.end))
            } else {
                Err(index_error())
            }
        }
    }

    /// Split the range in half, with the upper middle index landing in the
    /// latter half. Proof of length `P` transfers to the latter half.
    #[inline]
    pub fn split_in_half(self) -> (PRange<'id, T>, PRange<'id, T, P>) {
        unsafe {
            let mid_offset = self.len() / 2;
            let mid = self.start.offset(mid_offset as isize);
            (PRange::from(self.start, mid), PRange::from(mid, self.end))
        }
    }
}

impl<'id, T, P> PRange<'id, T, P> {
    #[inline]
    pub fn first(self) -> PIndex<'id, T, P> {
        unsafe {
            PIndex::from(self.start)
        }
    }

    /// Return the middle index, rounding up on even
    #[inline]
    pub fn upper_middle(self) -> PIndex<'id, T, P> {
        unsafe {
            let mid = ptrdistance(self.end, self.start) / 2;
            PIndex::from(self.start.offset(mid as isize))
        }
    }

    #[inline]
    pub fn past_the_end(self) -> PIndex<'id, T, Unknown> {
        unsafe {
            PIndex::from(self.end)
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
            PRange::from(self.start.offset(1), self.end)
        }
    }

    #[inline]
    pub fn init(self) -> PRange<'id, T> {
        // in bounds since it's nonempty
        unsafe {
            PRange::from(self.start, self.end.offset(-1))
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

impl<'id, T, Array> Container<'id, Array> where Array: Buffer<Target=[T]> {
    #[inline]
    pub fn pointer_range(&self) -> PRange<'id, T> {
        unsafe {
            let start = self.as_ptr();
            let end = start.offset(self.len() as isize);
            PRange::from(start, end)
        }
    }

    pub fn pointer_slice(&self) -> PSlice<'id, T> {
        unsafe {
            let start = self.as_ptr();
            PSlice::from(start, self.len())
        }
    }

    fn start(&self) -> *const T {
        self.as_ptr()
    }

    /// Return the distance (in number of elements) from the 
    /// start of the container to the pointer.
    pub fn distance_to<P>(&self, ptr: PIndex<'id, T, P>) -> usize {
        ptrdistance(ptr.ptr(), self.start())
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
            PRange::from(end, index.ptr().offset(1))
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
                ptr = ptr.offset(1);
            }
            (PRange::from(range.start, ptr),
             PRange::from(ptr, range.end))
        }
    }

    #[inline]
    pub fn split_at_pointer<P>(&self, index: PIndex<'id, T, P>)
        -> (PRange<'id, T>, PRange<'id, T, P>) {
        unsafe {
            let pr = self.pointer_range();
            (PRange::from(pr.start, index.idx),
             PRange::from(index.idx, pr.end))
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
            (PRange::from(range.start, ptr),
             PRange::from(ptr, range.end))
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

pub trait PointerRange<'id> : Copy {
    type Item;
    fn ptr(self) -> *const Self::Item;
    fn len(self) -> usize;
}

impl<'id, T, P> PointerRange<'id> for PRange<'id, T, P>
{
    type Item = T;
    fn ptr(self) -> *const Self::Item { self.start }
    fn len(self) -> usize { self.len() }
}

impl<'id, T, P> PointerRange<'id> for PSlice<'id, T, P>
{
    type Item = T;
    fn ptr(self) -> *const Self::Item { self.start }
    fn len(self) -> usize { self.len() }
}

pub trait ContainerRef<'id> {
    type Item;
    type Ref;

    unsafe fn dereference(ptr: *const Self::Item) -> Self::Ref;
}

impl<'id, 'a, Array, T: 'a> ContainerRef<'id> for &'a Container<'id, Array>
    where Array: Buffer<Target=[T]>,
{
    type Item = T;
    type Ref = &'a T;

    unsafe fn dereference(ptr: *const Self::Item) -> Self::Ref {
        &*ptr
    }
}

impl<'id, 'a, Array, T: 'a> ContainerRef<'id> for &'a mut Container<'id, Array>
    where Array: BufferMut<Target=[T]>,
{
    type Item = T;
    type Ref = &'a mut T;

    unsafe fn dereference(ptr: *const Self::Item) -> Self::Ref {
        &mut *(ptr as *mut T)
    }
}

impl<'id, T, Array> Container<'id, Array> where Array: BufferMut<Target=[T]> {

    /// Rotate elements in the range by one step to the right (towards higher indices)
    #[inline]
    pub fn rotate1_(&mut self, r: PRange<'id, T, NonEmpty>) {
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
    pub fn swap_ptr(&mut self, i: PIndex<'id, T>, j: PIndex<'id, T>)
        where Array: BufferMut<Target=[T]>,
    {
        // ptr::swap is ok with equal pointers
        unsafe {
            ptr::swap(i.ptr_mut(), j.ptr_mut())
        }
    }

}


impl<'id, T, P> Iterator for PRange<'id, T, P> {
    type Item = PIndex<'id, T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.start != self.end {
            let index = self.start;
            unsafe {
                //assume(!index.is_null());
                self.start = self.start.offset(1);
                Some(PIndex::inbounds(index))
            }
        } else {
            None
        }
    }
}

impl<'id, T, P> DoubleEndedIterator for PRange<'id, T, P> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start != self.end {
            unsafe {
                //assume(!self.end.is_null());
                self.end  = self.end.offset(-1);
                Some(PIndex::inbounds(self.end))
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
            PRange::from(range.start, range.start.offset(range.len as isize))
        }
    }
}
impl<'id, T, P> From<PRange<'id, T, P>> for PSlice<'id, T, P> {
    fn from(range: PRange<'id, T, P>) -> Self {
        unsafe {
            PSlice::from(range.start, ptrdistance(range.end, range.start))
        }
    }
}

impl<'id, T, P> PSlice<'id, T, P> {
    pub unsafe fn from(start: *const T, len: usize) -> Self {
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
                Ok(PSlice::from(self.start, self.len))
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
            (PSlice::from(self.start, mid_offset), PSlice::from(mid, self.len() - mid_offset))
        }
    }
}

impl<'id, T, P> PSlice<'id, T, P> {
    #[inline]
    pub fn first(self) -> PIndex<'id, T, P> {
        unsafe {
            PIndex::from(self.start)
        }
    }

    /// Return the middle index, rounding up on even
    #[inline]
    pub fn upper_middle(self) -> PIndex<'id, T, P> {
        unsafe {
            let mid = self.len() / 2;
            PIndex::from(self.start.offset(mid as isize))
        }
    }

    #[inline]
    pub fn past_the_end(self) -> PIndex<'id, T, Unknown> {
        unsafe {
            PIndex::from(self.start.offset(self.len as isize))
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
            PSlice::from(self.start.offset(1), self.len - 1)
        }
    }

    #[inline]
    pub fn init(self) -> PSlice<'id, T> {
        // in bounds since it's nonempty
        unsafe {
            PSlice::from(self.start, self.len - 1)
        }
    }
}

    /*
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
    */
// Copyright 2016 bluss
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/// Extension methods for raw pointers
pub trait PointerExt : Copy {
    unsafe fn offset(self, i: isize) -> Self;

    /// Increment by 1
    #[inline(always)]
    unsafe fn inc(&mut self) {
        *self = self.offset(1);
    }

    /// Decrement by 1
    #[inline(always)]
    unsafe fn dec(&mut self) {
        *self = self.offset(-1);
    }

    /// Offset by `s` multiplied by `index`.
    #[inline(always)]
    unsafe fn stride_offset(self, s: isize, index: usize) -> Self {
        self.offset(s * index as isize)
    }
}

impl<T> PointerExt for *const T {
    #[inline(always)]
    unsafe fn offset(self, i: isize) -> Self {
        self.offset(i)
    }
}

impl<T> PointerExt for *mut T {
    #[inline(always)]
    unsafe fn offset(self, i: isize) -> Self {
        self.offset(i)
    }
}

/// `&self[r]` where `r` is a `PRange<'id>`.
impl<'id, T, Array, P> ops::Index<PRange<'id, T, P>> for Container<'id, Array>
    where Array: Buffer<Target=[T]>,
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
    where Array: BufferMut<Target=[T]>,
{
    #[inline(always)]
    fn index_mut(&mut self, r: PRange<'id, T, P>) -> &mut [T] {
        unsafe {
            from_raw_parts_mut(r.start as *mut T, r.len())
        }
    }
}
