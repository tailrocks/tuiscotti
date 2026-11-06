// @generated sink primitives and canonical Protobuf wire operations.
use crate::sample::SampleError;

const MAX_ENCODED_BYTES: u64 = 64 * 1024 * 1024;

pub(super) trait ProtoSink {
    fn position(&self) -> u64;
    fn put(&mut self, bytes: &[u8]) -> Result<(), SampleError>;
}

#[derive(Debug)]
pub(super) enum EmitError {
    Sample(SampleError),
    Invariant { detail: &'static str },
}

impl From<SampleError> for EmitError {
    fn from(error: SampleError) -> Self {
        Self::Sample(error)
    }
}

pub(super) struct CountSink {
    position: u64,
}

impl CountSink {
    pub(super) fn new() -> Self {
        Self { position: 0 }
    }
}

#[cfg(test)]
pub(super) struct VecSink {
    bytes: Vec<u8>,
    limit: usize,
}

#[cfg(test)]
impl VecSink {
    pub(super) fn new(limit: usize) -> Result<Self, SampleError> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(limit)
            .map_err(|source| SampleError::Allocation {
                site: "generated writer test sink",
                requested: u64::try_from(limit).unwrap_or(u64::MAX),
                source,
            })?;
        Ok(Self { bytes, limit })
    }
    pub(super) fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

#[cfg(test)]
impl ProtoSink for VecSink {
    fn position(&self) -> u64 {
        self.bytes.len() as u64
    }
    fn put(&mut self, bytes: &[u8]) -> Result<(), SampleError> {
        let next = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or(SampleError::TooLarge {
                what: "generated writer test sink",
                bytes: u64::MAX,
                limit: self.limit as u64,
            })?;
        if next > self.limit {
            return Err(SampleError::TooLarge {
                what: "generated writer test sink",
                bytes: next as u64,
                limit: self.limit as u64,
            });
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn vector_sink_preserves_written_bytes_and_position() -> Result<(), SampleError> {
    let mut sink = VecSink::new(2)?;
    sink.put(&[1, 2])?;
    assert_eq!(sink.position(), 2);
    assert_eq!(sink.finish(), [1, 2]);
    Ok(())
}

impl ProtoSink for CountSink {
    fn position(&self) -> u64 {
        self.position
    }
    fn put(&mut self, bytes: &[u8]) -> Result<(), SampleError> {
        let next = self
            .position
            .checked_add(bytes.len() as u64)
            .ok_or(SampleError::TooLarge {
                what: "encoded sample",
                bytes: u64::MAX,
                limit: MAX_ENCODED_BYTES,
            })?;
        if next > MAX_ENCODED_BYTES {
            return Err(SampleError::TooLarge {
                what: "encoded sample",
                bytes: next,
                limit: MAX_ENCODED_BYTES,
            });
        }
        self.position = next;
        Ok(())
    }
}

pub(super) fn put_varint<S: ProtoSink>(sink: &mut S, mut value: u64) -> Result<(), EmitError> {
    let mut bytes = [0u8; 10];
    let mut used = 0;
    while value >= 0x80 {
        bytes[used] = value.to_le_bytes()[0] | 0x80;
        used += 1;
        value >>= 7;
    }
    bytes[used] = value.to_le_bytes()[0];
    sink.put(&bytes[..=used])?;
    Ok(())
}
pub(super) fn put_key<S: ProtoSink>(sink: &mut S, tag: u32, wire: u32) -> Result<(), EmitError> {
    let key = tag
        .checked_mul(8)
        .and_then(|v| v.checked_add(wire))
        .ok_or(EmitError::Invariant {
            detail: "field key overflow",
        })?;
    put_varint(sink, u64::from(key))
}
pub(super) fn put_varint_field<S: ProtoSink>(
    sink: &mut S,
    tag: u32,
    value: u64,
) -> Result<(), EmitError> {
    put_key(sink, tag, 0)?;
    put_varint(sink, value)
}
pub(super) fn put_fixed32_field<S: ProtoSink>(
    sink: &mut S,
    tag: u32,
    value: u32,
) -> Result<(), EmitError> {
    put_key(sink, tag, 5)?;
    sink.put(&value.to_le_bytes())?;
    Ok(())
}
pub(super) fn put_fixed64_field<S: ProtoSink>(
    sink: &mut S,
    tag: u32,
    value: u64,
) -> Result<(), EmitError> {
    put_key(sink, tag, 1)?;
    sink.put(&value.to_le_bytes())?;
    Ok(())
}
pub(super) fn put_bytes_field<S: ProtoSink>(
    sink: &mut S,
    tag: u32,
    value: &[u8],
) -> Result<(), EmitError> {
    put_key(sink, tag, 2)?;
    put_varint(sink, value.len() as u64)?;
    sink.put(value)?;
    Ok(())
}
pub(super) fn put_nested_prefix<S: ProtoSink>(
    sink: &mut S,
    tag: u32,
    len: u64,
) -> Result<(), EmitError> {
    put_key(sink, tag, 2)?;
    put_varint(sink, len)
}
fn verify_nested_length(start: u64, end: u64, expected: u64) -> Result<(), EmitError> {
    if end.checked_sub(start) != Some(expected) {
        return Err(EmitError::Invariant {
            detail: "nested encoded length changed between passes",
        });
    }
    Ok(())
}
