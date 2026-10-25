//! Bounded sample-file reads shared by the commands taking a sample path.
//!
//! Reads stop past [`MAX_SAMPLE_FILE_BYTES`]: oversized input reports a
//! typed failure instead of allocating without bound.

use std::fs;
use std::io::Read as _;
use std::path::Path;

use tuiscotti::sample::MAX_SAMPLE_FILE_BYTES;

/// How a bounded sample read failed.
#[derive(Debug)]
pub(crate) enum ReadFailure {
    /// The file passes the encoded-sample limit.
    TooLarge {
        /// The rejected byte count (limit plus one).
        bytes: u64,
    },
    /// The file could not be opened or read.
    Io {
        /// The filesystem failure.
        source: std::io::Error,
    },
}

impl ReadFailure {
    /// The Section 16.3 code for this failure: both shapes exit 3.
    #[must_use]
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::TooLarge { .. } => "RESOURCE_LIMIT",
            Self::Io { .. } => "IO",
        }
    }

    /// The diagnostic for stderr and the JSON `reason` field.
    #[must_use]
    pub(crate) fn message(&self, path: &Path) -> String {
        match self {
            Self::TooLarge { bytes } => format!(
                "sample {} is {bytes} bytes, above the {MAX_SAMPLE_FILE_BYTES} byte limit",
                path.display()
            ),
            Self::Io { source } => format!("cannot read sample {}: {source}", path.display()),
        }
    }
}

/// Read a sample file with a bounded read.
pub(crate) fn read_sample_bytes(path: &Path) -> Result<Vec<u8>, ReadFailure> {
    let file = fs::File::open(path).map_err(|source| ReadFailure::Io { source })?;
    let mut bounded = file.take(MAX_SAMPLE_FILE_BYTES + 1);
    let mut bytes = Vec::new();
    bounded
        .read_to_end(&mut bytes)
        .map_err(|source| ReadFailure::Io { source })?;
    let len = u64::try_from(bytes.len()).map_err(|_| ReadFailure::TooLarge { bytes: u64::MAX })?;
    if len > MAX_SAMPLE_FILE_BYTES {
        return Err(ReadFailure::TooLarge { bytes: len });
    }
    Ok(bytes)
}
