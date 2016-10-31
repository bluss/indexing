use std::fmt;

/// Error produced when an indexing operation is out of bounds or otherwise
/// inapplicable.
///
/// Carries no context information (to be as light as possible).
#[derive(Copy, Debug, PartialEq)]
pub struct IndexingError(());

#[inline]
pub fn index_error() -> IndexingError {
    IndexingError(())
}

impl Clone for IndexingError {
    fn clone(&self) -> Self { *self }
}

impl IndexingError {
    pub fn description(&self) -> &str {
        "index error"
    }
}

impl fmt::Display for IndexingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.description())
    }
}
