use std::io::Read as _;

use super::{MAX_SAMPLE_FILE_BYTES, SampleError};

pub(crate) fn read_bounded_optional(
    path: &std::path::Path,
) -> Result<Option<Vec<u8>>, SampleError> {
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(SampleError::Io { source }),
    };
    let capacity =
        usize::try_from(MAX_SAMPLE_FILE_BYTES + 1).map_err(|_| SampleError::ResourceLimit {
            resource: "sample input buffer",
            claimed: Some(MAX_SAMPLE_FILE_BYTES + 1),
            limit: usize::MAX as u64,
            source: None,
        })?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|source| SampleError::Allocation {
            site: "sample input buffer",
            requested: capacity as u64,
            source,
        })?;
    bytes.resize(capacity, 0);
    let mut used = 0;
    while used < capacity {
        match file.read(&mut bytes[used..]) {
            Ok(0) => break,
            Ok(n) => used += n,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(source) => return Err(SampleError::Io { source }),
        }
    }
    if used == capacity {
        return Err(SampleError::TooLarge {
            what: "evidence file",
            bytes: used as u64,
            limit: MAX_SAMPLE_FILE_BYTES,
        });
    }
    bytes.truncate(used);
    Ok(Some(bytes))
}

pub(crate) fn read_bounded(path: &std::path::Path) -> Result<Vec<u8>, SampleError> {
    match read_bounded_optional(path)? {
        Some(bytes) => Ok(bytes),
        None => Err(SampleError::Io {
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("evidence file {} is missing", path.display()),
            ),
        }),
    }
}
