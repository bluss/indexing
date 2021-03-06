//! Sound unchecked indexing in Rust using “generativity”; a type system
//! approach to indices, pointers and ranges that are trusted to be in bounds.
//!
//! We are developing our own “algebra” for transformations of in bounds ranges.
//!
//! Apart from trusted single indices and pointers, there are intervals like
//! `Range<'id, P>` (indices) and `PRange<'id, T, P>` (pointers).
//!
//! These particles use marker types to for example enable certain methods only
//! for ranges that are known to be nonempty.
//!
//! ***This is an experiment.*** The API is all of inconsistent, incomplete
//! and redundant, but it explores interesting concepts.
//!
//! # Basic Parts
//!
//! - A scope is created using the [`scope`](container/fn.scope.html) function;
//!   inside this scope, there is a [`Container`][c] object that has two roles:
//!   (1) it gives out or vets trusted indices, pointers and ranges (2) it
//!   provides access to the underlying data through these indices and ranges.
//!
//! - The container and its indices and ranges are “branded” with a lifetime
//!   parameter `'id` which is an identity marker. Branded items
//!   can't leave their scope, and they tie the items uniquely to a particular
//!   container. This makes it possible to trust them.
//!
//! - `Index<'id>` is a trusted index
//! - `Range<'id, P>` is a trusted range.
//! - For a range, if the proof parameter `P` is `NonEmpty`, then the range is
//! known to have at least one element. An observation: A non-empty range always
//! has a valid front index, so it is interchangeable with the index
//! representation.
//! - indices and pointers also use the same proof parameter. A `NonEmpty`
//!   index points to a valid element, while an `Unknown` index is an edge
//!   index (it can be used to slice the container, but not to dereference to
//!   an element).
//! - All ranges have a `.first()` method to get the first index or pointer
//!   in the range, but it's only when the range is nonempty that the returned
//!   particle is also `NonEmpty` and thus dereferenceable.
//!
//! [c]: container/struct.Container.html
//!
//! # Raw Pointers
//!
//! Branded raw pointers work very similarly to indices. However, the code
//! needs revision and it's not of good quality, so it's not enabled by default.
//!
//! - `PIndex<'id, T>` and `PRange<'id, T, P>` are equivalent to `Index` and
//! `Range`, but they use trusted raw pointers instead.
//! There are even two kinds of ranges: `PRange` uses a begin and end pointer
//! representation, and `PSlice` a begin pointer and length representation.
//!
//! # Borrowing Rules
//!
//! - The indices, pointers and ranges are freely copyable and do not track
//! mutability or exclusive access themselves. All access to the underlying data
//! goes through the Container, for example by indexing the container with
//! a trusted particle.
//!
//!
//!
//! # Example
//!
//! Find the lower bound index for element `elt` with a binary search using pointer ranges:
//!
//! ```rust
//! use indexing::scope;
//!
//! fn lower_bound<T: PartialOrd>(v: &[T], elt: &T) -> usize {
//!     scope(v, move |v| {
//!         let mut range = v.range();
//!         while let Ok(range_) = range.nonempty() {
//!             // The upper half of the split range still carries the proof
//!             // that it is non-empty, so we can access the element at `b.first()`
//!             let (a, b) = range_.split_in_half();
//!
//!             // THIS is the only access to the data in the underlying slice;
//!             // accessing the first element after the range's split point.
//!             // Access uses indexing syntax `v[index]` but note that the access
//!             // uses no runtime bounds checking and is guaranteed to be in bounds.
//!             if v[b.first()] < *elt {
//!                 // A nonempty range has a tail (everything but the first element)
//!                 range = b.tail();
//!             } else {
//!                 range = a;
//!             }
//!         }
//!         // return the start index of the range
//!         range.first().integer()
//!     })
//! }
//!
//!
//! // Find the lower bound for "2", which is the point exactly between the ones and the twos.
//! let data = [0, 1, 1, 2, 2, 2, 3, 4];
//! assert_eq!(lower_bound(&data, &2), 3);
//! ```
//!
#![doc(html_root_url="https://docs.rs/indexing/0.2/")]
#![cfg_attr(not(feature = "use_std"), no_std)]

#[cfg(not(feature = "use_std"))]
extern crate core as std;

use std::marker::PhantomData;
use std::fmt::{self, Debug};

#[macro_use] mod macro_utils;
pub mod indexing;
pub mod proof;
pub mod algorithms;
pub mod container_traits;
pub mod container;
#[cfg(feature="experimental_pointer_ranges")]
pub mod pointer;
mod index_error;
mod pointer_ext;

pub use crate::index_error::IndexingError;

pub use crate::container::{Container, scope};

pub use crate::proof::{NonEmpty, Unknown};


// Common types //

/// `Id<'id>` is invariant w.r.t `'id`
///
/// This means that the inference engine is not allowed to shrink or
/// grow 'id to solve the borrow system.
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq)]
struct Id<'id> { id: PhantomData<*mut &'id ()>, }

unsafe impl<'id> Sync for Id<'id> { }
unsafe impl<'id> Send for Id<'id> { }

impl<'id> Default for Id<'id> {
    #[inline]
    fn default() -> Self {
        Id { id: PhantomData }
    }
}

impl<'id> Debug for Id<'id> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Id<'id>")
    }
}

/// A branded index.
///
/// `Index<'id>` only indexes the container instantiated with the exact same
/// particular lifetime for the parameter `'id` at its inception from
/// the `scope()` function.
///
/// The type parameter `Proof` determines if the index is dereferenceable.
///
/// A `NonEmpty` index points to a valid element. An `Unknown` index is unknown,
/// or it points to an edge index (just past the end).
pub struct Index<'id, Proof = NonEmpty> {
    id: Id<'id>,
    index: usize,
    /// NonEmpty or Unknown
    proof: PhantomData<Proof>,
}
copy_and_clone!(['id, P] Index<'id, P>);

impl<'id, P> Index<'id, P> {
    #[inline(always)]
    unsafe fn new(index: usize) -> Index<'id, P> {
        debug_assert!(index as isize >= 0);
        Index { id: Id::default(), index: index, proof: PhantomData }
    }

    #[inline]
    // Assume any proof for index
    unsafe fn assume_any_index<Q>(other: Index<'id, Q>) -> Self {
        Self::new(other.index)
    }
}


/// A branded range.
///
/// `Range<'id>` only indexes the container instantiated with the exact same
/// particular lifetime for the parameter `'id` at its inception from
/// the `scope()` function.
///
/// The `Range` may carry a proof of nonemptiness (type parameter `Proof`),
/// which enables further methods.
///
/// The range is delimited by a start index and an end index. Some methods
/// will use offsets relative the the start of a range, others will use
/// “absolute indices” which are offsets relative to the base `Container`
/// itself.
pub struct Range<'id, Proof=Unknown> {
    id: Id<'id>,
    start: usize,
    end: usize,
    /// NonEmpty or Unknown
    proof: PhantomData<Proof>,
}
copy_and_clone!(['id, P] Range<'id, P>);

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

    #[inline]
    unsafe fn assume_nonempty_range<Q>(other: Range<'id, Q>) -> Range<'id, NonEmpty> {
        Range::from_ne(other.start, other.end)
    }
}

impl<'id, P> Range<'id, P> {
    #[inline(always)]
    unsafe fn from_any(start: usize, end: usize) -> Range<'id, P> {
        debug_assert!(start <= end);
        Range { id: Id::default(), start: start, end: end, proof: PhantomData }
    }

    #[inline]
    unsafe fn assume_any_range<Q>(other: Range<'id, Q>) -> Self {
        Self::from_any(other.start, other.end)
    }
}

// Access the internals of Container in the whole crate (but not outside)
trait ContainerPrivate {
    type Array;
    fn array(&self) -> &Self::Array;
    fn array_mut(&mut self) -> &mut Self::Array;
}
