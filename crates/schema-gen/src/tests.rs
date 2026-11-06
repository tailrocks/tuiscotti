use std::fmt::Write as _;
use std::{collections::BTreeMap, env, fs, path::PathBuf};

use prost::Message;
use prost_types::{
    DescriptorProto, FieldDescriptorProto, FileDescriptorProto, FileDescriptorSet,
    OneofDescriptorProto,
    field_descriptor_proto::{Label, Type},
};

use super::{
    DESC, POLICY, SCHEMA_DIR,
    codegen::{handler_type, sanitize},
    model::{Constraint, FieldPolicy, MessagePolicy, Policy},
    policy::parse_policy,
};

pub(super) fn rustc_preamble() -> &'static str {
    r"mod sample { #[derive(Debug)] pub enum SampleError { TooLarge { what: &'static str, bytes: u64, limit: u64 }, Allocation { site: &'static str, requested: u64, source: std::collections::TryReserveError }, UnsupportedContract { version: u32 }, InvalidSnapshot { detail: String }, DimensionMismatch { detail: String }, GateMismatch { detail: String } } }
#[derive(Clone, Copy, Debug)] struct WirePath { field: u8 }
impl WirePath { fn new() -> Self { Self { field: 0 } } fn field(&mut self, id: u8) -> bool { self.field = id; true } fn index(&mut self, _: u64) -> bool { true } fn tag(&mut self, _: u32) -> bool { true } fn pop_index(&mut self) {} fn pop_field(&mut self) {} }
#[derive(Debug)] enum BinaryFormatError { Wire { offset: usize, path: WirePath, detail: &'static str } }
#[derive(Debug)] enum DecodeError { Format(BinaryFormatError), Invariant { detail: &'static str }, Allocation { site: &'static str, requested: u64, source: std::collections::TryReserveError }, UnsupportedSchema(String), Domain(sample::SampleError), Resource { resource: &'static str, claimed: Option<u64>, limit: u64, source: Option<Box<()>> } }
struct DecodeBudget;
"
}

pub(super) fn fixtures() -> (FileDescriptorSet, Policy, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let schema = root.join(SCHEMA_DIR);
    let descriptor = fs::read(schema.join(DESC)).expect("descriptor fixture");
    let policy_text = fs::read_to_string(schema.join(POLICY)).expect("policy fixture");
    (
        FileDescriptorSet::decode(descriptor.as_slice()).expect("descriptor decode"),
        parse_policy(&policy_text).expect("policy parse"),
        policy_text,
    )
}

pub(super) fn recording_handler(policy: &Policy) -> String {
    let mut out = String::from(
        "struct Recorder { visits: usize, default_color: usize, indexed: usize, rgb: usize }\n",
    );
    for (message, definition) in &policy.messages {
        let trait_name = super::codegen::pascal(message);
        if definition.fields.is_empty() {
            let _ = writeln!(
                out,
                "impl generated::Decode{trait_name} for Recorder {{ type Error = DecodeError;"
            );
        } else {
            let _ = writeln!(
                out,
                "impl<'a> generated::Decode{trait_name}<'a> for Recorder {{ type Error = DecodeError;"
            );
        }
        for (field_name, field) in &definition.fields {
            let method = sanitize(&field.handler);
            let ty = handler_type(message, field_name, field, &policy.messages);
            let _ = writeln!(
                out,
                "fn {method}(&mut self, value: {ty}) -> Result<(), DecodeError> {{ self.visits += 1;"
            );
            if field.oneof.is_some() {
                if field.type_name.is_some() {
                    let child = field.type_name.as_deref().unwrap_or_default();
                    if policy.messages[child].fields.is_empty() {
                        let _ = writeln!(out, "generated::Validated{child}::visit(self);");
                    } else {
                        out.push_str("value.visit(self)?;\n");
                    }
                }
                let counter = match field.handler.as_str() {
                    "default_color" => "default_color",
                    "indexed" => "indexed",
                    "rgb" if message == "Color" => "rgb",
                    _ => "",
                };
                if !counter.is_empty() {
                    let _ = writeln!(out, "self.{counter} += 1;");
                }
            } else if field.kind == "message" {
                if field.cardinality == "repeated" {
                    out.push_str("for item in value { item?.visit(self)?; }\n");
                } else if field.presence == "optional" {
                    out.push_str("if let Some(item) = value { item.visit(self)?; }\n");
                } else {
                    out.push_str("value.visit(self)?;\n");
                }
            }
            out.push_str("Ok(()) }\n");
        }
        for constraint in &definition.constraints {
            if policy.constraints[constraint].phase == "mapping" {
                let method = super::codegen::snake(constraint);
                let _ = writeln!(
                    out,
                    "fn {method}(&mut self, _: generated::Validated{message}<'a>) -> Result<(), DecodeError> {{ self.visits += 1; Ok(()) }}"
                );
            }
        }
        out.push_str("}\n");
    }
    out
}

pub(super) fn recursive_policy_fixture() -> (FileDescriptorSet, Policy) {
    (recursive_descriptor(), recursive_node_policy())
}

fn recursive_descriptor() -> FileDescriptorSet {
    let value_field = FieldDescriptorProto {
        name: Some("value".into()),
        number: Some(1),
        label: Some(Label::Optional as i32),
        r#type: Some(Type::Uint32 as i32),
        oneof_index: Some(0),
        proto3_optional: Some(true),
        ..Default::default()
    };
    let child_field = FieldDescriptorProto {
        name: Some("child".into()),
        number: Some(2),
        label: Some(Label::Optional as i32),
        r#type: Some(Type::Message as i32),
        type_name: Some(".tuiscotti.private.v1.Node".into()),
        ..Default::default()
    };
    let node = DescriptorProto {
        name: Some("Node".into()),
        field: vec![value_field, child_field],
        oneof_decl: vec![OneofDescriptorProto {
            name: Some("_value".into()),
            ..Default::default()
        }],
        ..Default::default()
    };
    let file = FileDescriptorProto {
        name: Some("recursive-test.proto".into()),
        package: Some("tuiscotti.private.v1".into()),
        syntax: Some("proto3".into()),
        message_type: vec![node],
        ..Default::default()
    };
    FileDescriptorSet { file: vec![file] }
}

fn recursive_node_policy() -> Policy {
    let mut fields = BTreeMap::new();
    fields.insert(
        "value".into(),
        FieldPolicy {
            tag: 1,
            kind: "uint32".into(),
            cardinality: "singular".into(),
            presence: "optional".into(),
            oneof: None,
            type_name: None,
            handler: "value".into(),
            constraints: vec!["UINT32".into()],
        },
    );
    fields.insert(
        "child".into(),
        FieldPolicy {
            tag: 2,
            kind: "message".into(),
            cardinality: "singular".into(),
            presence: "optional".into(),
            oneof: None,
            type_name: Some("Node".into()),
            handler: "child".into(),
            constraints: vec![],
        },
    );
    let mut constraints = BTreeMap::new();
    constraints.insert(
        "UINT32".into(),
        Constraint {
            phase: "field".into(),
        },
    );
    let mut messages = BTreeMap::new();
    messages.insert(
        "Node".into(),
        MessagePolicy {
            fields,
            constraints: vec![],
        },
    );
    Policy {
        version: 1,
        package: "tuiscotti.private.v1".into(),
        constraints,
        messages,
        enums: BTreeMap::new(),
        retired: vec![],
    }
}

mod generated;
mod policy;
