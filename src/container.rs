
use std::cmp;
use std::ops;
use std::ptr;
use std::mem;

use std::fmt::{self, Debug};

use std::marker::PhantomData;

use index_error::IndexingError;
use index_error::index_error;
use proof::*;
use std;

use container_traits::*;
use indexing::{IntoCheckedRange};
use {Id, Index, Range};
use ContainerPrivate;

/// A branded container, that allows access only to indices and ranges with
/// the exact same brand in the `'id` parameter.
///
/// The elements in the underlying data structure are accessible partly
/// through special purpose methods, and through indexing/slicing.
///
/// The `Container` can be indexed like `self[i]` where `i` is a trusted
/// dereferenceable index
/// or range, and equivalently using `&self[i..]` or `&self[..i]` where
/// `i` is a trusted index. Indexing like this uses no runtime bounds checking
/// at all, and it statically guaranteed to be in bounds.
///
/// The container can also be sliced for its complete range: `&self[..]`.
pub struct Container<'id, Array, Mode = ()> {
    id: Id<'id>,
    arr: Array,
    mode: PhantomData<Mode>,
}

/// Only indexing mode for a container (disallows access through pointer).
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

impl<'id, Array, Mode> ContainerPrivate for Container<'id, Array, Mode> {
    type Array = Array;
    #[inline(always)]
    fn array(&self) -> &Self::Array {
        &self.arr
    }
    #[inline(always)]
    fn array_mut(&mut self) -> &mut Self::Array {
        &mut self.arr
    }
}

impl<'id, Array, T, Mode> Container<'id, Array, Mode>
    where Array: Trustworthy<Item=T>,
{
    #[inline]
    pub fn len(&self) -> usize {
        self.arr.base_len()
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

    /// Vet the absolute `index`.
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

    /// Vet the range `r`.
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
        self.range_of(..index)
    }

    /// Return the range after (not including) the index itself
    #[inline]
    pub fn after(&self, index: Index<'id>) -> Range<'id> {
        self.range_of(index.after()..)
    }

    #[inline]
    pub fn range_of<P, R>(&self, r: R) -> Range<'id>
        where R: OnePointRange<Index<'id, P>>,
    {
        debug_assert!(!(r.start().is_some() && r.end().is_some()));
        unsafe {
            let start = r.start().map(|i| i.index).unwrap_or(0);
            let end = r.end().map(|i| i.index).unwrap_or(self.len());
            debug_assert!(start <= end && end <= self.len());
            Range::from(start, end)
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
              Array: Contiguous<Item=T>,
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
        where F: FnMut(&'b T) -> bool, T: 'b,
              Array: Contiguous<Item=T>,
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
              Array: Contiguous<Item=T>,
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
        where Array: GetUncheckedMut
    {
        unsafe {
            let self_mut = self as *mut Self;
            let pi: *mut _ = &mut (*self_mut)[i];
            let pj: *mut _ = &mut (*self_mut)[j];
            ptr::swap(pi, pj);
        }
    }

    /// Rotate elements in the range `r` by one step to the right (towards higher indices)
    #[inline]
    pub fn rotate1_up<R>(&mut self, r: R)
        where Array: Contiguous + GetUncheckedMut,
              R: IntoCheckedRange<'id>
    {
        if let Ok(r) = r.into() {
            if r.first() != r.last() {
                unsafe {
                    let last_ptr = &self[r.last()] as *const Array::Item;
                    let first_ptr = &mut self[r.first()] as *mut Array::Item;
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
        where Array: Contiguous + GetUncheckedMut,
              R: IntoCheckedRange<'id>
    {
        if let Ok(r) = r.into() {
            if r.first() != r.last() {
                unsafe {
                    let last_ptr = &mut self[r.last()] as *mut Array::Item;
                    let first_ptr = &mut self[r.first()] as *mut Array::Item;
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
        where Array: ContiguousMut
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
              Array: GetUncheckedMut,
    {
        let len = cmp::min(r.len(), s.len());
        for i in 0..len {
            unsafe {
                f(
                    self.arr.xget_unchecked_mut(r.start + i),
                    self.arr.xget_unchecked_mut(s.start + i)
                )
            }
        }
    }
}

/// Methods specific to only index mode
impl<'id, Array, T> Container<'id, Array, OnlyIndex>
    where Array: Pushable<Item=T>,
{
    /// Add one element to the underlying storage, and return its index.
    ///
    /// All outstanding indices remain valid, only the length of the
    /// container is now larger.
    pub fn push(&mut self, element: T) -> Index<'id> {
        let i = self.arr.push(element);
        debug_assert!(i < self.arr.base_len());
        unsafe {
            Index::new(i)
        }
    }

    /// Insert one element in the underlying storage at `index`.
    ///
    /// All outstanding indices remain valid (in bounds), but elements have
    /// shifted.
    pub fn insert<Q>(&mut self, index: Index<'id, Q>, element: T) {
        debug_assert!(index.index <= self.arr.base_len());
        unsafe {
            self.arr.insert_unchecked(index.index, element);
        }
    }
}

/// `&self[i]` where `i` is an `Index<'id>`.
impl<'id, Array, M> ops::Index<Index<'id>> for Container<'id, Array, M>
    where Array: GetUnchecked
{
    type Output = Array::Item;
    #[inline(always)]
    fn index(&self, index: Index<'id>) -> &Self::Output {
        unsafe {
            self.arr.xget_unchecked(index.index)
        }
    }
}

/// `&mut self[i]` where `i` is an `Index<'id>`.
impl<'id, Array, M> ops::IndexMut<Index<'id>> for Container<'id, Array, M>
    where Array: GetUncheckedMut
{
    #[inline(always)]
    fn index_mut(&mut self, index: Index<'id>) -> &mut Self::Output {
        unsafe {
            self.arr.xget_unchecked_mut(index.index)
        }
    }
}

/// `&self[r]` where `r` is a `Range<'id>`.
impl<'id, T, Array, P, M> ops::Index<Range<'id, P>> for Container<'id, Array, M>
    where Array: Contiguous<Item=T>,
{
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: Range<'id, P>) -> &Self::Output {
        unsafe {
            std::slice::from_raw_parts(
                self.arr.begin().offset(r.start as isize),
                r.len())
        }
    }
}

/// `&mut self[r]` where `r` is a `Range<'id>`.
impl<'id, Array, P, M> ops::IndexMut<Range<'id, P>> for Container<'id, Array, M>
    where Array: ContiguousMut,
{
    #[inline(always)]
    fn index_mut(&mut self, r: Range<'id, P>) -> &mut Self::Output {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.arr.begin_mut().offset(r.start as isize),
                r.len())
        }
    }
}

/// `&self[i..]` where `i` is an `Index<'id, P>` which may be an edge index.
impl<'id, T, P, Array, M> ops::Index<ops::RangeFrom<Index<'id, P>>> for Container<'id, Array, M>
    where Array: Contiguous<Item=T>,
{
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: ops::RangeFrom<Index<'id, P>>) -> &[T] {
        let i = r.start.index;
        unsafe {
            std::slice::from_raw_parts(
                self.arr.begin().offset(i as isize),
                self.len() - i)
        }
    }
}

/// `&mut self[i..]` where `i` is an `Index<'id, P>` which may be an edge index.
impl<'id, T, P, Array, M> ops::IndexMut<ops::RangeFrom<Index<'id, P>>> for Container<'id, Array, M>
    where Array: ContiguousMut<Item=T>,
{
    #[inline(always)]
    fn index_mut(&mut self, r: ops::RangeFrom<Index<'id, P>>) -> &mut [T] {
        let i = r.start.index;
        unsafe {
            std::slice::from_raw_parts_mut(
                self.arr.begin_mut().offset(i as isize),
                self.len() - i)
        }
    }
}

/// `&self[..i]` where `i` is an `Index<'id, P>`, which may be an edge index.
impl<'id, T, P, Array, M> ops::Index<ops::RangeTo<Index<'id, P>>> for Container<'id, Array, M>
    where Array: Contiguous<Item=T>,
{
    type Output = [T];
    #[inline(always)]
    fn index(&self, r: ops::RangeTo<Index<'id, P>>) -> &[T] {
        let i = r.end.index;
        unsafe {
            std::slice::from_raw_parts(self.arr.begin(), i)
        }
    }
}

/// `&mut self[..i]` where `i` is an `Index<'id, P>`, which may be an edge index.
impl<'id, T, P, Array, M> ops::IndexMut<ops::RangeTo<Index<'id, P>>> for Container<'id, Array, M>
    where Array: ContiguousMut<Item=T>,
{
    #[inline(always)]
    fn index_mut(&mut self, r: ops::RangeTo<Index<'id, P>>) -> &mut [T] {
        let i = r.end.index;
        unsafe {
            std::slice::from_raw_parts_mut(self.arr.begin_mut(), i)
        }
    }
}

/// `&self[..]`
impl<'id, T, Array, M> ops::Index<ops::RangeFull> for Container<'id, Array, M>
    where Array: Contiguous<Item=T>,
{
    type Output = [T];
    #[inline(always)]
    fn index(&self, _: ops::RangeFull) -> &[T] {
        self.arr.as_slice()
    }
}

/// `&mut self[..]`
impl<'id, T, Array> ops::IndexMut<ops::RangeFull> for Container<'id, Array>
    where Array: ContiguousMut<Item=T>,
{
    #[inline(always)]
    fn index_mut(&mut self, _: ops::RangeFull) -> &mut [T] {
        self.arr.as_mut_slice()
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

/// Create an indexing scope for a container.
///
/// The indexing scope is a closure that is passed a unique lifetime for
/// the parameter `'id`; this lifetime brands the container and its indices
/// and ranges, so that they are trusted to be in bounds.
///
/// Indices and ranges branded with `'id` can not leave the closure. The
/// container can only be accessed and mutated through the `Container` wrapper
/// passed as the first argument to the indexing scope.
pub fn scope<Array, F, Out>(arr: Array, f: F) -> Out
    where F: for<'id> FnOnce(Container<'id, Array>) -> Out,
          Array: Trustworthy,
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
    f(Container { id: Id::default(), arr: arr, mode: PhantomData })
}

#[test]
fn test_intervals() {
    let mut data = [0; 8];
    scope(&mut data[..], |mut data| {
        let r = data.range();
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
    scope(&mut data[..], |mut arr| {
        let r = arr.range();
        for elt in &mut arr[r] {
            *elt += 1;
        }
        // println!("{:?}", &mut arr[r]);
    });
}


#[test]
fn test_scan() {
    let mut data = [0, 0, 0, 1, 2];
    scope(&mut data[..], |data| {
        let r = data.range().nonempty().unwrap();
        let range = data.scan_from(r.first(), |elt| *elt == 0);
        assert_eq!(&data[range], &[0, 0, 0]);
        let range = data.scan_from(range.last(), |elt| *elt != 0);
        assert_eq!(&data[range], &[0, 1, 2]);
    });
}

#[test]
fn test_nonempty() {
    let mut data = [0, 1, 2, 3, 4, 5];
    scope(&mut data[..], |data| {
        let mut r = data.range().nonempty().unwrap();
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
    scope(&mut data[..], |data| {
        let r = data.range();
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
