//! Binary persistence and CLI contract tests using independently assembled wire messages.

#![expect(
    clippy::expect_used,
    reason = "integration failures need actionable setup context"
)]

#[path = "compressed_binary/fixtures.rs"]
mod fixtures;

use std::io::Write as _;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use fixtures::{
    TEST_ID, container, dirs, header_mut, message_with_duplicate_union, message_with_gate_wire,
    message_with_schema, message_with_test_id, put_u16, run_cli, sha256, truncated_header,
    uncontainer, valid_container, valid_message, valid_message_with_text,
    valid_noncanonical_message, write,
};
use flate2::Compression;
use flate2::write::ZlibEncoder;
use prost::Message as _;
use tui_test::{SnapshotColor, SnapshotScope, SnapshotSource, SnapshotUnderline};
use tuiscotti::render_profile::{PROFILE_NAME, resolve_profile};
use tuiscotti::sample::{Observation, Sample};
use tuiscotti::view::{ViewSettings, capture_view};

#[test]
fn independently_assembled_binary_preserves_full_state_values() {
    let bytes = valid_container();
    let sample = Sample::from_bytes(&bytes).expect("independent protobuf sample decodes");

    assert_eq!(sample.test_id(), TEST_ID);
    assert_eq!(sample.gate().as_str(), "state");
    assert_eq!(sample.observation().note(), Some(""));
    let snapshot = sample.snapshot();
    assert_eq!(snapshot.cols(), 2);
    assert_eq!(snapshot.rows(), 1);
    assert_eq!(snapshot.scope(), SnapshotScope::FullHistory);
    assert_eq!(snapshot.viewport_origin(), 0);
    assert_eq!(snapshot.title(), Some(""));
    assert_eq!(snapshot.palette().len(), 256);
    assert_eq!(snapshot.palette()[0].r, 0);
    assert_eq!(snapshot.palette()[255].g, 0);
    assert_eq!(snapshot.default_foreground().r, 1);
    assert_eq!(snapshot.default_background().g, 5);
    assert_eq!(snapshot.default_cursor().b, 9);

    let cell = snapshot.cell_at(0, 0).expect("first cell exists");
    assert_eq!(cell.text(), "e\u{301}");
    assert_eq!(cell.width(), 1);
    assert_eq!(cell.fg(), SnapshotColor::Rgb(0, 128, 255));
    assert_eq!(cell.bg(), SnapshotColor::Indexed(255));
    assert!(cell.has(tui_test::terminal::cell::Attrs::BOLD));
    assert!(!cell.has(tui_test::terminal::cell::Attrs::DIM));
    assert!(cell.has(tui_test::terminal::cell::Attrs::ITALIC));
    assert!(!cell.has(tui_test::terminal::cell::Attrs::INVERSE));
    assert!(cell.has(tui_test::terminal::cell::Attrs::INVISIBLE));
    assert!(!cell.has(tui_test::terminal::cell::Attrs::STRIKE));
    assert!(cell.has(tui_test::terminal::cell::Attrs::BLINK));
    assert_eq!(cell.underline(), SnapshotUnderline::Dashed);
    assert_eq!(cell.underline_color(), SnapshotColor::Indexed(0));
    assert_eq!(cell.link_uri(), Some(""));
    assert_eq!(cell.link_id(), Some("link-id"));

    let wide = snapshot.cell_at(1, 0).expect("clipped wide cell exists");
    assert_eq!(wide.text(), "🦀");
    assert_eq!(wide.width(), 2);
    assert_eq!(wide.fg(), SnapshotColor::Default);
    assert_eq!(wide.bg(), SnapshotColor::Rgb(16, 32, 48));

    let cursor = snapshot.cursor();
    assert_eq!(cursor.col(), 123);
    assert_eq!(cursor.row(), 456);
    assert!(!cursor.in_capture());
    assert!(!cursor.visible());
    assert_eq!(cursor.shape().name(), "bar");
    assert_eq!(cursor.color().r, 17);
    assert_eq!(cursor.blink(), Some(false));

    let metadata = snapshot.metadata();
    assert_eq!(metadata.captured_at_ms, u64::MAX);
    assert_eq!(metadata.revision, 0);
    assert_eq!(metadata.backend, "fixture-backend");
    assert_eq!(metadata.session, "");
    assert_eq!(metadata.source, SnapshotSource::Live);

    let encoded_again = sample.to_bytes().expect("sample encodes as binary");
    let decoded_again = Sample::from_bytes(&encoded_again).expect("written bytes decode");
    assert_eq!(decoded_again.snapshot().metadata(), metadata);
    assert_eq!(decoded_again.snapshot().cells(), snapshot.cells());
    assert_eq!(decoded_again.observation().note(), Some(""));

    let directory = tempfile::tempdir().expect("sample directory creates");
    let file_path = directory.path().join("sample.tui.pb");
    write(&file_path, &bytes);
    let from_file = Sample::from_file(&file_path).expect("binary sample loads from file");
    assert_eq!(from_file.snapshot().cells(), snapshot.cells());
    let saved_path = directory.path().join("saved.tui.pb");
    sample
        .save(&saved_path)
        .expect("binary sample saves to file");
    assert!(
        std::fs::read(saved_path)
            .expect("saved binary reads")
            .starts_with(b"TUISCPB\0")
    );
}

#[test]
fn pixel_sample_round_trip_keeps_contract_floats_png_bytes_and_digest() {
    let settings = ViewSettings::builtin();
    let snapshot = capture_view(12, 3, &settings, |frame| {
        frame.buffer_mut()[(0, 0)].set_char('X');
        frame.buffer_mut()[(0, 1)].set_char('y');
    })
    .expect("fixture view captures");
    let profile = resolve_profile(PROFILE_NAME).expect("pinned render profile resolves");
    let test_id = format!(
        "binary/pixels-{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let original = Sample::with_pixels(&test_id, &snapshot, Observation::noted(""), profile)
        .expect("pixel sample builds");
    let encoded = original.to_bytes().expect("pixel sample encodes");
    assert!(encoded.starts_with(b"TUISCPB\0"));

    let decoded = Sample::from_bytes(&encoded).expect("pixel sample decodes");
    assert_eq!(decoded.test_id(), original.test_id());
    assert_eq!(
        decoded.snapshot().metadata(),
        original.snapshot().metadata()
    );
    assert_eq!(decoded.snapshot().cells(), original.snapshot().cells());
    assert_eq!(
        original.state_digest().expect("original digest"),
        decoded.state_digest().expect("round-trip digest"),
    );
    assert_eq!(decoded.observation().note(), Some(""));

    assert_contract_fields(&original, &decoded);
    assert_image_fields(&original, &decoded);
}

#[test]
fn iccp_expansion_bomb_passes_real_accept_and_check_callers() {
    let settings = ViewSettings::builtin();
    let snapshot = capture_view(12, 3, &settings, |frame| {
        frame.buffer_mut()[(0, 0)].set_char('X');
    })
    .expect("fixture view captures");
    let profile = resolve_profile(PROFILE_NAME).expect("pinned render profile resolves");
    let test_id = format!(
        "binary/icc-bomb/pixels-{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let sample = Sample::with_pixels(&test_id, &snapshot, Observation::empty(), profile)
        .expect("pixel sample builds");
    let encoded = sample.to_bytes().expect("pixel sample encodes");
    let mut wire =
        WireSample::decode(uncontainer(&encoded).as_slice()).expect("sample protobuf decodes");
    let image = wire.image.as_mut().expect("pixel sample has PNG evidence");
    let mut png = image.png.take().expect("PNG bytes are stored");
    let profile_bytes = vec![0; 16 * 1024 * 1024];
    let mut compressor = ZlibEncoder::new(Vec::new(), Compression::fast());
    compressor
        .write_all(&profile_bytes)
        .expect("ICC profile compresses");
    let compressed_profile = compressor.finish().expect("ICC profile finishes");
    drop(profile_bytes);
    let mut payload = b"tuiscotti\0\0".to_vec();
    payload.extend_from_slice(&compressed_profile);
    let iccp = png_chunk(*b"iCCP", &payload);
    png.splice(33..33, iccp);
    image.png_sha256 = Some(fixtures::sha256(&png));
    image.png = Some(png);
    let bomb_sample = fixtures::container(&wire.encode_to_vec());

    let temporary = tempfile::tempdir().expect("temporary directory creates");
    let (baselines, artifacts) = dirs(temporary.path());
    let candidate = temporary.path().join("icc-bomb.tui.pb");
    write(&candidate, &bomb_sample);

    let accept = accept_args(&candidate, &baselines, &bomb_sample);
    let accept_refs: Vec<&str> = accept.iter().map(String::as_str).collect();
    let accepted = run_cli(&accept_refs, None);
    assert_eq!(
        accepted.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&accepted.stderr)
    );
    assert_eq!(result(&accepted)["status"], "pass");

    let checked = run_cli(
        &[
            "check",
            "--sample",
            path(&candidate),
            "--baselines",
            path(&baselines),
            "--artifacts",
            path(&artifacts),
            "--json",
        ],
        None,
    );
    assert_eq!(
        checked.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert_eq!(result(&checked)["status"], "pass");
}

fn png_chunk(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut chunk = Vec::new();
    chunk.extend_from_slice(
        &u32::try_from(payload.len())
            .expect("chunk length fits u32")
            .to_be_bytes(),
    );
    chunk.extend_from_slice(&kind);
    chunk.extend_from_slice(payload);
    let mut crc = !0_u32;
    for byte in &chunk[4..] {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0_u32.wrapping_sub(crc & 1));
        }
    }
    chunk.extend_from_slice(&(!crc).to_be_bytes());
    chunk
}

#[derive(Clone, PartialEq, prost::Message)]
struct WireSample {
    #[prost(string, optional, tag = "1")]
    schema: Option<String>,
    #[prost(string, optional, tag = "2")]
    test_id: Option<String>,
    #[prost(int32, optional, tag = "3")]
    gate: Option<i32>,
    #[prost(bytes = "vec", optional, tag = "4")]
    snapshot: Option<Vec<u8>>,
    #[prost(bytes = "vec", optional, tag = "5")]
    render_contract: Option<Vec<u8>>,
    #[prost(message, optional, tag = "6")]
    image: Option<WireImage>,
    #[prost(bytes = "vec", optional, tag = "7")]
    observation: Option<Vec<u8>>,
}

#[derive(Clone, PartialEq, prost::Message)]
struct WireImage {
    #[prost(int32, optional, tag = "1")]
    format: Option<i32>,
    #[prost(uint32, optional, tag = "2")]
    width: Option<u32>,
    #[prost(uint32, optional, tag = "3")]
    height: Option<u32>,
    #[prost(string, optional, tag = "4")]
    png_sha256: Option<String>,
    #[prost(string, optional, tag = "5")]
    rgba_sha256: Option<String>,
    #[prost(bytes = "vec", optional, tag = "6")]
    png: Option<Vec<u8>>,
}

fn assert_contract_fields(original: &Sample, decoded: &Sample) {
    let original_contract = original.render_contract().expect("render contract exists");
    let decoded_contract = decoded.render_contract().expect("decoded contract exists");
    assert_eq!(decoded_contract.profile(), original_contract.profile());
    assert_eq!(
        decoded_contract.contract_version(),
        original_contract.contract_version()
    );
    assert_eq!(
        decoded_contract.renderer_version(),
        original_contract.renderer_version()
    );
    assert_eq!(
        decoded_contract.raster_policy(),
        original_contract.raster_policy()
    );
    assert_eq!(decoded_contract.fonts(), original_contract.fonts());
    assert_eq!(
        decoded_contract.cell_width().to_bits(),
        original_contract.cell_width().to_bits()
    );
    assert_eq!(
        decoded_contract.cell_height().to_bits(),
        original_contract.cell_height().to_bits()
    );
    assert_eq!(
        decoded_contract.font_size().to_bits(),
        original_contract.font_size().to_bits()
    );
    assert_eq!(
        decoded_contract.baseline().to_bits(),
        original_contract.baseline().to_bits()
    );
    assert_eq!(
        decoded_contract.scale().to_bits(),
        original_contract.scale().to_bits()
    );
    assert_eq!(decoded_contract.title_bar(), original_contract.title_bar());
    assert_eq!(
        decoded_contract.palette_digest(),
        original_contract.palette_digest()
    );
    assert_eq!(
        decoded_contract.cursor_phase(),
        original_contract.cursor_phase()
    );
    assert_eq!(decoded_contract.blink(), original_contract.blink());
    assert_eq!(
        decoded_contract.platform_key(),
        original_contract.platform_key()
    );
}

fn assert_image_fields(original: &Sample, decoded: &Sample) {
    let original_image = original.image().expect("PNG evidence exists");
    let decoded_image = decoded.image().expect("decoded PNG evidence exists");
    assert_eq!(decoded_image.format(), original_image.format());
    assert_eq!(decoded_image.width(), original_image.width());
    assert_eq!(decoded_image.height(), original_image.height());
    assert_eq!(decoded_image.png_sha256(), original_image.png_sha256());
    assert_eq!(decoded_image.rgba_sha256(), original_image.rgba_sha256());
    assert_eq!(decoded_image.png_base64(), original_image.png_base64());
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(decoded_image.png_base64())
            .expect("stored PNG base64 decodes"),
        base64::engine::general_purpose::STANDARD
            .decode(original_image.png_base64())
            .expect("original PNG base64 decodes"),
    );
}

fn accept_args(candidate: &Path, baselines: &Path, bytes: &[u8]) -> Vec<String> {
    let candidate_text = path(candidate);
    let baselines_text = path(baselines);
    let digest = sha256(bytes);
    [
        "accept".to_string(),
        "--candidate".to_string(),
        candidate_text.to_string(),
        "--candidate-sha256".to_string(),
        digest,
        "--expect-baseline".to_string(),
        "missing".to_string(),
        "--baselines".to_string(),
        baselines_text.to_string(),
        "--json".to_string(),
    ]
    .into()
}

fn result(output: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).expect("CLI emits one JSON result")
}

fn path(value: &Path) -> &str {
    value.to_str().expect("temporary paths are UTF-8")
}

fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory).expect("directory reads") {
            let path = entry.expect("directory entry reads").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files
}

#[path = "compressed_binary/approval.rs"]
mod approval;
#[path = "compressed_binary/decoder.rs"]
mod decoder;
#[path = "compressed_binary/full_roundtrip.rs"]
mod full_roundtrip;
#[path = "compressed_binary/owned.rs"]
mod owned;
