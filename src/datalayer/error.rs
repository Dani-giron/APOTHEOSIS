use std::fmt;
use std::num::ParseIntError;

/// Error returned when constructing a record from an untrusted string, e.g. a
/// line read from a dataset file. Record constructors return this instead of
/// panicking so a malformed entry can be skipped without aborting the whole
/// ingestion.
#[derive(Debug)]
pub enum RecordError {
    /// The input was expected to be a `u32` but did not parse as one.
    InvalidNumber {
        input: String,
        source: ParseIntError,
    },
    /// The input was expected to be a TLSH digest but did not parse as one.
    InvalidTlshHash { input: String },
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecordError::InvalidNumber { input, source } => {
                write!(f, "invalid number record {input:?}: {source}")
            }
            RecordError::InvalidTlshHash { input } => {
                write!(f, "invalid TLSH hash {input:?}")
            }
        }
    }
}

impl std::error::Error for RecordError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RecordError::InvalidNumber { source, .. } => Some(source),
            RecordError::InvalidTlshHash { .. } => None,
        }
    }
}
