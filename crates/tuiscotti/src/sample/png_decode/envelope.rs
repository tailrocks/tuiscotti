use super::super::embedded_png::{fdeflate, png};
use super::{SampleError, arithmetic_error, bounded};

pub(super) const MAX_METADATA_ENVELOPE: u64 = 64 * 1024 * 1024;

pub(super) fn metadata_input_bytes(bytes: &[u8]) -> Result<u64, SampleError> {
    let mut remaining = bytes.get(8..).unwrap_or_default();
    let mut total = 8_u64;
    while !remaining.is_empty() {
        if remaining.len() < 8 {
            total = total
                .checked_add(remaining.len() as u64)
                .ok_or_else(|| arithmetic_error("PNG metadata input bytes"))?;
            break;
        }
        let length =
            u32::from_be_bytes([remaining[0], remaining[1], remaining[2], remaining[3]]) as usize;
        let available = remaining.len().saturating_sub(8);
        let payload = length.min(available);
        let crc_bytes = (available - payload).min(4);
        let complete = crc_bytes == 4;
        let consumed = 8 + payload + crc_bytes;
        let image_data = matches!(&remaining[4..8], b"IDAT" | b"fdAT");
        let end = &remaining[4..8] == b"IEND";
        let metadata = if image_data {
            consumed - payload
        } else {
            consumed
        };
        total = total
            .checked_add(metadata as u64)
            .ok_or_else(|| arithmetic_error("PNG metadata input bytes"))?;
        remaining = &remaining[consumed..];
        if !complete || end {
            break;
        }
    }
    Ok(total)
}

pub(super) fn metadata_envelope(p: u64) -> Result<u64, SampleError> {
    let p = u128::from(p);
    let n = p / 12;
    let s = std::mem::size_of::<png::text_metadata::TEXtChunk>()
        .max(std::mem::size_of::<png::text_metadata::ZTXtChunk>())
        .max(std::mem::size_of::<png::text_metadata::ITXtChunk>()) as u128;
    let value = 3 * p.max(128) + 9 * p + 16 * n + 8 + (3 * n + 12) * s;
    bounded(value, "PNG metadata allocation bytes")
}

pub(super) fn header_envelope(p: u64) -> Result<u64, SampleError> {
    let deflate = 6_u128 * (2288 + 1920)
        + 2 * (4096 + 512) * 4
        + std::mem::size_of::<fdeflate::Decompressor>() as u128;
    bounded(
        u128::from(metadata_envelope(p)?) + deflate,
        "PNG header allocation bytes",
    )
}

pub(super) fn row_envelope(width: u32, height: u32) -> Result<u64, SampleError> {
    let w = u128::from(width);
    let h = u128::from(height);
    let r = 8 * w + 1;
    let t = 7 * h * r;
    let rows = 3 * (t + 8192).max(131_072) + 6 * r.max(8) + 2 * (4 * w).max(8);
    bounded(rows + 1024, "PNG row allocation bytes")
}

pub(super) fn counter_limit(p: u64, width: u32, height: u32) -> Result<usize, SampleError> {
    let value = header_envelope(p)?
        .checked_add(row_envelope(width, height)?)
        .ok_or_else(|| arithmetic_error("PNG decoder allocation bytes"))?;
    Ok(addressable_counter(value, usize::MAX))
}

pub(super) fn addressable_counter(value: u64, addressable: usize) -> usize {
    usize::try_from(value).map_or(addressable, |value| value.min(addressable))
}
