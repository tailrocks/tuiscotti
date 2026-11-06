use super::*;

#[test]
fn binary_decoder_rejects_json_versions_bad_frames_and_malformed_wire() {
    let temp = tempfile::tempdir().expect("temporary directory creates");
    let (baselines, artifacts) = dirs(temp.path());
    for (name, bytes, expected_code) in rejected_samples() {
        let case = temp.path().join(format!("{name}.tui.pb"));
        write(&case, &bytes);
        let out = run_cli(
            &[
                "check",
                "--sample",
                path(&case),
                "--baselines",
                path(&baselines),
                "--artifacts",
                path(&artifacts),
                "--json",
            ],
            Some("true"),
        );
        let result = result(&out);
        assert_eq!(out.status.code(), Some(3), "case {name}");
        assert_eq!(result["error_code"], expected_code, "case {name}");
    }
    assert_eq!(walk_files(&baselines), [] as [PathBuf; 0]);
}

fn rejected_samples() -> Vec<(&'static str, Vec<u8>, &'static str)> {
    let valid = valid_container();
    let mut cases = container_rejections(&valid);
    cases.extend(wire_rejections());
    cases
}

fn container_rejections(valid: &[u8]) -> Vec<(&'static str, Vec<u8>, &'static str)> {
    let mut wrong_container = valid.to_vec();
    put_u16(header_mut(&mut wrong_container), 8, 2);
    let mut wrong_codec = valid.to_vec();
    put_u16(header_mut(&mut wrong_codec), 10, 0);
    let mut wrong_message = valid.to_vec();
    wrong_message[12..16].copy_from_slice(&2_u32.to_le_bytes());
    let mut oversized_output = valid.to_vec();
    oversized_output[16..24].copy_from_slice(&((64 * 1024 * 1024 + 1) as u64).to_le_bytes());
    let mut wrong_output_length = valid.to_vec();
    wrong_output_length[16..24].copy_from_slice(&1_u64.to_le_bytes());
    let mut wrong_payload_length = valid.to_vec();
    let payload_length = u64::try_from(valid.len() - 32).expect("fixture length fits");
    wrong_payload_length[24..32].copy_from_slice(&(payload_length + 1).to_le_bytes());
    let mut bad_frame = valid.to_vec();
    let last = bad_frame.len() - 1;
    bad_frame[last] ^= 0x01;
    let truncated_frame = valid[..valid.len() - 2].to_vec();
    let mut physical_trailing_bytes = valid.to_vec();
    physical_trailing_bytes.push(0);
    let mut trailing_frame = valid.to_vec();
    let extra_frame = container(&[0x00]);
    trailing_frame.extend_from_slice(&extra_frame[32..]);
    let compressed_len = u64::try_from(trailing_frame.len() - 32).expect("fixture length fits");
    trailing_frame[24..32].copy_from_slice(&compressed_len.to_le_bytes());

    vec![
        (
            "json",
            br#"{"schema":"tuiscotti.sample.v1"}"#.to_vec(),
            "INVALID_SAMPLE",
        ),
        ("short-magic", truncated_header(), "INVALID_SAMPLE"),
        ("container-version", wrong_container, "UNSUPPORTED_SCHEMA"),
        ("codec", wrong_codec, "UNSUPPORTED_SCHEMA"),
        ("message-version", wrong_message, "UNSUPPORTED_SCHEMA"),
        ("oversized-output", oversized_output, "RESOURCE_LIMIT"),
        ("wrong-output-length", wrong_output_length, "INVALID_SAMPLE"),
        (
            "wrong-payload-length",
            wrong_payload_length,
            "INVALID_SAMPLE",
        ),
        ("checksum", bad_frame, "INVALID_SAMPLE"),
        ("truncated-frame", truncated_frame, "INVALID_SAMPLE"),
        ("trailing-bytes", physical_trailing_bytes, "INVALID_SAMPLE"),
        ("concatenated-frame", trailing_frame, "INVALID_SAMPLE"),
        ("bad-magic", b"unknown-format".to_vec(), "INVALID_SAMPLE"),
    ]
}

fn wire_rejections() -> Vec<(&'static str, Vec<u8>, &'static str)> {
    let mut unknown = valid_message();
    fixtures::varint_field(99, 1, &mut unknown);
    let mut duplicate = valid_message();
    fixtures::length_field(2, b"binary/second", &mut duplicate);
    let mut truncated_wire = valid_message();
    truncated_wire.extend_from_slice(&[0x0a, 0x05, 0x41]);
    let unknown_gate = container(&message_with_gate_wire(&[0x18, 0x63]));
    let zero_gate = container(&message_with_gate_wire(&[0x18, 0x00]));
    let invalid_utf8 = container(&message_with_test_id(&[0xff]));
    let malformed_varint = container(&message_with_gate_wire(&[
        0x18, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
    ]));
    let logical_schema = container(&message_with_schema("tuiscotti.sample.v2"));
    vec![
        ("unknown-field", container(&unknown), "INVALID_SAMPLE"),
        (
            "duplicate-singular",
            container(&duplicate),
            "INVALID_SAMPLE",
        ),
        (
            "duplicate-union",
            container(&message_with_duplicate_union()),
            "INVALID_SAMPLE",
        ),
        (
            "truncated-wire",
            container(&truncated_wire),
            "INVALID_SAMPLE",
        ),
        ("unknown-enum", unknown_gate, "INVALID_SAMPLE"),
        ("zero-enum", zero_gate, "INVALID_SAMPLE"),
        ("invalid-utf8", invalid_utf8, "INVALID_SAMPLE"),
        ("malformed-varint", malformed_varint, "INVALID_SAMPLE"),
        ("logical-schema", logical_schema, "UNSUPPORTED_SCHEMA"),
    ]
}
