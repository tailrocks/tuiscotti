// @generated strict one-pass scanner and bounded frame stack.
use std::fmt::Write as _;

use super::{BinaryFormatError, DecodeBudget, DecodeError, WirePath};

#[derive(Clone, Copy, Debug)]
struct Span<'a> {
    bytes: &'a [u8],
    start: usize,
    end: usize,
}
#[derive(Clone, Copy, Default, Debug)]
struct ScanSummary {
    l: u64,
    c: u64,
    f: u64,
    p: u64,
    test_id: Option<(usize, usize)>,
    platform_key: Option<(usize, usize)>,
    grid: Option<(u32, u32)>,
    image: Option<(u32, u32)>,
}
#[derive(Clone, Copy)]
struct Frame<'a> {
    kind: &'static str,
    span: Span<'a>,
    cursor: usize,
    seen: u64,
    oneofs: u64,
    repeated: [u64; 16],
    values: [u64; 16],
    summary: ScanSummary,
    indexed: bool,
}
#[derive(Clone, Copy, Debug)]
enum FaultKind {
    Wire,
    UnsupportedSchema(u64),
    UnsupportedSampleSchema(usize, usize),
    UnsupportedContract(u32),
    Resource {
        resource: &'static str,
        claimed: Option<u64>,
        limit: u64,
    },
    InvalidSnapshot,
    DimensionMismatch,
    GateMismatch,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct WireFault {
    offset: usize,
    path: WirePath,
    detail: &'static str,
    kind: FaultKind,
}

pub(super) fn scan_sample<'a>(
    bytes: &'a [u8],
    _budget: &mut DecodeBudget,
) -> Result<ValidatedSample<'a>, DecodeError> {
    let span = Span {
        bytes,
        start: 0,
        end: bytes.len(),
    };
    let summary =
        scan(span, "Sample").map_err(|error| to_decode_error_with_bytes(error, Some(bytes)))?;
    Ok(ValidatedSample { span, summary })
}
impl ValidatedSample<'_> {
    pub(super) fn scan_claims(&self) -> (u64, u64, u64, u64) {
        (
            self.summary.l,
            self.summary.c,
            self.summary.f,
            self.summary.p,
        )
    }
}

fn to_decode_error(error: WireFault) -> DecodeError {
    to_decode_error_with_bytes(error, None)
}
fn to_decode_error_with_bytes(error: WireFault, bytes: Option<&[u8]>) -> DecodeError {
    match error.kind {
        FaultKind::Wire => DecodeError::Format(BinaryFormatError::Wire {
            offset: error.offset,
            path: error.path,
            detail: error.detail,
        }),
        FaultKind::UnsupportedSchema(version) => decimal_string(version),
        FaultKind::UnsupportedSampleSchema(start, len) => match bytes {
            Some(bytes) => schema_string(bytes, start, len),
            None => DecodeError::Invariant {
                detail: "unsupported schema diagnostic requires the scanned message",
            },
        },
        FaultKind::UnsupportedContract(version) => {
            DecodeError::Domain(crate::sample::SampleError::UnsupportedContract { version })
        }
        FaultKind::Resource {
            resource,
            claimed,
            limit,
        } => DecodeError::Resource {
            resource,
            claimed,
            limit,
            source: None,
        },
        FaultKind::InvalidSnapshot => {
            DecodeError::Domain(crate::sample::SampleError::InvalidSnapshot {
                detail: error.detail.to_owned(),
            })
        }
        FaultKind::DimensionMismatch => {
            DecodeError::Domain(crate::sample::SampleError::DimensionMismatch {
                detail: error.detail.to_owned(),
            })
        }
        FaultKind::GateMismatch => DecodeError::Domain(crate::sample::SampleError::GateMismatch {
            detail: error.detail.to_owned(),
        }),
    }
}
fn decimal_string(value: u64) -> DecodeError {
    let mut text = String::new();
    if let Err(source) = text.try_reserve_exact(20) {
        return DecodeError::Allocation {
            site: "unsupported schema diagnostic",
            requested: 20,
            source,
        };
    }
    if write!(&mut text, "{value}").is_err() {
        return DecodeError::Invariant {
            detail: "failed to format numeric schema diagnostic",
        };
    }
    DecodeError::UnsupportedSchema(text)
}
fn schema_string(bytes: &[u8], start: usize, len: usize) -> DecodeError {
    let Some(end) = start.checked_add(len) else {
        return DecodeError::Invariant {
            detail: "unsupported schema span overflows the message",
        };
    };
    let Some(value) = bytes.get(start..end) else {
        return DecodeError::Invariant {
            detail: "unsupported schema span is outside the message",
        };
    };
    let Ok(value) = std::str::from_utf8(value) else {
        return DecodeError::Invariant {
            detail: "unsupported schema was not validated as UTF-8",
        };
    };
    let Ok(requested) = u64::try_from(value.len()) else {
        return DecodeError::Invariant {
            detail: "unsupported schema length does not fit accounting",
        };
    };
    let mut text = String::new();
    if let Err(source) = text.try_reserve_exact(value.len()) {
        return DecodeError::Allocation {
            site: "unsupported schema diagnostic",
            requested,
            source,
        };
    }
    text.push_str(value);
    DecodeError::UnsupportedSchema(text)
}
fn fault(offset: usize, path: WirePath, detail: &'static str) -> WireFault {
    WireFault {
        offset,
        path,
        detail,
        kind: FaultKind::Wire,
    }
}
fn resource_fault(
    offset: usize,
    path: WirePath,
    detail: &'static str,
    resource: &'static str,
    claimed: Option<u64>,
    limit: u64,
) -> WireFault {
    WireFault {
        offset,
        path,
        detail,
        kind: FaultKind::Resource {
            resource,
            claimed,
            limit,
        },
    }
}
pub(super) fn read_varint(
    bytes: &[u8],
    cursor: &mut usize,
    end: usize,
    path: WirePath,
) -> Result<u64, WireFault> {
    let start = *cursor;
    let mut value = 0u64;
    for index in 0..10 {
        if *cursor >= end {
            return Err(fault(*cursor, path, "truncated varint"));
        }
        let byte = bytes[*cursor];
        *cursor += 1;
        if index == 9 && byte > 1 {
            return Err(fault(start, path, "varint exceeds 64 bits"));
        }
        value |= u64::from(byte & 0x7f) << (index * 7);
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(fault(start, path, "unterminated varint"))
}
fn read_length(
    bytes: &[u8],
    cursor: &mut usize,
    end: usize,
    path: WirePath,
) -> Result<(usize, usize), WireFault> {
    let length = read_varint(bytes, cursor, end, path)?;
    let length = usize::try_from(length).map_err(|_| {
        resource_fault(
            *cursor,
            path,
            "length does not fit usize",
            "message length",
            Some(length),
            usize::MAX as u64,
        )
    })?;
    let start = *cursor;
    let finish = start.checked_add(length).ok_or_else(|| {
        resource_fault(
            start,
            path,
            "length overflow",
            "message length",
            None,
            usize::MAX as u64,
        )
    })?;
    if finish > end {
        return Err(fault(start, path, "length exceeds containing message"));
    }
    *cursor = finish;
    Ok((start, finish))
}
struct ScanState<'a> {
    stack: [Option<Frame<'a>>; 32],
    depth: usize,
    path: WirePath,
}
#[derive(Clone, Copy)]
struct FieldStep {
    index: usize,
    tag: u32,
    offset: usize,
    path: WirePath,
}
