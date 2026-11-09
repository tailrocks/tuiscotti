use super::*;
use crate::sample::budget::Phase;
use crate::sample::wire::generated::scan_sample;
use crate::sample::{BinaryFormatError, Gate as DomainGate};
use base64::Engine as _;
use sha2::{Digest, Sha256};
use tui_test::SnapshotColor;
use tui_test::profile::Rgb;
use tui_test::terminal::cell::Attrs;

fn varint(mut value: u64, out: &mut Vec<u8>) {
    while value >= 0x80 {
        out.push((value as u8) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

fn scalar(tag: u32, value: u64, out: &mut Vec<u8>) {
    varint(u64::from(tag) << 3, out);
    varint(value, out);
}

fn fixed32(tag: u32, value: u32, out: &mut Vec<u8>) {
    varint((u64::from(tag) << 3) | 5, out);
    out.extend_from_slice(&value.to_le_bytes());
}

fn fixed64(tag: u32, value: u64, out: &mut Vec<u8>) {
    varint((u64::from(tag) << 3) | 1, out);
    out.extend_from_slice(&value.to_le_bytes());
}

fn bytes(tag: u32, value: &[u8], out: &mut Vec<u8>) {
    varint((u64::from(tag) << 3) | 2, out);
    varint(value.len() as u64, out);
    out.extend_from_slice(value);
}

fn rgb() -> Vec<u8> {
    let mut out = Vec::new();
    scalar(1, 12, &mut out);
    scalar(2, 34, &mut out);
    scalar(3, 56, &mut out);
    out
}

fn default_color() -> Vec<u8> {
    let mut color = Vec::new();
    bytes(1, &[], &mut color);
    color
}

fn cell() -> Vec<u8> {
    let mut out = Vec::new();
    bytes(1, "界".as_bytes(), &mut out);
    scalar(2, 2, &mut out);
    bytes(3, &default_color(), &mut out);
    bytes(4, &default_color(), &mut out);
    for tag in 5..=11 {
        scalar(tag, u64::from(tag == 5 || tag == 7), &mut out);
    }
    scalar(12, 2, &mut out);
    bytes(13, &default_color(), &mut out);
    bytes(14, b"https://example.test/a", &mut out);
    bytes(15, b"link-1", &mut out);
    out
}

fn continuation() -> Vec<u8> {
    let mut out = Vec::new();
    bytes(1, &[], &mut out);
    scalar(2, 0, &mut out);
    bytes(3, &default_color(), &mut out);
    bytes(4, &default_color(), &mut out);
    for tag in 5..=11 {
        scalar(tag, 0, &mut out);
    }
    scalar(12, 1, &mut out);
    bytes(13, &default_color(), &mut out);
    out
}

fn snapshot() -> Vec<u8> {
    let mut out = Vec::new();
    scalar(1, 1, &mut out);
    scalar(2, 2, &mut out);
    scalar(3, 1, &mut out);
    scalar(4, 1, &mut out);
    scalar(5, 0, &mut out);
    bytes(6, &cell(), &mut out);
    bytes(6, &continuation(), &mut out);
    for _ in 0..256 {
        bytes(7, &rgb(), &mut out);
    }
    bytes(8, &rgb(), &mut out);
    bytes(9, &rgb(), &mut out);
    bytes(10, &rgb(), &mut out);
    let mut cursor = Vec::new();
    scalar(1, 0, &mut cursor);
    scalar(2, 0, &mut cursor);
    scalar(3, 1, &mut cursor);
    scalar(4, 1, &mut cursor);
    scalar(5, 1, &mut cursor);
    bytes(6, &rgb(), &mut cursor);
    scalar(7, 0, &mut cursor);
    bytes(11, &cursor, &mut out);
    bytes(12, b"window", &mut out);
    let mut metadata = Vec::new();
    scalar(1, 1_234, &mut metadata);
    scalar(2, 9, &mut metadata);
    bytes(3, b"backend", &mut metadata);
    bytes(4, b"session", &mut metadata);
    scalar(5, 1, &mut metadata);
    bytes(13, &metadata, &mut out);
    out
}

fn valid_wire() -> Vec<u8> {
    let mut out = Vec::new();
    bytes(1, b"tuiscotti.sample.v1", &mut out);
    bytes(2, b"suite/wide-cell", &mut out);
    scalar(3, 1, &mut out);
    bytes(4, &snapshot(), &mut out);
    bytes(7, &[], &mut out);
    out
}

#[test]
fn unsupported_logical_schema_reports_the_rejected_value_verbatim() {
    let mut wire = Vec::new();
    bytes(1, b"tuiscotti.sample.v2", &mut wire);
    let mut budget = DecodeBudget::new().unwrap();

    let error = scan_sample(&wire, &mut budget).unwrap_err();

    let DecodeError::UnsupportedSchema(value) = error else {
        panic!("expected unsupported logical schema")
    };
    assert_eq!(value, "tuiscotti.sample.v2");
}

#[test]
fn unsupported_snapshot_schema_reports_its_version_payload_verbatim() {
    let mut snapshot = Vec::new();
    scalar(1, 2, &mut snapshot);
    let mut wire = Vec::new();
    bytes(4, &snapshot, &mut wire);
    let mut budget = DecodeBudget::new().unwrap();

    let error = scan_sample(&wire, &mut budget).unwrap_err();

    let DecodeError::UnsupportedSchema(value) = error else {
        panic!("expected unsupported snapshot schema")
    };
    assert_eq!(value, "2");
}

fn png() -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, 20, 20);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    let rgba = vec![17; 20 * 20 * 4];
    writer.write_image_data(&rgba).unwrap();
    drop(writer);
    bytes
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn contract(platform: &str) -> Vec<u8> {
    let mut font = Vec::new();
    bytes(
        1,
        b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        &mut font,
    );
    scalar(2, 2, &mut font);
    scalar(3, 1, &mut font);
    scalar(4, 1, &mut font);
    scalar(5, 3, &mut font);

    let mut out = Vec::new();
    bytes(1, b"profile", &mut out);
    scalar(2, 1, &mut out);
    bytes(3, b"renderer", &mut out);
    bytes(4, b"pinned-v1", &mut out);
    bytes(5, &font, &mut out);
    fixed32(6, (-0.0_f32).to_bits(), &mut out);
    fixed32(7, 20.0_f32.to_bits(), &mut out);
    fixed32(8, 12.5_f32.to_bits(), &mut out);
    fixed32(9, 15.0_f32.to_bits(), &mut out);
    fixed64(10, 1.5_f64.to_bits(), &mut out);
    scalar(11, 1, &mut out);
    bytes(
        12,
        b"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        &mut out,
    );
    scalar(13, 2, &mut out);
    scalar(14, 0, &mut out);
    bytes(15, platform.as_bytes(), &mut out);
    out
}

fn pixel_wire() -> (Vec<u8>, Vec<u8>) {
    let png = png();
    let mut image = Vec::new();
    scalar(1, 1, &mut image);
    scalar(2, 20, &mut image);
    scalar(3, 20, &mut image);
    bytes(4, hex(&Sha256::digest(&png)).as_bytes(), &mut image);
    bytes(
        5,
        hex(&Sha256::digest(vec![17; 20 * 20 * 4])).as_bytes(),
        &mut image,
    );
    bytes(6, &png, &mut image);

    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    let mut out = Vec::new();
    bytes(1, b"tuiscotti.sample.v1", &mut out);
    let test_id = format!("suite/visual/pixels-{platform}");
    bytes(2, test_id.as_bytes(), &mut out);
    scalar(3, 2, &mut out);
    bytes(4, &snapshot(), &mut out);
    bytes(5, &contract(&platform), &mut out);
    bytes(6, &image, &mut out);
    let mut observation = Vec::new();
    bytes(1, b"captured", &mut observation);
    bytes(7, &observation, &mut out);
    (out, png)
}

#[test]
#[expect(
    clippy::unwrap_used,
    reason = "test setup unwraps compile-time-valid fixture data"
)]
fn maps_every_snapshot_component_through_upstream_validators() {
    let bytes = valid_wire();
    let mut budget = DecodeBudget::new().unwrap();
    budget
        .reserve_message_error_envelope(u64::try_from(bytes.len()).unwrap())
        .unwrap();
    budget.set_phase(Phase::Codec).unwrap();
    budget.set_phase(Phase::Scan).unwrap();
    let root = scan_sample(&bytes, &mut budget).unwrap();
    let sample = map_sample(root, &mut budget).unwrap();
    budget.finish_output().unwrap();

    assert_eq!(sample.test_id(), "suite/wide-cell");
    assert_eq!(sample.gate(), DomainGate::State);
    assert_eq!(sample.snapshot().cols(), 2);
    assert_eq!(sample.snapshot().rows(), 1);
    assert_eq!(sample.snapshot().cells()[0].text(), "界");
    assert_eq!(sample.snapshot().cells()[0].width(), 2);
    assert_eq!(sample.snapshot().cells()[0].fg(), SnapshotColor::Default);
    assert!(sample.snapshot().cells()[0].has(Attrs::BOLD));
    assert!(sample.snapshot().cells()[0].has(Attrs::ITALIC));
    assert_eq!(sample.snapshot().cells()[0].link_id(), Some("link-1"));
    assert_eq!(sample.snapshot().palette()[255], Rgb::new(12, 34, 56));
    assert_eq!(sample.snapshot().title(), Some("window"));
    assert_eq!(sample.snapshot().metadata().captured_at_ms, 1_234);
    assert_eq!(sample.snapshot().metadata().backend, "backend");
    assert_eq!(sample.snapshot().metadata().session, "session");
    assert_eq!(sample.snapshot().cursor().blink(), Some(false));
    assert!(sample.render_contract().is_none());
    assert!(sample.image().is_none());
    assert!(sample.observation().note().is_none());
}

#[test]
#[expect(
    clippy::unwrap_used,
    reason = "test setup unwraps compile-time-valid fixture data"
)]
fn maps_contract_fonts_and_exact_png_evidence() {
    let (bytes, expected_png) = pixel_wire();
    let mut budget = DecodeBudget::new().unwrap();
    budget
        .reserve_message_error_envelope(u64::try_from(bytes.len()).unwrap())
        .unwrap();
    budget.set_phase(Phase::Codec).unwrap();
    budget.set_phase(Phase::Scan).unwrap();
    let root = scan_sample(&bytes, &mut budget).unwrap();
    let sample = map_sample(root, &mut budget).unwrap();
    budget.finish_output().unwrap();

    let contract = sample.render_contract().unwrap();
    assert_eq!(contract.profile(), "profile");
    assert_eq!(contract.renderer_version(), "renderer");
    assert_eq!(contract.raster_policy(), "pinned-v1");
    assert!(contract.cell_width().is_sign_negative());
    assert_eq!(contract.cell_height(), 20.0);
    assert_eq!(contract.font_size(), 12.5);
    assert_eq!(contract.baseline(), 15.0);
    assert_eq!(contract.scale(), 1.5);
    assert!(contract.title_bar());
    assert_eq!(contract.cursor_phase(), crate::sample::CursorPhase::Hidden);
    assert_eq!(contract.blink(), Some(false));
    assert_eq!(contract.fonts().len(), 1);
    assert_eq!(contract.fonts()[0].digest(), "a".repeat(64));
    assert_eq!(contract.fonts()[0].face_index(), 2);
    assert!(contract.fonts()[0].bold());
    assert!(contract.fonts()[0].italic());
    assert_eq!(contract.fonts()[0].fallback_position(), 3);
    assert_eq!(sample.observation().note(), Some("captured"));
    let image = sample.image().unwrap();
    assert_eq!((image.width(), image.height()), (20, 20));
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(image.png_base64())
            .unwrap(),
        expected_png
    );
}

#[test]
#[expect(clippy::unwrap_used, reason = "test fixture and temporary codec level")]
fn binary_round_trip_through_container_and_allocation_ledger() {
    // Temporary test level only; production level awaits benchmark selection.
    let encoded = super::super::super::container::encode_container(&valid_wire(), 3).unwrap();
    let sample = super::super::decode_sample(&encoded).unwrap();
    let owned = super::super::decode_owned_sample(encoded.clone()).unwrap();
    assert_eq!(sample.test_id(), owned.test_id());
    let second = super::super::encode_sample(&sample, 3).unwrap();
    let decoded = super::super::decode_sample(&second).unwrap();
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
        sample.state_digest().unwrap(),
        decoded.state_digest().unwrap()
    );
    assert!(super::super::decode_sample(b"{} ").is_err());
    let mut trailing = encoded;
    trailing.push(0);
    assert!(super::super::decode_sample(&trailing).is_err());
}

#[test]
#[expect(clippy::unwrap_used, reason = "test fixture and temporary codec level")]
fn binary_pixel_round_trip_preserves_png_and_float_bits() {
    // Temporary test level only; production level awaits benchmark selection.
    let (message, png) = pixel_wire();
    let encoded = super::super::super::container::encode_container(&message, 3).unwrap();
    let sample = super::super::decode_sample(&encoded).unwrap();
    let next = super::super::encode_sample(&sample, 3).unwrap();
    let decoded = super::super::decode_sample(&next).unwrap();
    assert_eq!(
        sample.render_contract().unwrap().cell_width().to_bits(),
        decoded.render_contract().unwrap().cell_width().to_bits()
    );
    assert_eq!(
        sample.state_digest().unwrap(),
        decoded.state_digest().unwrap()
    );
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(decoded.image().unwrap().png_base64())
            .unwrap(),
        png
    );
}

#[test]
#[expect(
    clippy::unwrap_used,
    reason = "test fixtures use a temporary codec level"
)]
fn container_and_wire_reject_corruption_and_duplicate_fields() {
    let mut duplicate = valid_wire();
    bytes(2, b"suite/wide-cell", &mut duplicate);
    let encoded = super::super::super::container::encode_container(&duplicate, 3).unwrap();
    assert!(matches!(
        super::super::decode_sample(&encoded),
        Err(DecodeError::Format(_))
    ));
    let good = super::super::super::container::encode_container(&valid_wire(), 3).unwrap();
    for end in 0..32 {
        assert!(super::super::decode_sample(&good[..end]).is_err());
    }
    let mut version = good.clone();
    version[8] = 2;
    assert!(matches!(
        super::super::decode_sample(&version),
        Err(DecodeError::UnsupportedContainerVersion(2))
    ));
    let mut codec = good.clone();
    codec[10] = 2;
    assert!(matches!(
        super::super::decode_sample(&codec),
        Err(DecodeError::UnsupportedCodec(2))
    ));
    let mut corrupted = good;
    let last = corrupted.len() - 1;
    corrupted[last] ^= 1;
    assert!(matches!(
        super::super::decode_sample(&corrupted),
        Err(DecodeError::Format(_))
    ));
}

#[test]
#[expect(
    clippy::unwrap_used,
    reason = "test fixture uses an explicit temporary codec level"
)]
fn binary_file_entry_bounded_read_and_capacity_accounting() {
    use std::io::Write as _;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("candidate.tui.pb");
    let encoded = super::super::super::container::encode_container(&valid_wire(), 3).unwrap();
    std::fs::File::create(&path)
        .unwrap()
        .write_all(&encoded)
        .unwrap();
    let from_file = super::super::decode_file(&path).unwrap();
    assert_eq!(from_file.test_id(), "suite/wide-cell");
    let bounded = crate::sample::io::read_bounded(&path).unwrap();
    assert_eq!(bounded, encoded);
    assert!(bounded.capacity() >= crate::sample::MAX_SAMPLE_FILE_BYTES as usize + 1);
    let mut smaller = Vec::with_capacity(encoded.len());
    smaller.extend_from_slice(&encoded);
    let small_capacity = smaller.capacity();
    let large_capacity = bounded.capacity();
    let (_, small_peak) = super::super::decode_owned_sample_with_peak(smaller).unwrap();
    let (_, large_peak) = super::super::decode_owned_sample_with_peak(bounded).unwrap();
    assert_eq!(
        large_peak - small_peak,
        (large_capacity - small_capacity) as u64
    );

    let boundary = dir.path().join("boundary.tui.pb");
    std::fs::File::create(&boundary)
        .unwrap()
        .set_len(crate::sample::MAX_SAMPLE_FILE_BYTES)
        .unwrap();
    assert!(matches!(
        super::super::decode_file(&boundary),
        Err(crate::sample::SampleError::BinaryFormat { .. })
    ));
    let oversized = dir.path().join("oversized.tui.pb");
    std::fs::File::create(&oversized)
        .unwrap()
        .set_len(crate::sample::MAX_SAMPLE_FILE_BYTES + 1)
        .unwrap();
    assert!(matches!(
        super::super::decode_file(&oversized),
        Err(crate::sample::SampleError::TooLarge { .. })
    ));
    let missing = super::super::decode_file(&dir.path().join("missing")).unwrap_err();
    assert!(
        matches!(missing, crate::sample::SampleError::Io { source } if source.kind() == std::io::ErrorKind::NotFound)
    );
    let directory = super::super::decode_file(dir.path()).unwrap_err();
    assert!(matches!(directory, crate::sample::SampleError::Io { .. }));
}

#[test]
#[expect(
    clippy::unwrap_used,
    reason = "test fixture uses an explicit temporary codec level"
)]
fn binary_errors_keep_sources_and_expected_public_codes() {
    use crate::approve::sample_code;
    use crate::sample::SampleError;
    use std::error::Error as _;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("candidate.tui.pb");
    let good = super::super::super::container::encode_container(&valid_wire(), 3).unwrap();
    for (offset, code) in [
        (8, "UNSUPPORTED_SCHEMA"),
        (10, "UNSUPPORTED_SCHEMA"),
        (12, "UNSUPPORTED_SCHEMA"),
    ] {
        let mut changed = good.clone();
        changed[offset] = 2;
        std::fs::write(&path, changed).unwrap();
        let error = super::super::decode_file(&path).unwrap_err();
        assert_eq!(sample_code(&error), code);
        assert!(matches!(
            error,
            SampleError::UnsupportedContainerVersion { .. }
                | SampleError::UnsupportedCodec { .. }
                | SampleError::UnsupportedMessageVersion { .. }
        ));
    }
    std::fs::write(&path, b"{}").unwrap();
    let malformed = super::super::decode_file(&path).unwrap_err();
    assert_eq!(sample_code(&malformed), "INVALID_SAMPLE");
    assert!(matches!(malformed, SampleError::BinaryFormat { .. }));
    assert!(malformed.source().is_some());
    let resource: SampleError = DecodeError::resource("test limit", Some(2), 1).into();
    assert_eq!(sample_code(&resource), "RESOURCE_LIMIT");
    let mut reserve = Vec::<u8>::new();
    let source = reserve.try_reserve_exact(usize::MAX).unwrap_err();
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
}

#[test]
fn malformed_repeated_fields_report_their_index_and_restore_parent_path() {
    let mut snap = Vec::new();
    bytes(6, &cell(), &mut snap);
    let mut bad_cell = Vec::new();
    scalar(5, 2, &mut bad_cell);
    bytes(6, &bad_cell, &mut snap);
    let mut wire = Vec::new();
    bytes(4, &snap, &mut wire);
    let mut budget = DecodeBudget::new().unwrap();
    let error = scan_sample(&wire, &mut budget).unwrap_err();
    let DecodeError::Format(BinaryFormatError::Wire { path, .. }) = error else {
        panic!("expected wire error")
    };
    assert_eq!(path.to_string(), "snapshot.cells[1].bold");

    let mut snap = Vec::new();
    bytes(6, &cell(), &mut snap);
    let mut invalid_rgb = Vec::new();
    scalar(1, 256, &mut invalid_rgb);
    bytes(7, &invalid_rgb, &mut snap);
    let mut wire = Vec::new();
    bytes(4, &snap, &mut wire);
    let error = scan_sample(&wire, &mut budget).unwrap_err();
    let DecodeError::Format(BinaryFormatError::Wire { path, .. }) = error else {
        panic!("expected wire error")
    };
    assert_eq!(path.to_string(), "snapshot.palette[0].red");
}

#[test]
#[expect(
    clippy::unwrap_used,
    reason = "test uses a locally constructed compressed malformed message"
)]
fn unsupported_schema_error_retains_the_exact_bounded_wire_value() {
    let rejected = "x".repeat(1024 * 1024);
    let mut wire = Vec::new();
    bytes(1, rejected.as_bytes(), &mut wire);
    let encoded = super::super::super::container::encode_container(&wire, 3).unwrap();
    let error = super::super::decode_sample(&encoded).unwrap_err();
    let DecodeError::UnsupportedSchema(value) = error else {
        panic!("expected unsupported schema")
    };
    assert_eq!(value, rejected);
}
