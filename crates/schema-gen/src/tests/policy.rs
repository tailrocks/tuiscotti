use std::{env, fmt::Write as _, fs, process::Command};

use super::super::{
    policy::{parse_policy, validate},
    scanner::render_scanner,
};
use super::{fixtures, recursive_policy_fixture};
use crate::model::FieldPolicy;

#[test]
fn policy_rejects_unknown_and_duplicate_keys() {
    let (_, _, text) = fixtures();
    let unknown = format!("{text}\nunknown_policy_key = true\n");
    assert!(parse_policy(&unknown).is_err());
    let duplicate = text.replacen("version = 1", "version = 1\nversion = 1", 1);
    assert!(parse_policy(&duplicate).is_err());
    let (_, policy, _) = fixtures();
    assert_eq!(
        super::super::policy::validate_production_scan_depth(&policy)
            .expect("acyclic production message graph"),
        5
    );
}

#[test]
fn policy_rejects_missing_and_extra_fields() -> Result<(), String> {
    let (descriptor, mut policy, _) = fixtures();
    policy
        .messages
        .get_mut("Sample")
        .ok_or("Sample policy missing")?
        .fields
        .remove("schema");
    assert!(validate(&descriptor, &policy).is_err());
    let (descriptor, mut policy, _) = fixtures();
    policy
        .messages
        .get_mut("Sample")
        .ok_or("Sample policy missing")?
        .fields
        .insert(
            "surprise".into(),
            FieldPolicy {
                tag: 99,
                kind: "string".into(),
                cardinality: "singular".into(),
                presence: "optional".into(),
                oneof: None,
                type_name: None,
                handler: "surprise".into(),
                constraints: vec!["UTF8".into()],
            },
        );
    assert!(validate(&descriptor, &policy).is_err());
    Ok(())
}

#[test]
fn policy_rejects_omitted_image_pixel_grid_constraint() -> Result<(), String> {
    let (descriptor, mut policy, _) = fixtures();
    let constraint = "IMAGE_PIXEL_DIMS_MATCH_GRID_10X20";
    policy
        .messages
        .get_mut("Image")
        .ok_or("Image policy missing")?
        .constraints
        .retain(|id| id != constraint);
    policy.constraints.remove(constraint);
    assert!(validate(&descriptor, &policy).is_err());
    Ok(())
}

#[test]
fn recursive_test_descriptor_uses_the_production_32_frame_scanner() {
    let (descriptor, policy) = recursive_policy_fixture();
    validate(&descriptor, &policy).expect("recursive descriptor/policy fixture");
    assert!(super::super::policy::validate_production_scan_depth(&policy).is_err());
    let temp = env::temp_dir().join(format!("schema-gen-depth-{}", std::process::id()));
    fs::create_dir_all(&temp).expect("create scanner fixture directory");
    for (name, bytes) in render_scanner(&policy).expect("render recursive scanner") {
        fs::write(temp.join(name), bytes).expect("write recursive scanner shard");
    }
    let generated = temp
        .join("scanner.rs")
        .canonicalize()
        .expect("recursive scanner");
    let source_path = temp.join("probe.rs");
    let output = temp.join("probe");
    let mut source = r#"
#[derive(Clone, Copy, Debug)] struct WirePath;
impl WirePath { fn new() -> Self { Self } fn field(&mut self, _: u8) -> bool { true } fn index(&mut self, _: u64) -> bool { true } fn tag(&mut self, _: u32) -> bool { true } fn pop_index(&mut self) {} fn pop_field(&mut self) {} }
#[derive(Debug)] enum BinaryFormatError { Wire { offset: usize, path: WirePath, detail: &'static str } }
mod sample { #[derive(Debug)] pub enum SampleError { UnsupportedContract { version: u32 }, InvalidSnapshot { detail: String }, DimensionMismatch { detail: String }, GateMismatch { detail: String } } }
#[derive(Debug)] enum DecodeError { Format(BinaryFormatError), Invariant { detail: &'static str }, Allocation { site: &'static str, requested: u64, source: std::collections::TryReserveError }, UnsupportedSchema(String), Domain(sample::SampleError), Resource { resource: &'static str, claimed: Option<u64>, limit: u64, source: Option<Box<()>> } }
struct DecodeBudget;
mod generated {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)] enum WireKind { Varint, Fixed64, LengthDelimited, Fixed32 }
    #[derive(Clone, Copy, Debug)] struct FieldSpec { id: u8, message: &'static str, name: &'static str, tag: u32, kind: WireKind, repeated: bool, required: bool, optional: bool, type_name: Option<&'static str>, handler: &'static str, constraints: &'static [&'static str], oneof: Option<&'static str> }
    const FIELDS_NODE: &[FieldSpec] = &[
        FieldSpec { id: 0, message: "Node", name: "value", tag: 1, kind: WireKind::Varint, repeated: false, required: false, optional: true, type_name: None, handler: "value", constraints: &["UINT32"], oneof: None },
        FieldSpec { id: 1, message: "Node", name: "child", tag: 2, kind: WireKind::LengthDelimited, repeated: false, required: false, optional: true, type_name: Some("Node"), handler: "child", constraints: &[], oneof: None },
    ];
"#.to_owned();
    let _ = writeln!(source, "    include!({:?});", generated.to_string_lossy());
    source.push_str(r#"
    #[derive(Clone, Copy, Debug)] struct ValidatedSample<'a> { span: Span<'a>, summary: ScanSummary }
    pub(super) fn check(bytes: &[u8]) -> bool { scan(Span { bytes, start: 0, end: bytes.len() }, "Node").is_ok() }
}
fn varint(mut value: usize) -> Vec<u8> { let mut out = Vec::new(); while value >= 128 { out.push((value as u8) | 128); value >>= 7; } out.push(value as u8); out }
fn chain(depth: usize) -> Vec<u8> { let mut node = vec![0x08, 1]; for _ in 1..depth { let mut parent = vec![0x08, 1, 0x12]; parent.extend(varint(node.len())); parent.extend(node); node = parent; } node }
fn main() { assert!(generated::check(&chain(31))); assert!(generated::check(&chain(32))); assert!(!generated::check(&chain(33))); }
"#);
    fs::write(&source_path, source).expect("write scanner depth probe");
    let result = Command::new("rustc")
        .args(["--edition=2024", "--crate-name=schema_gen_depth"])
        .arg(&source_path)
        .arg("-o")
        .arg(&output)
        .output()
        .expect("run scanner depth compile");
    assert!(
        result.status.success(),
        "recursive scanner failed to compile: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let run = Command::new(&output)
        .status()
        .expect("run depth boundary probe");
    fs::remove_dir_all(&temp).expect("remove scanner fixture directory");
    assert!(run.success(), "32-frame boundary is incorrect");
}
