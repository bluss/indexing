//! Sound unchecked indexing in Rust using “generativity”; a type system
//! approach to indices and ranges that are trusted to be in bounds.
//!
//! Includes an index API and an interval (`Range<'id, P>`) API developing its
//! own “algebra” for transformations of in bounds ranges.
//!
//! ***This is an experiment.*** The API is all of inconsistent, incomplete
//! and redundant, but it explores interesting concepts.
#![doc(html_root_url="https://docs.rs/indexing/0.1/")]
#![cfg_attr(not(test), no_std)]

#[cfg(not(test))]
extern crate core as std;

use std::marker::PhantomData;
use std::fmt::{self, Debug};

mod indexing;
pub mod pointer;
pub mod algorithms;
mod index_error;
mod pointer_ext;

pub use index_error::IndexingError;

pub use indexing::{Buffer, BufferMut, Unknown, NonEmpty, Container, indices,
Range, Index};


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
