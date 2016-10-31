
use std::mem;

use indexing::{Range};
use pointer::{PIndex, PRange, PSlice};

/// Length marker for range known to not be empty.
#[derive(Copy, Clone, Debug)]
pub enum NonEmpty {}
/// Length marker for unknown length.
#[derive(Copy, Clone, Debug)]
pub enum Unknown {}

/// Represents the combination of two proofs `P` and `Q` by a new type `Sum`.
pub trait ProofAdd {
    type Sum;
}

impl<Q> ProofAdd for (NonEmpty, Q) { type Sum = NonEmpty; }
impl<Q> ProofAdd for (Unknown, Q) { type Sum = Q; }


pub trait Provable {
    type Proof;
    type WithoutProof;
    fn no_proof(self) -> Self::WithoutProof;
}

impl<'id, P> Provable for Range<'id, P> {
    type Proof = P;
    type WithoutProof = Range<'id, Unknown>;

    #[inline]
    fn no_proof(self) -> Self::WithoutProof {
        unsafe {
            mem::transmute(self)
        }
    }
}

impl<'id, T, P> Provable for PIndex<'id, T, P> {
    type Proof = P;
    type WithoutProof = PIndex<'id, T, Unknown>;

    #[inline]
    fn no_proof(self) -> Self::WithoutProof {
        unsafe {
            mem::transmute(self)
        }
    }
}

impl<'id, T, P> Provable for PRange<'id, T, P> {
    type Proof = P;
    type WithoutProof = PRange<'id, T, Unknown>;

    #[inline]
    fn no_proof(self) -> Self::WithoutProof {
        unsafe {
            mem::transmute(self)
        }
    }
}

impl<'id, T, P> Provable for PSlice<'id, T, P> {
    type Proof = P;
    type WithoutProof = PSlice<'id, T, Unknown>;

    #[inline]
    fn no_proof(self) -> Self::WithoutProof {
        unsafe {
            mem::transmute(self)
        }
    }
}

