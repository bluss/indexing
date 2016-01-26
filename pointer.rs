
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;
use std::ptr;
use super::Id;
use super::{NonEmpty, BufferMut, Container};

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
        PRange { id: Id::default(), start: start, end: end }
    }

    #[inline]
    pub fn len(&self) -> usize { ptrdistance(self.end, self.start) }
    #[inline]
    pub fn is_empty(&self) -> bool { self.start == self.end }

    /// Check if the range is empty. `NonEmpty` ranges have extra methods.
    #[inline]
    pub fn nonempty(&self) -> Result<Checked<Self, NonEmpty>, Self> {
        unsafe {
            if self.len() > 0 {
                Ok(Checked::new(*self))
            } else {
                Err(*self)
            }
        }
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

/// Deref to the inner range
// NOTE: immutable deref is ok, mutable would be unsound
impl<'id, X, L> Deref for Checked<X, L> {
    type Target = X;
    fn deref(&self) -> &X {
        &self.item
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

impl<'id, T, Array> Container<'id, Array> where Array: BufferMut<Target=[T]> {
    #[doc(hidden)]
    #[inline]
    pub fn pointer_range(&self) -> PRange<'id, T> {
        unsafe {
            let start = self.arr.as_ptr();
            let end = start.offset(self.arr.len() as isize);
            PRange::from(start, end)
        }
    }

    #[doc(hidden)]
    /// Rotate elements in the range by one step to the right (towards higher indices)
    #[inline]
    pub fn rotate1_(&mut self, r: Checked<PRange<'id, T>, NonEmpty>) {
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

    #[doc(hidden)]
    /// Examine the elements before `index` in order from higher indices towards lower.
    /// While the closure returns `true`, continue scan and include the scanned
    /// element in the range.
    ///
    /// Result always includes `index` in the range
    #[inline]
    pub fn scan_tail_<F>(&self, index: PIndex<'id, T>, mut f: F) -> Checked<PRange<'id, T>, NonEmpty>
        where F: FnMut(&T) -> bool
    {
        unsafe {
            let mut start = index.ptr();
            for elt in self[..index].iter().rev() {
                if !f(elt) {
                    break;
                }
                start = elt as *const _;
            }
            Checked::new(PRange::from(start, index.ptr().offset(1)))
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
