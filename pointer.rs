
use std::marker::PhantomData;
use std::mem;
use super::Id;
use super::{Checked, Empty, NonEmpty};
use super::{Range, Index};

use std::intrinsics::assume;

/// `PIndex` wraps a valid, non-dangling index or pointer to a location
#[allow(raw_pointer_derive)]
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
    #[inline]
    pub fn ptr(&self) -> *const T {
        self.idx
    }

    #[inline]
    pub fn ptr_mut(&self) -> *mut T {
        self.idx as *mut T
    }
}

/// `PRange` wraps a valid range
#[allow(raw_pointer_derive)]
#[derive(Debug)]
pub struct PRange<'id, T> {
    id: Id<'id>,
    start: *const T,
    end: *const T,
}

impl<'id, T> Copy for PRange<'id, T> { }
impl<'id, T> Clone for PRange<'id, T> {
    fn clone(&self) -> Self { *self }
}


/// return the number of steps between a and b
fn ptrdistance<T>(a: *const T, b: *const T) -> usize {
    (a as usize - b as usize) / mem::size_of::<T>()
}

impl<'id, T> PRange<'id, T> {
    #[inline(always)]
    pub unsafe fn from(start: *const T, end: *const T) -> Self {
        PRange { id: PhantomData, start: start, end: end }
    }

    #[inline]
    pub fn len(&self) -> usize { ptrdistance(self.end, self.start) }
    #[inline]
    pub fn is_empty(&self) -> bool { self.start == self.end }

    /// Check if the range is empty. `NonEmpty` ranges have extra methods.
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
}

impl<'id, T> Checked<PRange<'id, T>, NonEmpty> {
    #[inline]
    pub fn first(&self) -> PIndex<'id, T> {
        PIndex { id: self.id, idx: self.start }
    }

    /// Return the middle index, rounding up on even
    #[inline]
    pub fn upper_middle(&self) -> PIndex<'id, T> {
        unsafe {
            let mid = ptrdistance(self.end, self.start) / 2;
            PIndex { id: self.id, idx: self.start.offset(mid as isize)  }
        }
    }

    #[inline]
    pub fn tail(&self) -> PRange<'id, T> {
        // in bounds since it's nonempty
        unsafe {
            PRange::from(self.start.offset(1), self.end)
        }
    }

    #[inline]
    pub fn init(&self) -> PRange<'id, T> {
        // in bounds since it's nonempty
        unsafe {
            PRange::from(self.start, self.end.offset(-1))
        }
    }

    #[inline]
    pub fn last(&self) -> PIndex<'id, T> {
        unsafe {
            PIndex { id: self.id, idx: self.end.offset(-1) }
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
                assume(!index.is_null());
                self.start = self.start.offset(1);
                Some(PIndex { id: PhantomData, idx: index })
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
                assume(!self.end.is_null());
                self.end  = self.end.offset(-1);
                Some(PIndex { id: PhantomData, idx: self.end })
            }
        } else {
            None
        }
    }
}
