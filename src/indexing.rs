

// Modules

use std::cmp;
use std::mem;

use std::fmt::{self, Debug};

use index_error::IndexingError;
use index_error::index_error;
use std;
use prelude::*;
use base::ProofAdd;

use {Id, Index, Range};



impl<'id, P> Index<'id, P> {
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

impl<'id, P> Range<'id, P> {
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

