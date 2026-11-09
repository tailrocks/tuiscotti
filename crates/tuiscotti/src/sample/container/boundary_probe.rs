use super::*;
use std::error::Error;

#[test]
fn real_container_decode_rejects_precharged_message_and_codec_boundaries()
-> Result<(), Box<dyn Error>> {
    let message = b"allocation boundary probe";
    let input = encode_container(message, 3).map_err(|e| format!("{e:?}"))?;
    let n = message.len() as u64;
    for (name, available) in [
        ("message", 33 * n - 1),
        ("codec", 33 * n + CODEC_ENVELOPE - 1),
    ] {
        let mut budget = DecodeBudget::new().map_err(|e| format!("{e:?}"))?;
        let filler = budget.limit() - budget.total() - available;
        let _held = budget
            .charge("injected pressure", filler)
            .map_err(|e| format!("{e:?}"))?;
        let Err(error) = decode_container(&input, &mut budget) else {
            return Err(format!("{name} unexpectedly succeeded").into());
        };
        let error: super::super::SampleError = error.into();
        assert!(matches!(
            error,
            super::super::SampleError::ResourceLimit {
                claimed: Some(_),
                ..
            }
        ));
        assert_eq!(crate::approve::sample_code(&error), "RESOURCE_LIMIT");
        eprintln!(
            "{name}: available={available}, input={}, message={n}, codec_envelope={CODEC_ENVELOPE}, error={error:?}",
            input.len()
        );
    }
    Ok(())
}

#[test]
fn zstd_allocation_error_is_a_resource_failure_and_keeps_its_source() {
    let code =
        (zstd_safe::zstd_sys::ZSTD_ErrorCode::ZSTD_error_memory_allocation as usize).wrapping_neg();
    let error: super::super::SampleError = codec_error(code).into();
    assert!(matches!(
        error,
        super::super::SampleError::ZstdResourceLimit { .. }
    ));
    assert_eq!(crate::approve::sample_code(&error), "RESOURCE_LIMIT");
    assert_eq!(
        error.source().map(ToString::to_string),
        Some(zstd_safe::get_error_name(code).to_owned())
    );

    let invalid = (zstd_safe::zstd_sys::ZSTD_ErrorCode::ZSTD_error_corruption_detected as usize)
        .wrapping_neg();
    let error: super::super::SampleError = codec_error(invalid).into();
    assert!(matches!(
        error,
        super::super::SampleError::BinaryFormat { .. }
    ));
    assert_eq!(crate::approve::sample_code(&error), "INVALID_SAMPLE");
    assert!(error.source().and_then(Error::source).is_some());
}
