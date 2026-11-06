use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
};

use super::{
    codegen::{ordered_fields, pascal, sanitize, snake},
    model::{FieldPolicy, MessagePolicy},
};

pub(super) fn render_writer(
    messages: &BTreeMap<String, MessagePolicy>,
) -> Result<(String, BTreeMap<String, String>), String> {
    let mut root = String::from(
        "// @generated canonical Protobuf writer traversal.\ninclude!(\"writer_common.rs\");\n",
    );
    let mut files = BTreeMap::new();
    files.insert("writer_common.rs".into(), writer_common().to_owned());
    for (message, policy) in messages {
        let mut body = format!(
            "// @generated canonical emitter for {message}.\npub(super) fn emit_{}<S: ProtoSink, E: Encode{}>(adapter: &E, sink: &mut S) -> Result<(), EmitError> {{\n    let _ = (adapter, &*sink);\n",
            snake(message),
            pascal(message)
        );
        if policy.fields.is_empty() {
            body.push_str("    sink.put(&[])?;\n");
        }
        let oneofs: BTreeSet<_> = policy
            .fields
            .values()
            .filter_map(|f| f.oneof.as_deref())
            .collect();
        for oneof in &oneofs {
            writeln!(body, "    let mut oneof_{} = 0u32;", snake(oneof))
                .map_err(|_| "failed to format generated source".to_owned())?;
        }
        let mut helpers = String::new();
        for (field, fp) in ordered_fields(policy) {
            let accessor = sanitize(field);
            if message == "Snapshot" && field.as_str() == "scope" {
                body.push_str("    emit_snapshot_scope(adapter, sink)?;\n");
                helpers.push_str("fn emit_snapshot_scope<S: ProtoSink, E: EncodeSnapshot>(adapter: &E, sink: &mut S) -> Result<(), EmitError> {\n    let value = adapter.scope();\n");
                emit_value(&mut helpers, fp, "value", fp.tag, 4)?;
                helpers.push_str("    Ok(())\n}\n");
            } else if let Some(union) = fp.oneof.as_deref() {
                writeln!(body, "    if let Some(value) = adapter.{accessor}() {{\n        oneof_{0} = oneof_{0}.checked_add(1).ok_or(EmitError::Invariant {{ detail: \"oneof count overflow\" }})?;", snake(union))
                    .map_err(|_| "failed to format generated source".to_owned())?;
                emit_value(&mut body, fp, "value", fp.tag, 8)?;
                body.push_str("    }\n");
            } else if fp.cardinality == "repeated" {
                writeln!(
                    body,
                    "    let values = adapter.{accessor}();\n    for value in values {{"
                )
                .map_err(|_| "failed to format generated source".to_owned())?;
                emit_value(&mut body, fp, "value", fp.tag, 8)?;
                body.push_str("    }\n");
            } else if fp.presence == "optional" {
                writeln!(body, "    if let Some(value) = adapter.{accessor}() {{")
                    .map_err(|_| "failed to format generated source".to_owned())?;
                emit_value(&mut body, fp, "value", fp.tag, 8)?;
                body.push_str("    }\n");
            } else {
                writeln!(body, "    let value = adapter.{accessor}();")
                    .map_err(|_| "failed to format generated source".to_owned())?;
                emit_value(&mut body, fp, "value", fp.tag, 4)?;
            }
        }
        for oneof in oneofs {
            writeln!(body, "    if oneof_{} != 1 {{ return Err(EmitError::Invariant {{ detail: \"oneof must select exactly one branch\" }}); }}", snake(oneof))
                .map_err(|_| "failed to format generated source".to_owned())?;
        }
        body.push_str("    Ok(())\n}\n");
        body.push_str(&helpers);
        let file = format!("emit_{}.rs", snake(message));
        files.insert(file.clone(), body);
        writeln!(root, "include!({file:?});")
            .map_err(|_| "failed to format generated source".to_owned())?;
    }
    root.push_str("pub(super) fn encoded_len<E: EncodeSample>(adapter: &E) -> Result<u64, EmitError> { let mut sink = CountSink::new(); emit_sample(adapter, &mut sink)?; Ok(sink.position()) }\n");
    root.push_str("pub(super) fn encode_into<S: ProtoSink, E: EncodeSample>(adapter: &E, sink: &mut S) -> Result<(), EmitError> { emit_sample(adapter, sink) }\n");
    Ok((root, files))
}

const WRITER_COMMON: &str = r#"// @generated sink primitives and canonical Protobuf wire operations.
use crate::sample::SampleError;

const MAX_ENCODED_BYTES: u64 = 64 * 1024 * 1024;

pub(super) trait ProtoSink {
    fn position(&self) -> u64;
    fn put(&mut self, bytes: &[u8]) -> Result<(), SampleError>;
}

#[derive(Debug)]
pub(super) enum EmitError {
    Sample(SampleError),
    Invariant { detail: &'static str },
}

impl From<SampleError> for EmitError {
    fn from(error: SampleError) -> Self { Self::Sample(error) }
}

pub(super) struct CountSink { position: u64 }

impl CountSink {
    pub(super) fn new() -> Self { Self { position: 0 } }
}

#[cfg(test)]
pub(super) struct VecSink { bytes: Vec<u8>, limit: usize }

#[cfg(test)]
impl VecSink {
    pub(super) fn new(limit: usize) -> Result<Self, SampleError> {
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(limit).map_err(|source| SampleError::Allocation {
            site: "generated writer test sink", requested: u64::try_from(limit).unwrap_or(u64::MAX), source,
        })?;
        Ok(Self { bytes, limit })
    }
    pub(super) fn finish(self) -> Vec<u8> { self.bytes }
}

#[cfg(test)]
impl ProtoSink for VecSink {
    fn position(&self) -> u64 { self.bytes.len() as u64 }
    fn put(&mut self, bytes: &[u8]) -> Result<(), SampleError> {
        let next = self.bytes.len().checked_add(bytes.len()).ok_or(SampleError::TooLarge {
            what: "generated writer test sink", bytes: u64::MAX, limit: self.limit as u64,
        })?;
        if next > self.limit { return Err(SampleError::TooLarge { what: "generated writer test sink", bytes: next as u64, limit: self.limit as u64 }); }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn vector_sink_preserves_written_bytes_and_position() -> Result<(), SampleError> {
    let mut sink = VecSink::new(2)?;
    sink.put(&[1, 2])?;
    assert_eq!(sink.position(), 2);
    assert_eq!(sink.finish(), [1, 2]);
    Ok(())
}

impl ProtoSink for CountSink {
    fn position(&self) -> u64 { self.position }
    fn put(&mut self, bytes: &[u8]) -> Result<(), SampleError> {
        let next = self.position.checked_add(bytes.len() as u64).ok_or(
            SampleError::TooLarge { what: "encoded sample", bytes: u64::MAX, limit: MAX_ENCODED_BYTES })?;
        if next > MAX_ENCODED_BYTES {
            return Err(SampleError::TooLarge { what: "encoded sample", bytes: next, limit: MAX_ENCODED_BYTES });
        }
        self.position = next;
        Ok(())
    }
}

pub(super) fn put_varint<S: ProtoSink>(sink: &mut S, mut value: u64) -> Result<(), EmitError> {
    let mut bytes = [0u8; 10];
    let mut used = 0;
    while value >= 0x80 { bytes[used] = value.to_le_bytes()[0] | 0x80; used += 1; value >>= 7; }
    bytes[used] = value.to_le_bytes()[0];
    sink.put(&bytes[..=used])?;
    Ok(())
}
pub(super) fn put_key<S: ProtoSink>(sink: &mut S, tag: u32, wire: u32) -> Result<(), EmitError> {
    let key = tag.checked_mul(8).and_then(|v| v.checked_add(wire)).ok_or(EmitError::Invariant { detail: "field key overflow" })?;
    put_varint(sink, u64::from(key))
}
pub(super) fn put_varint_field<S: ProtoSink>(sink: &mut S, tag: u32, value: u64) -> Result<(), EmitError> {
    put_key(sink, tag, 0)?; put_varint(sink, value)
}
pub(super) fn put_fixed32_field<S: ProtoSink>(sink: &mut S, tag: u32, value: u32) -> Result<(), EmitError> {
    put_key(sink, tag, 5)?; sink.put(&value.to_le_bytes())?; Ok(())
}
pub(super) fn put_fixed64_field<S: ProtoSink>(sink: &mut S, tag: u32, value: u64) -> Result<(), EmitError> {
    put_key(sink, tag, 1)?; sink.put(&value.to_le_bytes())?; Ok(())
}
pub(super) fn put_bytes_field<S: ProtoSink>(sink: &mut S, tag: u32, value: &[u8]) -> Result<(), EmitError> {
    put_key(sink, tag, 2)?; put_varint(sink, value.len() as u64)?; sink.put(value)?; Ok(())
}
pub(super) fn put_nested_prefix<S: ProtoSink>(sink: &mut S, tag: u32, len: u64) -> Result<(), EmitError> {
    put_key(sink, tag, 2)?; put_varint(sink, len)
}
fn verify_nested_length(start: u64, end: u64, expected: u64) -> Result<(), EmitError> {
    if end.checked_sub(start) != Some(expected) { return Err(EmitError::Invariant { detail: "nested encoded length changed between passes" }); }
    Ok(())
}
"#;

fn writer_common() -> &'static str {
    WRITER_COMMON
}

fn emit_value(
    out: &mut String,
    field: &FieldPolicy,
    value: &str,
    tag: i32,
    indent: usize,
) -> Result<(), String> {
    let pad = " ".repeat(indent);
    match field.kind.as_str() {
        "uint32" | "bool" => writeln!(
            out,
            "{pad}put_varint_field(sink, {tag}, u64::from({value}))?;"
        )
        .map_err(|_| "failed to format generated source".to_owned())?,
        "uint64" => write_source(
            out,
            format_args!("{pad}put_varint_field(sink, {tag}, {value})?;"),
        )?,
        "enum" => write_source(
            out,
            format_args!(
                "{pad}put_varint_field(sink, {tag}, u64::try_from({value}.to_wire()).map_err(|_| EmitError::Invariant {{ detail: \"negative enum value\" }})?)?;"
            ),
        )?,
        "float" => write_source(
            out,
            format_args!("{pad}put_fixed32_field(sink, {tag}, {value}.to_bits())?;"),
        )?,
        "double" => write_source(
            out,
            format_args!("{pad}put_fixed64_field(sink, {tag}, {value}.to_bits())?;"),
        )?,
        "string" => write_source(
            out,
            format_args!("{pad}put_bytes_field(sink, {tag}, {value}.as_bytes())?;"),
        )?,
        "bytes" => write_source(
            out,
            format_args!("{pad}put_bytes_field(sink, {tag}, {value})?;"),
        )?,
        "message" => {
            let child = field
                .type_name
                .as_deref()
                .ok_or_else(|| "message field lacks type name".to_owned())?;
            let emit = format!("emit_{}", snake(child));
            write_source(
                out,
                format_args!(
                    "{pad}let mut nested_count = CountSink::new();\n{pad}{emit}(&{value}, &mut nested_count)?;\n{pad}let nested_len = nested_count.position();\n{pad}put_nested_prefix(sink, {tag}, nested_len)?;\n{pad}let nested_start = sink.position();\n{pad}{emit}(&{value}, sink)?;\n{pad}verify_nested_length(nested_start, sink.position(), nested_len)?;"
                ),
            )?;
        }
        other => return Err(format!("unsupported writer field type {other}")),
    }
    Ok(())
}

fn write_source(out: &mut String, args: std::fmt::Arguments<'_>) -> Result<(), String> {
    out.write_fmt(args)
        .map_err(|_| "failed to format generated source".to_owned())
}
