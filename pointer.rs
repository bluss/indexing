
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;
use std::ptr;
use super::Id;
use super::{NonEmpty, Buffer, BufferMut, Container};
use {Unknown};
use IndexingError;
use index_error::index_error;

/// `PIndex` wraps a valid, non-dangling index or pointer to a location
#[derive(Debug)]
pub struct PIndex<'id, T> {
    id: Id<'id>,
    idx: *const T,
}

impl<'id, T> Copy for PIndex<'id, T> { }
impl<'id, T> Clone for PIndex<'id, T> {
    fn clone(&self) -> Self { *self }
}

impl<'id, T> PIndex<'id, T> {
    #[inline(always)]
    pub fn ptr(self) -> *const T {
        self.idx
    }

    #[inline(always)]
    pub fn ptr_mut(self) -> *mut T {
        self.idx as *mut T
    }
}

impl<'id, T> PartialEq for PIndex<'id, T> {
    fn eq(&self, rhs: &Self) -> bool {
        self.idx == rhs.idx
    }
}
impl<'id, T> Eq for PIndex<'id, T> { }

/// `PRange` wraps a valid range
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

impl<'id, T> PRange<'id, T, NonEmpty> {
    #[inline]
    pub fn first(self) -> PIndex<'id, T> {
        PIndex { id: self.id, idx: self.start }
    }

    /// Return the middle index, rounding up on even
    #[inline]
    pub fn upper_middle(self) -> PIndex<'id, T> {
        unsafe {
            let mid = ptrdistance(self.end, self.start) / 2;
            PIndex { id: self.id, idx: self.start.offset(mid as isize)  }
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

    #[inline]
    pub fn last(self) -> PIndex<'id, T> {
        unsafe {
            PIndex { id: self.id, idx: self.end.offset(-1) }
        }
    }
}

impl<'id, T, Array> Container<'id, Array> where Array: Buffer<Target=[T]> {
    #[inline]
    pub fn pointer_range(&self) -> PRange<'id, T> {
        unsafe {
            let start = self.arr.as_ptr();
            let end = start.offset(self.arr.len() as isize);
            PRange::from(start, end)
        }
    }

    /// Return the distance (in number of elements) from the 
    /// start of the container to the start of the range.
    pub fn distance_to<P>(&self, r: PRange<'id, T, P>) -> usize {
        ptrdistance(r.start, self.arr.as_ptr())
    }

    /// Examine the elements before `index` in order from higher indices towards lower.
    /// While the closure returns `true`, continue scan and include the scanned
    /// element in the range.
    ///
    /// Result always includes `index` in the range
    #[inline]
    pub fn scan_tail_<F>(&self, index: PIndex<'id, T>, mut f: F) -> PRange<'id, T, NonEmpty>
        where F: FnMut(&T) -> bool
    {
        unsafe {
            let container_start = self.pointer_range().start;
            let mut end = index.ptr();
            loop {
                if !f(&*end.offset(-1)) {
                    break;
                }
                if container_start == end {
                    break;
                }
                end.dec();
            }
            PRange::from(end, index.ptr().offset(1))
        }
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

}


impl<'id, T> Iterator for PRange<'id, T> {
    type Item = PIndex<'id, T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.start != self.end {
            let index = self.start;
            unsafe {
                //assume(!index.is_null());
                self.start = self.start.offset(1);
                Some(PIndex { id: self.id, idx: index })
            }
        } else {
            None
        }
    }
}

impl<'id, T> DoubleEndedIterator for PRange<'id, T> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start != self.end {
            unsafe {
                //assume(!self.end.is_null());
                self.end  = self.end.offset(-1);
                Some(PIndex { id: self.id, idx: self.end })
            }
        } else {
            None
        }
    }
}

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
