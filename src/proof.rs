
use std::mem;

use crate::{Index, Range};
use crate::pointer::{PIndex, PRange, PSlice};

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
    type WithoutProof : Provable<Proof=Unknown>;
    /// Return a copy of self with the proof parameter set to `Unknown`.
    fn no_proof(self) -> Self::WithoutProof;
}

impl<'id, P> Provable for Index<'id, P> {
    type Proof = P;
    type WithoutProof = Index<'id, Unknown>;

    #[inline]
    fn no_proof(self) -> Self::WithoutProof {
        unsafe {
            Index::assume_any_index(self)
        }
    }
}

impl<'id, P> Provable for Range<'id, P> {
    type Proof = P;
    type WithoutProof = Range<'id, Unknown>;

    #[inline]
    fn no_proof(self) -> Self::WithoutProof {
        unsafe {
            Range::assume_any_range(self)
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

#[cfg(test)]
pub(crate) trait ProofType {
    fn nonempty() -> bool;
    fn unknown() -> bool { !Self::nonempty() }
}

#[cfg(test)]
impl ProofType for Unknown {
    fn nonempty() -> bool { false }
}

#[cfg(test)]
impl ProofType for NonEmpty {
    fn nonempty() -> bool { false }
}


#[cfg(test)]
impl<'id, P> Index<'id, P> {
    pub(crate) fn nonempty_proof(&self) -> bool where P: ProofType
    { P::nonempty() }
}

#[cfg(test)]
impl<'id, P> Range<'id, P> {
    pub(crate) fn nonempty_proof(&self) -> bool where P: ProofType
    { P::nonempty() }
}
