use super::super::super::SampleError;

pub(super) struct WriterSink {
    bytes: Vec<u8>,
    expected: u64,
}

impl WriterSink {
    pub(super) fn new(expected: u64) -> Result<Self, SampleError> {
        if expected > crate::sample::MAX_SAMPLE_FILE_BYTES {
            return Err(SampleError::TooLarge {
                what: "encoded sample",
                bytes: expected,
                limit: crate::sample::MAX_SAMPLE_FILE_BYTES,
            });
        }
        let capacity = usize::try_from(expected).map_err(|_| SampleError::TooLarge {
            what: "encoded sample",
            bytes: expected,
            limit: crate::sample::MAX_SAMPLE_FILE_BYTES,
        })?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(capacity)
            .map_err(|source| SampleError::EncodeAllocation { source })?;
        Ok(Self { bytes, expected })
    }

    pub(super) fn finish(self) -> Result<Vec<u8>, SampleError> {
        if self.bytes.len() as u64 != self.expected {
            return Err(SampleError::EncodeInvariant {
                detail: "written length differs from measured length",
            });
        }
        Ok(self.bytes)
    }
}

impl super::super::generated::ProtoSink for WriterSink {
    fn position(&self) -> u64 {
        self.bytes.len() as u64
    }

    fn put(&mut self, bytes: &[u8]) -> Result<(), SampleError> {
        let next =
            self.position()
                .checked_add(bytes.len() as u64)
                .ok_or(SampleError::TooLarge {
                    what: "encoded sample",
                    bytes: u64::MAX,
                    limit: self.expected,
                })?;
        if next > self.expected {
            return Err(SampleError::EncodeInvariant {
                detail: "writer exceeded its measured length",
            });
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
}

pub(super) fn map_emit_error(error: super::super::generated::EmitError) -> SampleError {
    match error {
        super::super::generated::EmitError::Sample(error) => error,
        super::super::generated::EmitError::Invariant { detail } => {
            SampleError::EncodeInvariant { detail }
        }
    }
}
