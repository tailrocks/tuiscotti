#![doc = include_str!("../docs/input.md")]
use std::fs;
use std::io::Read as _;
use std::path::Path;

use tuiscotti::sample::MAX_SAMPLE_FILE_BYTES;

#[derive(Debug)]
pub(crate) enum ReadFailure {
    TooLarge {
        bytes: u64,
    },
    Io {
        source: std::io::Error,
    },
    Allocation {
        source: std::collections::TryReserveError,
    },
}

impl ReadFailure {
    #[must_use]
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::TooLarge { .. } | Self::Allocation { .. } => "RESOURCE_LIMIT",
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
            Self::Allocation { source } => format!("cannot allocate sample input buffer: {source}"),
        }
    }
}

pub(crate) fn read_sample_bytes(path: &Path) -> Result<Vec<u8>, ReadFailure> {
    let mut file = fs::File::open(path).map_err(|source| ReadFailure::Io { source })?;
    let capacity =
        usize::try_from(MAX_SAMPLE_FILE_BYTES + 1).map_err(|_| ReadFailure::TooLarge {
            bytes: MAX_SAMPLE_FILE_BYTES + 1,
        })?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|source| ReadFailure::Allocation { source })?;
    bytes.resize(capacity, 0);
    let mut used = 0;
    while used < capacity {
        match file.read(&mut bytes[used..]) {
            Ok(0) => break,
            Ok(n) => used += n,
            Err(source) if source.kind() == std::io::ErrorKind::Interrupted => {}
            Err(source) => return Err(ReadFailure::Io { source }),
        }
    }
    if used == capacity {
        return Err(ReadFailure::TooLarge { bytes: used as u64 });
    }
    bytes.truncate(used);
    Ok(bytes)
}
