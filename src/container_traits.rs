
/// The most basic container trait: it can have indices and ranges that are
/// trusted to be in bounds.
pub unsafe trait Trustworthy {
    type Item;
    fn base_len(&self) -> usize;
}

/// The container has a contiguous addressable range.
pub unsafe trait Contiguous : Trustworthy {
    fn begin(&self) -> *const Self::Item;
    fn end(&self) -> *const Self::Item;
    fn as_slice(&self) -> &[Self::Item];
}

pub unsafe trait GetUnchecked : Trustworthy {
    unsafe fn xget_unchecked(&self, i: usize) -> &Self::Item;
}

pub unsafe trait GetUncheckedMut : GetUnchecked {
    unsafe fn xget_unchecked_mut(&mut self, i: usize) -> &mut Self::Item;
}

pub unsafe trait ContiguousMut : Contiguous {
    fn begin_mut(&mut self) -> *mut Self::Item;
    fn end_mut(&mut self) -> *mut Self::Item;
    fn as_mut_slice(&mut self) -> &mut [Self::Item];
}

unsafe impl<'a, C: ?Sized> Trustworthy for &'a C
    where C: Trustworthy
{
    type Item = C::Item;
    fn base_len(&self) -> usize {
        (**self).base_len()
    }
}

unsafe impl<'a, C: ?Sized> Trustworthy for &'a mut C
    where C: Trustworthy
{
    type Item = C::Item;
    fn base_len(&self) -> usize {
        (**self).base_len()
    }
}

unsafe impl<'a, C: ?Sized> ContiguousMut for &'a mut C
    where C: ContiguousMut
{
    fn begin_mut(&mut self) -> *mut Self::Item { (**self).begin_mut() }
    fn end_mut(&mut self) -> *mut Self::Item { (**self).end_mut() }
    fn as_mut_slice(&mut self) -> &mut [Self::Item] {
        (**self).as_mut_slice()
    }
}

unsafe impl<'a, C: ?Sized> GetUnchecked for &'a C
    where C: GetUnchecked
{
    unsafe fn xget_unchecked(&self, i: usize) -> &Self::Item {
        (**self).xget_unchecked(i)
    }
}

unsafe impl<'a, C: ?Sized> GetUnchecked for &'a mut C
    where C: GetUnchecked
{
    unsafe fn xget_unchecked(&self, i: usize) -> &Self::Item {
        (**self).xget_unchecked(i)
    }
}

unsafe impl<'a, C: ?Sized> GetUncheckedMut for &'a mut C
    where C: GetUncheckedMut
{
    unsafe fn xget_unchecked_mut(&mut self, i: usize) -> &mut Self::Item {
        (**self).xget_unchecked_mut(i)
    }
}

unsafe impl<'a, C: ?Sized> Contiguous for &'a C
    where C: Contiguous,
{
    fn begin(&self) -> *const Self::Item {
        (**self).begin()
    }
    fn end(&self) -> *const Self::Item {
        (**self).end()
    }
    fn as_slice(&self) -> &[Self::Item] {
        (**self).as_slice()
    }
}

unsafe impl<'a, C: ?Sized> Contiguous for &'a mut C
    where C: Contiguous,
{
    fn begin(&self) -> *const Self::Item {
        (**self).begin()
    }
    fn end(&self) -> *const Self::Item {
        (**self).end()
    }
    fn as_slice(&self) -> &[Self::Item] {
        (**self).as_slice()
    }
}

unsafe impl<T> Trustworthy for [T] {
    type Item = T;
    fn base_len(&self) -> usize { self.len() }
}

unsafe impl<T> ContiguousMut for [T] {
    fn begin_mut(&mut self) -> *mut Self::Item {
        self.as_mut_ptr()
    }
    fn end_mut(&mut self) -> *mut Self::Item {
        unsafe {
            self.begin_mut().add(self.len())
        }
    }
    fn as_mut_slice(&mut self) -> &mut [Self::Item] {
        self
    }
}

unsafe impl<T> GetUnchecked for [T] {
    unsafe fn xget_unchecked(&self, i: usize) -> &Self::Item {
        self.get_unchecked(i)
    }
}

unsafe impl<T> GetUncheckedMut for [T] {
    unsafe fn xget_unchecked_mut(&mut self, i: usize) -> &mut Self::Item {
        self.get_unchecked_mut(i)
    }
}

unsafe impl<T> Contiguous for [T] {
    fn begin(&self) -> *const Self::Item {
        self.as_ptr()
    }
    fn end(&self) -> *const Self::Item {
        unsafe {
            self.begin().add(self.len())
        }
    }
    fn as_slice(&self) -> &[Self::Item] {
        self
    }
}

#[cfg(feature = "use_std")]
mod vec_impls {
    use super::*;
    unsafe impl<T> Trustworthy for Vec<T> {
        type Item = T;
        fn base_len(&self) -> usize { self.len() }
    }

    unsafe impl<T> ContiguousMut for Vec<T> {
        fn begin_mut(&mut self) -> *mut Self::Item { (**self).begin_mut() }
        fn end_mut(&mut self) -> *mut Self::Item { (**self).end_mut() }
        fn as_mut_slice(&mut self) -> &mut [Self::Item] {
            self
        }
    }

    unsafe impl<T> GetUnchecked for Vec<T> {
        unsafe fn xget_unchecked(&self, i: usize) -> &Self::Item {
            self.get_unchecked(i)
        }
    }

    unsafe impl<T> GetUncheckedMut for Vec<T> {
        unsafe fn xget_unchecked_mut(&mut self, i: usize) -> &mut Self::Item {
            self.get_unchecked_mut(i)
        }
    }

    unsafe impl<T> Contiguous for Vec<T> {
        fn begin(&self) -> *const Self::Item {
            (**self).begin()
        }

        fn end(&self) -> *const Self::Item {
            (**self).end()
        }
        fn as_slice(&self) -> &[Self::Item] {
            self
        }
    }
    unsafe impl<T> Pushable for Vec<T> {
        fn push(&mut self, item: T) -> usize {
            let i = self.len();
            self.push(item);
            i
        }
        unsafe fn insert_unchecked(&mut self, index: usize, item: Self::Item) {
            self.insert(index, item)
        }
    }
}

pub unsafe trait Pushable : Trustworthy {
    fn push(&mut self, item: Self::Item) -> usize;
    unsafe fn insert_unchecked(&mut self, index: usize, item: Self::Item);
}


unsafe impl<'a, C: ?Sized> Pushable for &'a mut C
    where C: Pushable,
{
    fn push(&mut self, item: Self::Item) -> usize {
        (**self).push(item)
    }
    unsafe fn insert_unchecked(&mut self, index: usize, item: Self::Item) {
        (**self).insert_unchecked(index, item)
    }
}


/// A range being `..`, `a..`, `..b`, or `a..b`.
pub trait IndexRange<I> : Sized {
    fn start(&self) -> Option<I> { None }
    fn end(&self) -> Option<I> { None }
}

use std::ops::{RangeFull, RangeTo, RangeFrom, Range};

impl<I: Copy> IndexRange<I> for RangeFull { }
impl<I: Copy> IndexRange<I> for RangeFrom<I> {
    fn start(&self) -> Option<I> { Some(self.start) }
}
impl<I: Copy> IndexRange<I> for RangeTo<I> {
    fn end(&self) -> Option<I> { Some(self.end) }
}
impl<I: Copy> IndexRange<I> for Range<I> {
    fn start(&self) -> Option<I> { Some(self.start) }
    fn end(&self) -> Option<I> { Some(self.end) }
}

/// A range with at most one point, being `a..`, `..`, or `..b`.
// unsafe because: trusted to have at one endpoint
pub unsafe trait OnePointRange<I> : IndexRange<I> {
}

unsafe impl<I: Copy> OnePointRange<I> for RangeFrom<I> { }
unsafe impl<I: Copy> OnePointRange<I> for RangeTo<I> { }
unsafe impl<I: Copy> OnePointRange<I> for RangeFull { }
