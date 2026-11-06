use super::*;
use std::error::Error;
use std::io;

fn checked<T, E: std::fmt::Debug>(result: Result<T, E>) -> Result<T, Box<dyn Error>> {
    result.map_err(|error| io::Error::other(format!("{error:?}")).into())
}

fn expected_error<T, E: std::fmt::Debug>(result: Result<T, E>) -> io::Result<E> {
    result
        .err()
        .ok_or_else(|| io::Error::other("operation unexpectedly succeeded"))
}

#[test]
fn binary_round_trip_through_container_and_allocation_ledger() -> Result<(), Box<dyn Error>> {
    let encoded = checked(super::super::super::super::container::encode_container(
        &valid_wire(),
        3,
    ))?;
    let sample = checked(super::super::super::decode_sample(&encoded))?;
    let owned = checked(super::super::super::decode_owned_sample(encoded.clone()))?;
    assert_eq!(sample.test_id(), owned.test_id());
    let second = checked(super::super::super::encode_sample(&sample, 3))?;
    let decoded = checked(super::super::super::decode_sample(&second))?;
    assert_eq!(sample.test_id(), decoded.test_id());
    assert_eq!(
        sample.snapshot().metadata().backend,
        decoded.snapshot().metadata().backend
    );
    assert_eq!(
        sample.snapshot().cells()[0].text(),
        decoded.snapshot().cells()[0].text()
    );
    assert_eq!(
        checked(sample.state_digest())?,
        checked(decoded.state_digest())?
    );
    assert!(super::super::super::decode_sample(b"{} ").is_err());
    let mut trailing = encoded;
    trailing.push(0);
    assert!(super::super::super::decode_sample(&trailing).is_err());
    Ok(())
}

#[test]
fn public_sample_encoder_uses_level_22_and_decodes() -> Result<(), Box<dyn Error>> {
    assert_eq!(zstd_safe::max_c_level(), 22);
    let encoded = checked(super::super::super::super::container::encode_container(
        &valid_wire(),
        3,
    ))?;
    let sample = checked(super::super::super::decode_sample(&encoded))?;

    let public = checked(sample.to_bytes())?;
    let expected = checked(super::super::super::encode_sample(&sample, 22))?;
    assert_eq!(public, expected, "public writer uses the measured setting");

    let decoded = checked(Sample::from_bytes(&public))?;
    assert_eq!(
        checked(sample.state_digest())?,
        checked(decoded.state_digest())?
    );
    Ok(())
}

#[test]
fn binary_pixel_round_trip_preserves_png_and_float_bits() -> Result<(), Box<dyn Error>> {
    let (message, png) = pixel_wire()?;
    let encoded = checked(super::super::super::super::container::encode_container(
        &message, 3,
    ))?;
    let sample = checked(super::super::super::decode_sample(&encoded))?;
    let next = checked(super::super::super::encode_sample(&sample, 3))?;
    let decoded = checked(super::super::super::decode_sample(&next))?;
    let sample_contract = sample
        .render_contract()
        .ok_or_else(|| io::Error::other("decoded pixel sample has no contract"))?;
    let decoded_contract = decoded
        .render_contract()
        .ok_or_else(|| io::Error::other("round-tripped pixel sample has no contract"))?;
    assert_eq!(
        sample_contract.cell_width().to_bits(),
        decoded_contract.cell_width().to_bits()
    );
    assert_eq!(
        checked(sample.state_digest())?,
        checked(decoded.state_digest())?
    );
    let decoded_image = decoded
        .image()
        .ok_or_else(|| io::Error::other("round-tripped sample has no image"))?;
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(decoded_image.png_base64())
            .map_err(io::Error::other)?,
        png
    );
    Ok(())
}

#[test]
fn container_and_wire_reject_corruption_and_duplicate_fields() -> Result<(), Box<dyn Error>> {
    let mut duplicate = valid_wire();
    bytes(2, b"suite/wide-cell", &mut duplicate);
    let encoded = checked(super::super::super::super::container::encode_container(
        &duplicate, 3,
    ))?;
    assert!(matches!(
        super::super::super::decode_sample(&encoded),
        Err(DecodeError::Format(_))
    ));
    let good = checked(super::super::super::super::container::encode_container(
        &valid_wire(),
        3,
    ))?;
    for end in 0..32 {
        assert!(super::super::super::decode_sample(&good[..end]).is_err());
    }
    let mut version = good.clone();
    version[8] = 2;
    assert!(matches!(
        super::super::super::decode_sample(&version),
        Err(DecodeError::UnsupportedContainerVersion(2))
    ));
    let mut codec = good.clone();
    codec[10] = 2;
    assert!(matches!(
        super::super::super::decode_sample(&codec),
        Err(DecodeError::UnsupportedCodec(2))
    ));
    let mut corrupted = good;
    let last = corrupted.len() - 1;
    corrupted[last] ^= 1;
    assert!(matches!(
        super::super::super::decode_sample(&corrupted),
        Err(DecodeError::Format(_))
    ));
    Ok(())
}

#[test]
fn binary_file_entry_bounded_read_and_capacity_accounting() -> Result<(), Box<dyn Error>> {
    use std::io::Write as _;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("candidate.tui.pb");
    let encoded = checked(super::super::super::super::container::encode_container(
        &valid_wire(),
        3,
    ))?;
    std::fs::File::create(&path).and_then(|mut file| file.write_all(&encoded))?;
    let from_file = checked(super::super::super::decode_file(&path))?;
    assert_eq!(from_file.test_id(), "suite/wide-cell");
    let bounded = checked(crate::sample::io::read_bounded(&path))?;
    assert_eq!(bounded, encoded);
    assert!(bounded.capacity() < 8192);
    let mut smaller = Vec::with_capacity(encoded.len());
    smaller.extend_from_slice(&encoded);
    let small_capacity = smaller.capacity();
    let mut larger = Vec::with_capacity(small_capacity + 8192);
    larger.extend_from_slice(&bounded);
    let large_capacity = larger.capacity();
    let (_, small_peak) = checked(super::super::super::decode_owned_sample_with_peak(smaller))?;
    let (_, large_peak) = checked(super::super::super::decode_owned_sample_with_peak(larger))?;
    assert_eq!(
        large_peak - small_peak,
        u64::try_from(large_capacity - small_capacity)?
    );

    let boundary = dir.path().join("boundary.tui.pb");
    std::fs::File::create(&boundary)?.set_len(crate::sample::MAX_SAMPLE_FILE_BYTES)?;
    assert!(matches!(
        super::super::super::decode_file(&boundary),
        Err(crate::sample::SampleError::BinaryFormat { .. })
    ));
    let oversized = dir.path().join("oversized.tui.pb");
    std::fs::File::create(&oversized)?.set_len(crate::sample::MAX_SAMPLE_FILE_BYTES + 1)?;
    assert!(matches!(
        super::super::super::decode_file(&oversized),
        Err(crate::sample::SampleError::TooLarge { .. })
    ));
    let missing = expected_error(super::super::super::decode_file(
        &dir.path().join("missing"),
    ))?;
    assert!(
        matches!(missing, crate::sample::SampleError::Io { source } if source.kind() == std::io::ErrorKind::NotFound)
    );
    let directory = expected_error(super::super::super::decode_file(dir.path()))?;
    assert!(matches!(directory, crate::sample::SampleError::Io { .. }));
    Ok(())
}

#[test]
fn binary_errors_keep_sources_and_expected_public_codes() -> Result<(), Box<dyn Error>> {
    use crate::approve::sample_code;
    use crate::sample::SampleError;
    use std::error::Error as _;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("candidate.tui.pb");
    let good = checked(super::super::super::super::container::encode_container(
        &valid_wire(),
        3,
    ))?;
    for (offset, code) in [
        (8, "UNSUPPORTED_SCHEMA"),
        (10, "UNSUPPORTED_SCHEMA"),
        (12, "UNSUPPORTED_SCHEMA"),
    ] {
        let mut changed = good.clone();
        changed[offset] = 2;
        std::fs::write(&path, changed)?;
        let error = expected_error(super::super::super::decode_file(&path))?;
        assert_eq!(sample_code(&error), code);
        assert!(matches!(
            error,
            SampleError::UnsupportedContainerVersion { .. }
                | SampleError::UnsupportedCodec { .. }
                | SampleError::UnsupportedMessageVersion { .. }
        ));
    }
    std::fs::write(&path, b"{}")?;
    let malformed = expected_error(super::super::super::decode_file(&path))?;
    assert_eq!(sample_code(&malformed), "INVALID_SAMPLE");
    assert!(matches!(malformed, SampleError::BinaryFormat { .. }));
    assert!(malformed.source().is_some());
    let resource: SampleError = DecodeError::resource("test limit", Some(2), 1).into();
    assert_eq!(sample_code(&resource), "RESOURCE_LIMIT");
    let mut reserve = Vec::<u8>::new();
    let source = reserve
        .try_reserve_exact(usize::MAX)
        .err()
        .ok_or_else(|| io::Error::other("reserving usize::MAX bytes unexpectedly succeeded"))?;
    let allocation_failure: SampleError = DecodeError::Allocation {
        site: "test",
        requested: u64::MAX,
        source,
    }
    .into();
    assert_eq!(sample_code(&allocation_failure), "RESOURCE_LIMIT");
    assert!(allocation_failure.source().is_some());
    let allocation: SampleError = DecodeError::ContextAllocation { site: "test" }.into();
    assert_eq!(sample_code(&allocation), "RESOURCE_LIMIT");
    let invariant: SampleError = DecodeError::Invariant { detail: "test" }.into();
    assert!(matches!(invariant, SampleError::DecodeInvariant { .. }));
    assert_eq!(sample_code(&invariant), "INVALID_SAMPLE");
    assert_eq!(
        sample_code(&SampleError::PngInvalid {
            detail: "test".into()
        }),
        "INVALID_IMAGE"
    );
    assert_eq!(
        sample_code(&SampleError::Io {
            source: std::io::Error::other("test")
        }),
        "IO"
    );
    Ok(())
}

#[test]
fn malformed_repeated_fields_report_their_index_and_restore_parent_path()
-> Result<(), Box<dyn Error>> {
    let mut snap = Vec::new();
    bytes(6, &cell(), &mut snap);
    let mut bad_cell = Vec::new();
    scalar(5, 2, &mut bad_cell);
    bytes(6, &bad_cell, &mut snap);
    let mut wire = Vec::new();
    bytes(4, &snap, &mut wire);
    let mut budget = DecodeBudget::new().map_err(|error| io::Error::other(format!("{error:?}")))?;
    let error = expected_error(scan_sample(&wire, &mut budget))?;
    let DecodeError::Format(BinaryFormatError::Wire { path, .. }) = error else {
        return Err(io::Error::other("expected wire error for malformed cell").into());
    };
    assert_eq!(path.to_string(), "snapshot.cells[1].bold");

    let mut snap = Vec::new();
    bytes(6, &cell(), &mut snap);
    let mut invalid_rgb = Vec::new();
    scalar(1, 256, &mut invalid_rgb);
    bytes(7, &invalid_rgb, &mut snap);
    let mut wire = Vec::new();
    bytes(4, &snap, &mut wire);
    let error = expected_error(scan_sample(&wire, &mut budget))?;
    let DecodeError::Format(BinaryFormatError::Wire { path, .. }) = error else {
        return Err(io::Error::other("expected wire error for invalid palette color").into());
    };
    assert_eq!(path.to_string(), "snapshot.palette[0].red");
    Ok(())
}

#[test]
fn unsupported_schema_error_retains_the_exact_bounded_wire_value() -> Result<(), Box<dyn Error>> {
    let rejected = "x".repeat(1024 * 1024);
    let mut wire = Vec::new();
    bytes(1, rejected.as_bytes(), &mut wire);
    let encoded = checked(super::super::super::super::container::encode_container(
        &wire, 3,
    ))?;
    let error = expected_error(super::super::super::decode_sample(&encoded))?;
    let DecodeError::UnsupportedSchema(value) = error else {
        return Err(io::Error::other("expected unsupported schema error").into());
    };
    assert_eq!(value, rejected);
    Ok(())
}

#[test]
fn geometry_rejects_nonfinite_wire_values_with_field_paths() -> Result<(), Box<dyn Error>> {
    for (tag, name) in [
        (6, "cell_width"),
        (7, "cell_height"),
        (8, "font_size"),
        (9, "baseline"),
        (10, "scale"),
    ] {
        for bits in [0, 1, 2] {
            let mut contract = Vec::new();
            if tag == 10 {
                fixed64(
                    tag,
                    [f64::NAN, f64::INFINITY, f64::NEG_INFINITY][bits].to_bits(),
                    &mut contract,
                );
            } else {
                fixed32(
                    tag,
                    [f32::NAN, f32::INFINITY, f32::NEG_INFINITY][bits].to_bits(),
                    &mut contract,
                );
            }
            let mut wire = Vec::new();
            bytes(5, &contract, &mut wire);
            let mut budget = checked(DecodeBudget::new())?;
            let error = expected_error(scan_sample(&wire, &mut budget))?;
            let DecodeError::Format(BinaryFormatError::Wire { path, .. }) = error else {
                return Err(io::Error::other(format!("expected wire error: {error:?}")).into());
            };
            assert_eq!(path.to_string(), format!("render_contract.{name}"));
        }
    }
    Ok(())
}

#[test]
fn finite_geometry_bits_round_trip_and_nonfinite_samples_cannot_encode()
-> Result<(), Box<dyn Error>> {
    let (message, _) = pixel_wire()?;
    let encoded = checked(super::super::super::super::container::encode_container(
        &message, 3,
    ))?;
    let mut sample = checked(super::super::super::decode_sample(&encoded))?;
    let contract = sample
        .render_contract
        .as_mut()
        .ok_or_else(|| io::Error::other("missing contract"))?;
    contract.cell_width = -0.0;
    contract.cell_height = f32::from_bits(1);
    contract.font_size = f32::MAX;
    contract.baseline = f32::MIN;
    contract.scale = f64::from_bits(1);
    let encoded = checked(sample.to_bytes())?;
    let decoded = checked(Sample::from_bytes(&encoded))?;
    let actual = decoded
        .render_contract()
        .ok_or_else(|| io::Error::other("missing contract"))?;
    assert_eq!(actual.cell_width().to_bits(), (-0.0_f32).to_bits());
    assert_eq!(actual.cell_height().to_bits(), 1);
    assert_eq!(actual.font_size().to_bits(), f32::MAX.to_bits());
    assert_eq!(actual.baseline().to_bits(), f32::MIN.to_bits());
    assert_eq!(actual.scale().to_bits(), 1);
    for field in 0..5 {
        let mut invalid = sample.clone();
        let contract = invalid
            .render_contract
            .as_mut()
            .ok_or_else(|| io::Error::other("missing contract"))?;
        match field {
            0 => contract.cell_width = f32::NAN,
            1 => contract.cell_height = f32::INFINITY,
            2 => contract.font_size = f32::NEG_INFINITY,
            3 => contract.baseline = f32::NAN,
            _ => contract.scale = f64::INFINITY,
        }
        assert!(invalid.to_bytes().is_err());
    }
    Ok(())
}
