use super::{MAX_SAMPLE_FILE_BYTES, SampleError};

pub(crate) fn read_bounded_optional(
    path: &std::path::Path,
) -> Result<Option<Vec<u8>>, SampleError> {
    let file = match std::fs::File::open(path) {
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
    read_bounded_from(file, capacity).map(Some)
}

fn read_bounded_from(
    mut reader: impl std::io::Read,
    capacity: usize,
) -> Result<Vec<u8>, SampleError> {
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    loop {
        let remaining = capacity - bytes.len();
        let chunk_len = remaining.min(chunk.len());
        let count = match reader.read(&mut chunk[..chunk_len]) {
            Ok(0) => return Ok(bytes),
            Ok(count) => count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(source) => return Err(SampleError::Io { source }),
        };
        let used = bytes.len() + count;
        if used == capacity {
            return Err(SampleError::TooLarge {
                what: "evidence file",
                bytes: used as u64,
                limit: MAX_SAMPLE_FILE_BYTES,
            });
        }
        if used > bytes.capacity() {
            let target = used.max(bytes.capacity().saturating_mul(2)).min(capacity);
            bytes
                .try_reserve_exact(target - bytes.len())
                .map_err(|source| SampleError::Allocation {
                    site: "sample input buffer",
                    requested: target as u64,
                    source,
                })?;
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Read};

    struct Interrupted<R>(bool, R);

    impl<R: Read> Read for Interrupted<R> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if !self.0 {
                self.0 = true;
                return Err(io::ErrorKind::Interrupted.into());
            }
            self.1.read(buf)
        }
    }

    #[test]
    fn small_and_empty_files_use_small_buffers() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("result.json");
        std::fs::write(&path, b"{\"ok\":true}")?;
        let bytes = read_bounded(&path)?;
        assert_eq!(bytes, b"{\"ok\":true}");
        assert!(bytes.capacity() < 8192);
        std::fs::write(&path, [])?;
        let bytes = read_bounded(&path)?;
        assert_eq!(bytes, [] as [u8; 0]);
        assert_eq!(bytes.capacity(), 0);
        Ok(())
    }

    #[test]
    fn interrupted_reads_retry_and_errors_keep_source() -> Result<(), Box<dyn std::error::Error>> {
        struct Failure;
        impl Read for Failure {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "read failed",
                ))
            }
        }
        let input = Interrupted(false, &b"hello"[..]);
        assert_eq!(read_bounded_from(input, 100)?, b"hello");
        assert!(
            matches!(read_bounded_from(Failure, 100), Err(SampleError::Io { source }) if source.kind() == io::ErrorKind::PermissionDenied && source.to_string() == "read failed")
        );
        Ok(())
    }
}
