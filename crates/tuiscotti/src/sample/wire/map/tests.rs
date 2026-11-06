use super::*;
use crate::sample::budget::Phase;
use crate::sample::embedded_png::png as embedded_png;
use crate::sample::wire::generated::scan_sample;
use crate::sample::{BinaryFormatError, Gate as DomainGate};
use base64::Engine as _;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::io;
use tui_test::SnapshotColor;
use tui_test::profile::Rgb;
use tui_test::terminal::cell::Attrs;

fn varint(mut value: u64, out: &mut Vec<u8>) {
    while value >= 0x80 {
        out.push(value.to_le_bytes()[0] | 0x80);
        value >>= 7;
    }
    out.push(value.to_le_bytes()[0]);
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
    varint(u64::try_from(value.len()).unwrap_or(u64::MAX), out);
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

mod binary;
fn pixel_wire() -> io::Result<(Vec<u8>, Vec<u8>)> {
    let png = png()?;
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
    Ok((out, png))
}

fn png() -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut encoder = embedded_png::Encoder::new(&mut bytes, 20, 20);
    encoder.set_color(embedded_png::ColorType::Rgba);
    encoder.set_depth(embedded_png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|error| io::Error::other(format!("{error:?}")))?;
    let rgba = vec![17; 20 * 20 * 4];
    writer
        .write_image_data(&rgba)
        .map_err(|error| io::Error::other(format!("{error:?}")))?;
    drop(writer);
    Ok(bytes)
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

mod mapping;

fn wire_path(
    result: Result<impl std::fmt::Debug, DecodeError>,
) -> Result<(usize, String), Box<dyn Error>> {
    match result {
        Err(DecodeError::Format(BinaryFormatError::Wire { offset, path, .. })) => {
            Ok((offset, path.to_string()))
        }
        Err(error) => Err(io::Error::other(format!("expected wire error, got {error:?}")).into()),
        Ok(_) => Err(io::Error::other("expected wire error, got success").into()),
    }
}

#[test]
fn production_wire_paths_cover_deep_scalar_missing_unknown_and_offsets()
-> Result<(), Box<dyn Error>> {
    let mut budget = DecodeBudget::new().map_err(|error| io::Error::other(format!("{error:?}")))?;

    let (offset, path) = wire_path(scan_sample(&[], &mut budget))?;
    assert_eq!(offset, 0);
    assert_eq!(path, "schema");

    let mut wire = Vec::new();
    bytes(4, &[], &mut wire);
    let (_, path) = wire_path(scan_sample(&wire, &mut budget))?;
    assert_eq!(path, "snapshot.schema");

    let mut rgb = Vec::new();
    let mut color = Vec::new();
    bytes(3, &rgb, &mut color);
    let mut cell = Vec::new();
    bytes(3, &color, &mut cell);
    let mut snapshot = Vec::new();
    bytes(6, &cell, &mut snapshot);
    let mut wire = Vec::new();
    bytes(4, &snapshot, &mut wire);
    let (_, path) = wire_path(scan_sample(&wire, &mut budget))?;
    assert_eq!(path, "snapshot.cells[0].fg.rgb.red");

    let wire = [0x80];
    let (offset, path) = wire_path(scan_sample(&wire, &mut budget))?;
    assert_eq!(offset, wire.len());
    assert_eq!(path, "");

    let truncated_cell = [0x0a, 0x05, b'x'];
    let mut snapshot = Vec::new();
    bytes(6, &truncated_cell, &mut snapshot);
    let mut wire = Vec::new();
    bytes(4, &snapshot, &mut wire);
    let payload_offset = wire.len() - 1;
    let (offset, path) = wire_path(scan_sample(&wire, &mut budget))?;
    assert_eq!(offset, payload_offset);
    assert_eq!(path, "snapshot.cells[0].text");

    varint(u64::from(99_u32) << 3, &mut rgb);
    color.clear();
    bytes(3, &rgb, &mut color);
    cell.clear();
    bytes(3, &color, &mut cell);
    snapshot.clear();
    bytes(6, &cell, &mut snapshot);
    wire.clear();
    bytes(4, &snapshot, &mut wire);
    let unknown_tag_offset = wire.len() - rgb.len();
    let (offset, path) = wire_path(scan_sample(&wire, &mut budget))?;
    assert_eq!(path, "snapshot.cells[0].fg.rgb.#99");
    assert_eq!(offset, unknown_tag_offset);

    rgb.clear();
    rgb.push(0x08);
    rgb.extend_from_slice(&[0x80; 9]);
    rgb.push(0x02);
    color.clear();
    bytes(3, &rgb, &mut color);
    cell.clear();
    bytes(3, &color, &mut cell);
    snapshot.clear();
    bytes(6, &cell, &mut snapshot);
    wire.clear();
    bytes(4, &snapshot, &mut wire);
    let rgb_start = wire.len() - rgb.len();
    let (offset, path) = wire_path(scan_sample(&wire, &mut budget))?;
    assert_eq!(path, "snapshot.cells[0].fg.rgb.red");
    assert_eq!(offset, rgb_start + 1);
    Ok(())
}
