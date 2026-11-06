use tui_test::TerminalSnapshot;

use super::{
    Gate, MAX_IMAGE_DIM, MAX_RGBA_BYTES, MAX_SAMPLE_CELLS, MAX_TEST_ID_BYTES,
    MAX_TEST_ID_SEGMENT_BYTES, SampleError,
};

pub(super) fn validate_test_id(
    test_id: &str,
    gate: Gate,
    pixels_platform: &str,
) -> Result<(), SampleError> {
    validate_test_id_grammar(test_id)?;
    match gate {
        Gate::StateAndPixels => {
            if !test_id
                .strip_suffix(pixels_platform)
                .is_some_and(|prefix| prefix.ends_with("/pixels-"))
            {
                let suffix = format!("/pixels-{pixels_platform}");
                return Err(SampleError::GateMismatch {
                    detail: format!(
                        "state-and-pixels test ID {test_id:?} does not end with {suffix:?}"
                    ),
                });
            }
        }
        Gate::State => {
            if test_id
                .rsplit('/')
                .next()
                .is_some_and(|last| last.starts_with("pixels-"))
            {
                return Err(SampleError::GateMismatch {
                    detail: format!(
                        "state test ID {test_id:?} carries a pixels suffix; use the logical test ID"
                    ),
                });
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_test_id_grammar(test_id: &str) -> Result<(), SampleError> {
    let invalid = |detail: &str| SampleError::InvalidTestId {
        detail: detail.to_string(),
    };
    if test_id.is_empty() {
        return Err(invalid("test ID is empty"));
    }
    if test_id.len() > MAX_TEST_ID_BYTES {
        return Err(SampleError::InvalidTestId {
            detail: format!(
                "test ID is {} bytes, above the {MAX_TEST_ID_BYTES} byte limit",
                test_id.len()
            ),
        });
    }
    if !test_id.is_ascii() {
        return Err(invalid("test ID is not ASCII"));
    }
    if test_id.contains('\\') {
        return Err(invalid("test ID contains a backslash"));
    }
    if test_id.starts_with('/') {
        return Err(invalid("test ID is an absolute path"));
    }
    for segment in test_id.split('/') {
        if segment.is_empty() {
            return Err(invalid("test ID has an empty segment"));
        }
        if segment.len() > MAX_TEST_ID_SEGMENT_BYTES {
            return Err(SampleError::InvalidTestId {
                detail: format!(
                    "test ID segment {segment:?} passes the {MAX_TEST_ID_SEGMENT_BYTES} byte limit"
                ),
            });
        }
        if segment == "." || segment == ".." {
            return Err(SampleError::InvalidTestId {
                detail: format!("test ID segment {segment:?} is a directory reference"),
            });
        }
        let shaped = segment.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        });
        if !shaped {
            return Err(SampleError::InvalidTestId {
                detail: format!("test ID segment {segment:?} breaks the [a-z0-9_-] shape"),
            });
        }
        if is_device_name(segment) {
            return Err(SampleError::InvalidTestId {
                detail: format!("test ID segment {segment:?} is a reserved device name"),
            });
        }
    }
    Ok(())
}

fn is_device_name(segment: &str) -> bool {
    let lower = segment.to_ascii_lowercase();
    let stem = match lower.split('.').next() {
        Some(stem) => stem,
        None => lower.as_str(),
    };
    if matches!(stem, "con" | "prn" | "aux" | "nul") {
        return true;
    }
    let bytes = stem.as_bytes();
    if bytes.len() == 4
        && (stem.starts_with("com") || stem.starts_with("lpt"))
        && bytes[3].is_ascii_digit()
        && bytes[3] != b'0'
    {
        return true;
    }
    false
}
pub(super) fn check_grid_limits(cols: u32, rows: u32, cells: usize) -> Result<(), SampleError> {
    if cols == 0 || rows == 0 {
        return Err(SampleError::InvalidSnapshot {
            detail: "snapshot grid is empty".to_string(),
        });
    }
    if cols > MAX_IMAGE_DIM || rows > MAX_IMAGE_DIM {
        return Err(SampleError::DimensionTooLarge {
            width: cols,
            height: rows,
            limit: MAX_IMAGE_DIM,
        });
    }
    if cells > MAX_SAMPLE_CELLS {
        return Err(SampleError::TooLarge {
            what: "sample cells",
            bytes: cells as u64,
            limit: MAX_SAMPLE_CELLS as u64,
        });
    }
    Ok(())
}

pub(super) fn check_image_dims(width: u32, height: u32) -> Result<(), SampleError> {
    if width == 0 || height == 0 {
        return Err(SampleError::ZeroImageDimensions);
    }
    if width > MAX_IMAGE_DIM || height > MAX_IMAGE_DIM {
        return Err(SampleError::DimensionTooLarge {
            width,
            height,
            limit: MAX_IMAGE_DIM,
        });
    }
    let bytes = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(SampleError::RgbaTooLarge {
            bytes: u64::MAX,
            limit: MAX_RGBA_BYTES,
        })?;
    if bytes > MAX_RGBA_BYTES {
        return Err(SampleError::RgbaTooLarge {
            bytes,
            limit: MAX_RGBA_BYTES,
        });
    }
    Ok(())
}

pub(super) fn check_grid_image(
    snapshot: &TerminalSnapshot,
    width: u32,
    height: u32,
) -> Result<(), SampleError> {
    use crate::render_profile::{CELL_HEIGHT_U32, CELL_WIDTH_U32};
    let expected_width = snapshot.cols().checked_mul(CELL_WIDTH_U32).ok_or_else(|| {
        SampleError::DimensionMismatch {
            detail: "grid width overflows".to_string(),
        }
    })?;
    let expected_height = snapshot
        .rows()
        .checked_mul(CELL_HEIGHT_U32)
        .ok_or_else(|| SampleError::DimensionMismatch {
            detail: "grid height overflows".to_string(),
        })?;
    if (width, height) != (expected_width, expected_height) {
        return Err(SampleError::DimensionMismatch {
            detail: format!(
                "snapshot grid {}x{} needs {expected_width}x{expected_height} pixels, image is {width}x{height}",
                snapshot.cols(),
                snapshot.rows()
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn pixel_suffix_validation_preserves_matching_and_diagnostic() -> Result<(), Box<dyn Error>> {
        assert!(
            validate_test_id(
                "suite/pixels-linux-x86_64",
                Gate::StateAndPixels,
                "linux-x86_64"
            )
            .is_ok()
        );
        assert!(
            validate_test_id("pixels-linux-x86_64", Gate::StateAndPixels, "linux-x86_64").is_err()
        );
        let Err(error) = validate_test_id(
            "suite/pixels-linux-arm64",
            Gate::StateAndPixels,
            "linux-x86_64",
        ) else {
            return Err("mismatched pixel platform unexpectedly validated".into());
        };
        assert_eq!(
            error.to_string(),
            "gate mismatch: state-and-pixels test ID \"suite/pixels-linux-arm64\" does not end with \"/pixels-linux-x86_64\""
        );
        assert!(validate_test_id("suite/state", Gate::State, "linux-x86_64").is_ok());
        Ok(())
    }
}
