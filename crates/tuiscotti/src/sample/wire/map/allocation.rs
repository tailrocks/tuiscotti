use std::collections::TryReserveError;

use super::super::super::SampleError;
use super::super::super::decode_error::DecodeError;

pub(super) fn allocation(
    site: &'static str,
    requested: usize,
    source: TryReserveError,
) -> DecodeError {
    DecodeError::Allocation {
        site,
        requested: u64::try_from(requested).unwrap_or(u64::MAX),
        source,
    }
}

pub(super) fn copied_string(value: &str, site: &'static str) -> Result<String, DecodeError> {
    let mut result = String::new();
    result
        .try_reserve_exact(value.len())
        .map_err(|source| allocation(site, value.len(), source))?;
    result.push_str(value);
    Ok(result)
}

pub(super) fn copied_vec<T>(capacity: usize, site: &'static str) -> Result<Vec<T>, DecodeError> {
    let requested = capacity
        .checked_mul(std::mem::size_of::<T>())
        .ok_or_else(|| DecodeError::resource(site, None, u64::MAX))?;
    let mut result = Vec::new();
    result
        .try_reserve_exact(capacity)
        .map_err(|source| allocation(site, requested, source))?;
    Ok(result)
}

pub(super) fn domain_error(error: impl std::fmt::Display) -> DecodeError {
    DecodeError::Domain(SampleError::InvalidSnapshot {
        detail: error.to_string(),
    })
}

pub(super) fn matches_hex(bytes: &[u8], expected: &str) -> bool {
    if expected.len() != bytes.len() * 2 {
        return false;
    }
    bytes.iter().enumerate().all(|(index, byte)| {
        let high = b"0123456789abcdef"[usize::from(byte >> 4)];
        let low = b"0123456789abcdef"[usize::from(byte & 0x0f)];
        expected.as_bytes().get(index * 2) == Some(&high)
            && expected.as_bytes().get(index * 2 + 1) == Some(&low)
    })
}

pub(super) fn hex(bytes: &[u8]) -> Result<String, DecodeError> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let capacity = bytes
        .len()
        .checked_mul(2)
        .ok_or_else(|| DecodeError::resource("digest text bytes", None, u64::MAX))?;
    let mut result = String::new();
    result
        .try_reserve_exact(capacity)
        .map_err(|source| allocation("digest text", capacity, source))?;
    for byte in bytes {
        result.push(char::from(HEX[usize::from(byte >> 4)]));
        result.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    Ok(result)
}
