#![expect(
    clippy::expect_used,
    reason = "fixture setup failures need the local assertion"
)]

use std::path::Path;
use std::process::{Command, Output, Stdio};

use tuiscotti::sample::SAMPLE_SCHEMA;

const MAGIC: &[u8; 8] = b"TUISCPB\0";
const HEADER_BYTES: usize = 32;

pub(crate) const TEST_ID: &str = "binary/full-values";

pub(crate) fn run_cli(args: &[&str], ci: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tuiscotti"));
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.env_remove("CI");
    if let Some(value) = ci {
        command.env("CI", value);
    }
    command.output().expect("tuiscotti child process starts")
}

pub(crate) fn container(message: &[u8]) -> Vec<u8> {
    let mut compressed = vec![0; zstd_safe::compress_bound(message.len())];
    let mut context = zstd_safe::CCtx::try_create().expect("test Zstandard context allocates");
    context
        .set_parameter(zstd_safe::CParameter::ChecksumFlag(true))
        .expect("test frame enables content checksum");
    let compressed_len = context
        .compress2(&mut compressed, message)
        .expect("test protobuf compresses");
    compressed.truncate(compressed_len);

    let mut result = vec![0; HEADER_BYTES];
    result[..8].copy_from_slice(MAGIC);
    result[8..10].copy_from_slice(&1_u16.to_le_bytes());
    result[10..12].copy_from_slice(&1_u16.to_le_bytes());
    result[12..16].copy_from_slice(&1_u32.to_le_bytes());
    let message_len = u64::try_from(message.len()).expect("test message length fits u64");
    let compressed_len = u64::try_from(compressed.len()).expect("test compressed length fits u64");
    result[16..24].copy_from_slice(&message_len.to_le_bytes());
    result[24..32].copy_from_slice(&compressed_len.to_le_bytes());
    result.extend_from_slice(&compressed);
    result
}

pub(crate) fn uncontainer(bytes: &[u8]) -> Vec<u8> {
    assert!(bytes.len() >= HEADER_BYTES, "test sample has a full header");
    let length = usize::try_from(u64::from_le_bytes(
        bytes[16..24]
            .try_into()
            .expect("header contains expanded size"),
    ))
    .expect("expanded test sample fits usize");
    let mut message = vec![0; length];
    let written = zstd_safe::decompress(&mut message, &bytes[HEADER_BYTES..])
        .expect("test sample frame decompresses");
    assert_eq!(written, length, "frame size matches container header");
    message
}

pub(crate) fn valid_container() -> Vec<u8> {
    container(&valid_message())
}

pub(crate) fn valid_message() -> Vec<u8> {
    valid_message_with_text("e\u{301}")
}

pub(crate) fn valid_message_with_text(text: &str) -> Vec<u8> {
    let mut sample = Vec::new();
    string(1, SAMPLE_SCHEMA, &mut sample);
    string(2, TEST_ID, &mut sample);
    uint(3, 1, &mut sample);
    message(4, &snapshot_message(text), &mut sample);
    message(7, &observation_message(), &mut sample);
    sample
}

pub(crate) fn valid_noncanonical_message() -> Vec<u8> {
    let mut sample = Vec::new();
    message(7, &observation_message(), &mut sample);
    message(4, &snapshot_message("e\u{301}"), &mut sample);
    sample.extend_from_slice(&[0x18, 0x81, 0x00]);
    string(2, TEST_ID, &mut sample);
    string(1, SAMPLE_SCHEMA, &mut sample);
    sample
}

pub(crate) fn message_with_gate_wire(gate: &[u8]) -> Vec<u8> {
    let mut sample = Vec::new();
    string(1, SAMPLE_SCHEMA, &mut sample);
    string(2, TEST_ID, &mut sample);
    sample.extend_from_slice(gate);
    message(4, &snapshot_message("e\u{301}"), &mut sample);
    message(7, &observation_message(), &mut sample);
    sample
}

pub(crate) fn message_with_test_id(test_id: &[u8]) -> Vec<u8> {
    let mut sample = Vec::new();
    string(1, SAMPLE_SCHEMA, &mut sample);
    length_field(2, test_id, &mut sample);
    uint(3, 1, &mut sample);
    message(4, &snapshot_message("e\u{301}"), &mut sample);
    message(7, &observation_message(), &mut sample);
    sample
}

pub(crate) fn message_with_schema(schema: &str) -> Vec<u8> {
    let mut sample = Vec::new();
    string(1, schema, &mut sample);
    string(2, TEST_ID, &mut sample);
    uint(3, 1, &mut sample);
    message(4, &snapshot_message("e\u{301}"), &mut sample);
    message(7, &observation_message(), &mut sample);
    sample
}

pub(crate) fn message_with_duplicate_union() -> Vec<u8> {
    let mut sample = Vec::new();
    string(1, SAMPLE_SCHEMA, &mut sample);
    string(2, TEST_ID, &mut sample);
    uint(3, 1, &mut sample);
    message(4, &snapshot_message_with_duplicate_union(), &mut sample);
    message(7, &observation_message(), &mut sample);
    sample
}

fn snapshot_message(text: &str) -> Vec<u8> {
    snapshot_message_inner(text, false)
}

fn snapshot_message_with_duplicate_union() -> Vec<u8> {
    snapshot_message_inner("e\u{301}", true)
}

fn snapshot_message_inner(text: &str, duplicate_union: bool) -> Vec<u8> {
    let mut snapshot = Vec::new();
    uint(1, 1, &mut snapshot);
    uint(2, 2, &mut snapshot);
    uint(3, 1, &mut snapshot);
    uint(4, 3, &mut snapshot);
    uint(5, 0, &mut snapshot);
    if duplicate_union {
        message(6, &duplicate_union_cell(), &mut snapshot);
    } else {
        message(6, &styled_cell(text), &mut snapshot);
    }
    message(6, &wide_cell(), &mut snapshot);
    for index in 0..256_u32 {
        message(
            7,
            &rgb(
                u8::try_from(index).expect("palette index fits u8"),
                u8::try_from(255 - index).expect("palette index fits u8"),
                u8::try_from(index / 2).expect("palette index fits u8"),
            ),
            &mut snapshot,
        );
    }
    message(8, &rgb(1, 2, 3), &mut snapshot);
    message(9, &rgb(4, 5, 6), &mut snapshot);
    message(10, &rgb(7, 8, 9), &mut snapshot);
    message(11, &cursor_message(), &mut snapshot);
    string(12, "", &mut snapshot);
    message(13, &metadata_message(), &mut snapshot);
    snapshot
}

fn duplicate_union_cell() -> Vec<u8> {
    let mut cell = Vec::new();
    string(1, "e\u{301}", &mut cell);
    uint(2, 1, &mut cell);
    let mut color = indexed_color(255);
    message(3, &rgb(0, 128, 255), &mut color);
    message(3, &color, &mut cell);
    message(4, &indexed_color(255), &mut cell);
    for (tag, value) in [
        (5, true),
        (6, false),
        (7, true),
        (8, false),
        (9, true),
        (10, false),
        (11, true),
    ] {
        bool_value(tag, value, &mut cell);
    }
    uint(12, 6, &mut cell);
    message(13, &indexed_color(0), &mut cell);
    string(14, "", &mut cell);
    string(15, "link-id", &mut cell);
    cell
}

fn styled_cell(text: &str) -> Vec<u8> {
    let mut cell = Vec::new();
    string(1, text, &mut cell);
    uint(2, 1, &mut cell);
    message(3, &rgb_color(0, 128, 255), &mut cell);
    message(4, &indexed_color(255), &mut cell);
    bool_value(5, true, &mut cell);
    bool_value(6, false, &mut cell);
    bool_value(7, true, &mut cell);
    bool_value(8, false, &mut cell);
    bool_value(9, true, &mut cell);
    bool_value(10, false, &mut cell);
    bool_value(11, true, &mut cell);
    uint(12, 6, &mut cell);
    message(13, &indexed_color(0), &mut cell);
    string(14, "", &mut cell);
    string(15, "link-id", &mut cell);
    cell
}

fn wide_cell() -> Vec<u8> {
    let mut cell = Vec::new();
    string(1, "🦀", &mut cell);
    uint(2, 2, &mut cell);
    message(3, &default_color(), &mut cell);
    message(4, &rgb_color(16, 32, 48), &mut cell);
    for tag in 5..=11 {
        bool_value(tag, false, &mut cell);
    }
    uint(12, 1, &mut cell);
    message(13, &default_color(), &mut cell);
    cell
}

fn cursor_message() -> Vec<u8> {
    let mut cursor = Vec::new();
    uint(1, 123, &mut cursor);
    uint(2, 456, &mut cursor);
    bool_value(3, false, &mut cursor);
    bool_value(4, false, &mut cursor);
    uint(5, 3, &mut cursor);
    message(6, &rgb(17, 34, 51), &mut cursor);
    bool_value(7, false, &mut cursor);
    cursor
}

fn metadata_message() -> Vec<u8> {
    let mut metadata = Vec::new();
    uint(1, u64::MAX, &mut metadata);
    uint(2, 0, &mut metadata);
    string(3, "fixture-backend", &mut metadata);
    string(4, "", &mut metadata);
    uint(5, 1, &mut metadata);
    metadata
}

fn observation_message() -> Vec<u8> {
    let mut observation = Vec::new();
    string(1, "", &mut observation);
    observation
}

fn rgb(red: u8, green: u8, blue: u8) -> Vec<u8> {
    let mut value = Vec::new();
    uint(1, u64::from(red), &mut value);
    uint(2, u64::from(green), &mut value);
    uint(3, u64::from(blue), &mut value);
    value
}

fn rgb_color(red: u8, green: u8, blue: u8) -> Vec<u8> {
    let mut value = Vec::new();
    message(3, &rgb(red, green, blue), &mut value);
    value
}

fn indexed_color(index: u8) -> Vec<u8> {
    let mut value = Vec::new();
    uint(2, u64::from(index), &mut value);
    value
}

fn default_color() -> Vec<u8> {
    let mut value = Vec::new();
    message(1, &[], &mut value);
    value
}

pub(crate) fn varint_field(tag: u32, value: u64, output: &mut Vec<u8>) {
    varint(u64::from(tag) << 3, output);
    varint(value, output);
}

pub(crate) fn length_field(tag: u32, value: &[u8], output: &mut Vec<u8>) {
    varint((u64::from(tag) << 3) | 2, output);
    let length = u64::try_from(value.len()).expect("test field length fits u64");
    varint(length, output);
    output.extend_from_slice(value);
}

fn uint(tag: u32, value: u64, output: &mut Vec<u8>) {
    varint_field(tag, value, output);
}

fn bool_value(tag: u32, value: bool, output: &mut Vec<u8>) {
    uint(tag, u64::from(value), output);
}

fn string(tag: u32, value: &str, output: &mut Vec<u8>) {
    length_field(tag, value.as_bytes(), output);
}

fn message(tag: u32, value: &[u8], output: &mut Vec<u8>) {
    length_field(tag, value, output);
}

fn varint(mut value: u64, output: &mut Vec<u8>) {
    while value >= 0x80 {
        output.push(u8::try_from(value & 0x7f).expect("varint byte fits u8") | 0x80);
        value >>= 7;
    }
    output.push(u8::try_from(value).expect("varint byte fits u8"));
}

pub(crate) fn write(path: &Path, bytes: &[u8]) {
    std::fs::write(path, bytes).expect("fixture bytes write");
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest as _;

    let digest = sha2::Sha256::digest(bytes);
    let mut text = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut text, "{byte:02x}").expect("String formatting is infallible");
    }
    text
}

pub(crate) fn dirs(root: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let baselines = root.join("baselines");
    let artifacts = root.join("artifacts");
    std::fs::create_dir_all(&baselines).expect("baseline directory created");
    std::fs::create_dir_all(&artifacts).expect("artifact directory created");
    (baselines, artifacts)
}

pub(crate) fn header_mut(bytes: &mut [u8]) -> &mut [u8] {
    &mut bytes[..HEADER_BYTES]
}

pub(crate) fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn truncated_header() -> Vec<u8> {
    MAGIC[..6].to_vec()
}

pub(crate) fn message_with_png(png: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut image = Vec::new();
    uint(1, 1, &mut image);
    uint(2, u64::from(width), &mut image);
    uint(3, u64::from(height), &mut image);
    string(4, &sha256(png), &mut image);
    string(5, &"0".repeat(64), &mut image);
    length_field(6, png, &mut image);
    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    let mut contract = Vec::new();
    string(1, "profile", &mut contract);
    uint(2, 1, &mut contract);
    string(3, "renderer", &mut contract);
    string(4, "pinned-v1", &mut contract);
    for (tag, value) in [(6_u32, 0.0_f32), (7, 20.0), (8, 12.5), (9, 15.0)] {
        varint(u64::from(tag) << 3 | 5, &mut contract);
        contract.extend_from_slice(&value.to_bits().to_le_bytes());
    }
    varint(10 << 3 | 1, &mut contract);
    contract.extend_from_slice(&1.5_f64.to_bits().to_le_bytes());
    uint(11, 1, &mut contract);
    string(12, &"a".repeat(64), &mut contract);
    uint(13, 2, &mut contract);
    uint(14, 0, &mut contract);
    string(15, &platform, &mut contract);
    let mut sample = Vec::new();
    string(1, SAMPLE_SCHEMA, &mut sample);
    string(2, &format!("binary/pixels-{platform}"), &mut sample);
    uint(3, 2, &mut sample);
    message(4, &snapshot_message("x"), &mut sample);
    message(5, &contract, &mut sample);
    message(6, &image, &mut sample);
    message(7, &observation_message(), &mut sample);
    sample
}
