use sha2::{Digest, Sha256};

use crate::check::MISSING_BASELINE;

use super::errors::ApproveError;

fn ci_is_set() -> bool {
    std::env::var_os("CI").is_some()
}

pub(super) fn require_no_ci() -> Result<(), ApproveError> {
    if ci_is_set() {
        return Err(ApproveError::CiApprovalForbidden);
    }
    Ok(())
}

pub(super) fn require_digest(
    role: &str,
    value: &str,
    allow_missing: bool,
) -> Result<(), ApproveError> {
    let invalid = |detail: String| ApproveError::InvalidArgument { detail };
    if value.is_empty() {
        return Err(invalid(format!("{role} digest is missing")));
    }
    if allow_missing && value == MISSING_BASELINE {
        return Ok(());
    }
    if !is_hex_digest(value) {
        return Err(invalid(format!(
            "{role} digest {value:?} is not a 64-character lowercase hex digest"
        )));
    }
    if value.bytes().all(|byte| byte == b'0') {
        return Err(invalid(format!(
            "{role} digest must not be a zero placeholder"
        )));
    }
    Ok(())
}

fn is_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 15) as usize] as char);
    }
    out
}
