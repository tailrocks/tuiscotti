#![doc = include_str!("../docs/input.md")]
use std::fs;
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
    let file = fs::File::open(path).map_err(|source| ReadFailure::Io { source })?;
    let max_bytes_plus_one =
        usize::try_from(MAX_SAMPLE_FILE_BYTES + 1).map_err(|_| ReadFailure::TooLarge {
            bytes: MAX_SAMPLE_FILE_BYTES + 1,
        })?;
    read_bounded_from(file, max_bytes_plus_one)
}

fn read_bounded_from(
    mut reader: impl std::io::Read,
    max_bytes_plus_one: usize,
) -> Result<Vec<u8>, ReadFailure> {
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    loop {
        let remaining = max_bytes_plus_one - bytes.len();
        let chunk_len = remaining.min(chunk.len());
        let count = match reader.read(&mut chunk[..chunk_len]) {
            Ok(0) => return Ok(bytes),
            Ok(count) => count,
            Err(source) if source.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(source) => return Err(ReadFailure::Io { source }),
        };
        let used = bytes.len() + count;
        if used == max_bytes_plus_one {
            return Err(ReadFailure::TooLarge { bytes: used as u64 });
        }
        if used > bytes.capacity() {
            let target = used
                .max(bytes.capacity().saturating_mul(2))
                .min(max_bytes_plus_one);
            bytes
                .try_reserve_exact(target - bytes.len())
                .map_err(|source| ReadFailure::Allocation { source })?;
        }
        bytes.extend_from_slice(&chunk[..count]);
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
    fn small_and_empty_inputs_allocate_only_for_their_contents() {
        let bytes = read_bounded_from(&b"small"[..], 9).expect("small input reads");
        assert_eq!(bytes, b"small");
        assert!(bytes.capacity() < 9);

        let empty = read_bounded_from(&b""[..], 9).expect("empty input reads");
        assert_eq!(empty, [] as [u8; 0]);
        assert_eq!(empty.capacity(), 0);
    }

    #[test]
    fn exact_limit_is_accepted_and_limit_plus_one_is_rejected() {
        assert_eq!(
            read_bounded_from(&b"12345678"[..], 9).expect("input at the limit reads"),
            b"12345678"
        );
        let too_large =
            read_bounded_from(&b"123456789"[..], 9).expect_err("limit plus one byte must fail");
        assert!(matches!(too_large, ReadFailure::TooLarge { bytes: 9 }));
        assert_eq!(too_large.code(), "RESOURCE_LIMIT");
    }

    #[test]
    fn interrupted_reads_retry_and_other_errors_keep_the_source() {
        struct Failure;
        impl Read for Failure {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "read failed",
                ))
            }
        }

        assert_eq!(
            read_bounded_from(Interrupted(false, &b"hello"[..]), 16)
                .expect("interrupted input resumes"),
            b"hello"
        );
        let failure = read_bounded_from(Failure, 16).expect_err("read error must be returned");
        assert_eq!(failure.code(), "IO");
        assert!(matches!(
            failure,
            ReadFailure::Io { source }
                if source.kind() == io::ErrorKind::PermissionDenied
                    && source.to_string() == "read failed"
        ));
    }

    #[test]
    fn file_open_errors_keep_the_cli_io_mapping() {
        let path = Path::new("missing-input-for-tuiscotti-test");
        assert!(matches!(
            read_sample_bytes(path),
            Err(ReadFailure::Io { source }) if source.kind() == io::ErrorKind::NotFound
        ));
    }
}
