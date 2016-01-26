use std::fmt;
use std::error::Error;

#[derive(Copy, Debug, PartialEq)]
pub struct IndexingError(());

#[inline]
pub fn index_error() -> IndexingError {
    IndexingError(())
}

impl Clone for IndexingError {
    fn clone(&self) -> Self { *self }
}

impl Error for IndexingError {
    fn description(&self) -> &str {
        "index error"
    }
}

impl fmt::Display for IndexingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.description())
    }
}
