#![doc = include_str!("../docs/input.md")]
use std::fs;
use std::io::Read as _;
use std::path::Path;

use tuiscotti::sample::MAX_SAMPLE_FILE_BYTES;

#[derive(Debug)]
pub(crate) enum ReadFailure {
    TooLarge { bytes: u64 },
    Io { source: std::io::Error },
}

impl ReadFailure {
    #[must_use]
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::TooLarge { .. } => "RESOURCE_LIMIT",
            Self::Io { .. } => "IO",
        }
    }

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
