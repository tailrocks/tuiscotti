use super::*;
use std::error::Error;
use std::fs;

fn container_with_header(version: u16, expanded: u64) -> Vec<u8> {
    let mut bytes = vec![0; 32];
    bytes[..8].copy_from_slice(b"TUISCPB\0");
    bytes[8..10].copy_from_slice(&version.to_le_bytes());
    bytes[10..12].copy_from_slice(&1_u16.to_le_bytes());
    bytes[12..16].copy_from_slice(&1_u32.to_le_bytes());
    bytes[16..24].copy_from_slice(&expanded.to_le_bytes());
    bytes
}

fn save_existing_sample(bytes: &[u8]) -> Result<CheckError, Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let sample_path = temporary.path().join("sample.tui.pb");
    fs::write(&sample_path, bytes)?;
    let before = fs::read(&sample_path)?;
    let result = FrozenStore::save_candidate(temporary.path(), "candidate", b"replacement");
    let error = result
        .err()
        .ok_or("invalid saved sample unexpectedly validated")?;
    assert_eq!(fs::read(&sample_path)?, before);
    let entries = fs::read_dir(temporary.path())?.count();
    assert_eq!(
        entries, 1,
        "validation failure must not create evidence files"
    );
    Ok(error)
}

#[test]
fn saved_sample_resource_failure_keeps_code_and_does_not_write() -> Result<(), Box<dyn Error>> {
    let encoded = container_with_header(1, crate::sample::MAX_SAMPLE_FILE_BYTES + 1);
    let error = save_existing_sample(&encoded)?;
    let CheckError::Sample(sample) = error else {
        return Err("resource failure was classified as evidence corruption".into());
    };
    assert_eq!(crate::approve::sample_code(&sample), "RESOURCE_LIMIT");
    assert!(sample.source().is_none());

    for source in [
        SampleError::DimensionTooLarge {
            width: crate::sample::MAX_IMAGE_DIM + 1,
            height: 1,
            limit: crate::sample::MAX_IMAGE_DIM,
        },
        SampleError::RgbaTooLarge {
            bytes: crate::sample::MAX_RGBA_BYTES + 1,
            limit: crate::sample::MAX_RGBA_BYTES,
        },
    ] {
        let mapped = saved_sample_decode_error(Path::new("sample.tui.pb"), source);
        let CheckError::Sample(sample) = mapped else {
            return Err("image resource failure was classified as evidence corruption".into());
        };
        assert_eq!(crate::approve::sample_code(&sample), "RESOURCE_LIMIT");
    }

    let allocation_source = Vec::<u8>::new()
        .try_reserve_exact(usize::MAX)
        .err()
        .ok_or("reserving usize::MAX bytes unexpectedly succeeded")?;
    let allocation = saved_sample_decode_error(
        Path::new("sample.tui.pb"),
        SampleError::Allocation {
            site: "test sample allocation",
            requested: u64::MAX,
            source: allocation_source,
        },
    );
    let CheckError::Sample(sample) = allocation else {
        return Err("allocation failure was classified as evidence corruption".into());
    };
    assert_eq!(crate::approve::sample_code(&sample), "RESOURCE_LIMIT");
    assert!(sample.source().is_some());
    Ok(())
}

#[test]
fn saved_sample_unsupported_schema_keeps_code_and_does_not_write() -> Result<(), Box<dyn Error>> {
    let encoded = container_with_header(2, 0);
    let error = save_existing_sample(&encoded)?;
    let CheckError::Sample(sample) = error else {
        return Err("unsupported version was classified as evidence corruption".into());
    };
    assert_eq!(crate::approve::sample_code(&sample), "UNSUPPORTED_SCHEMA");
    Ok(())
}

#[test]
fn malformed_saved_sample_remains_evidence_corruption_without_writing() -> Result<(), Box<dyn Error>>
{
    let error = save_existing_sample(b"malformed sample")?;
    assert!(matches!(error, CheckError::EvidenceCorrupt { .. }));
    Ok(())
}

#[test]
fn saved_png_resource_failures_keep_resource_class_and_source() -> Result<(), Box<dyn Error>> {
    fn assert_resource(error: &CheckError, has_source: bool) -> Result<(), Box<dyn Error>> {
        let CheckError::Sample(sample) = error else {
            return Err("PNG resource failure was classified as evidence corruption".into());
        };
        assert_eq!(crate::approve::sample_code(sample), "RESOURCE_LIMIT");
        assert_eq!(sample.source().is_some(), has_source);
        assert!(error.source().is_some());
        Ok(())
    }

    let source = Vec::<u8>::new()
        .try_reserve_exact(usize::MAX)
        .err()
        .ok_or("reserving usize::MAX bytes unexpectedly succeeded")?;
    assert_resource(
        &saved_png_decode_error(
            Path::new("diff.png"),
            SampleError::Allocation {
                site: "test PNG allocation",
                requested: u64::MAX,
                source,
            },
        ),
        true,
    )?;
    assert_resource(
        &saved_png_decode_error(
            Path::new("diff.png"),
            SampleError::PngResourceLimit {
                limit: 128,
                source: crate::sample::PngDecoderError::new(
                    crate::sample::embedded_png::png::DecodingError::LimitsExceeded,
                ),
            },
        ),
        true,
    )?;
    for source in [
        SampleError::DimensionTooLarge {
            width: crate::sample::MAX_IMAGE_DIM + 1,
            height: 1,
            limit: crate::sample::MAX_IMAGE_DIM,
        },
        SampleError::RgbaTooLarge {
            bytes: crate::sample::MAX_RGBA_BYTES + 1,
            limit: crate::sample::MAX_RGBA_BYTES,
        },
    ] {
        assert_resource(
            &saved_png_decode_error(Path::new("diff.png"), source),
            false,
        )?;
    }
    Ok(())
}

#[test]
fn malformed_saved_png_remains_evidence_corruption() {
    let error = saved_png_decode_error(
        Path::new("diff.png"),
        SampleError::PngInvalid {
            detail: "invalid fixture".to_string(),
        },
    );
    assert!(matches!(error, CheckError::EvidenceCorrupt { .. }));
}
