use std::fmt::Write as _;

use super::super::{
    codegen::{encode_scalar_type, handler_type, ordered_fields, pascal, sanitize, snake},
    model::{FieldPolicy, MessagePolicy, Policy},
};

pub(crate) fn render_handler_file(short: &str, message: &MessagePolicy, policy: &Policy) -> String {
    let trait_name = pascal(short);
    let empty_message = message.fields.is_empty();
    let mut body = if empty_message {
        format!(
            "// @generated required handlers for {short}.\npub(super) trait Decode{trait_name} {{ type Error: From<DecodeError>;\n"
        )
    } else {
        format!(
            "// @generated required handlers for {short}.\npub(super) trait Decode{trait_name}<'a> {{ type Error: From<DecodeError>;\n"
        )
    };
    append_decode_handlers(&mut body, short, message, policy);
    body.push_str("}\n");
    let _ = writeln!(body, "pub(super) trait Encode{trait_name} {{");
    append_encode_handlers(&mut body, message);
    body.push_str("}\n");
    body
}

fn append_decode_handlers(
    body: &mut String,
    short: &str,
    message: &MessagePolicy,
    policy: &Policy,
) {
    for (field, field_policy) in ordered_fields(message) {
        let ty = handler_type(short, field, field_policy, &policy.messages);
        let _ = writeln!(
            body,
            "    fn {}(&mut self, value: {ty}) -> Result<(), Self::Error>;",
            sanitize(&field_policy.handler)
        );
    }
    for constraint in &message.constraints {
        if policy.constraints[constraint].phase == "mapping" {
            let method = snake(constraint);
            let _ = writeln!(
                body,
                "    fn {method}(&mut self, value: Validated{short}<'a>) -> Result<(), Self::Error>;"
            );
        }
    }
}

fn append_encode_handlers(body: &mut String, message: &MessagePolicy) {
    for (field, field_policy) in ordered_fields(message) {
        let method = sanitize(field);
        let assoc = pascal(field);
        let base = encode_scalar_type(&field_policy.kind, field_policy.type_name.as_deref(), false);
        if field_policy.cardinality == "repeated" {
            append_repeated_encode_handler(body, &assoc, field_policy);
            let _ = writeln!(body, "    fn {method}(&self) -> Self::{assoc}<'_>;");
        } else if field_policy.kind == "message" {
            append_message_encode_handler(body, &method, &assoc, field_policy);
        } else {
            let ty = if field_policy.oneof.is_some() || field_policy.presence == "optional" {
                format!("Option<{base}>")
            } else {
                base
            };
            let _ = writeln!(body, "    fn {method}(&self) -> {ty};");
        }
    }
}

fn append_repeated_encode_handler(body: &mut String, assoc: &str, field: &FieldPolicy) {
    if field.kind == "message" {
        let child = field.type_name.as_deref().unwrap_or("Message");
        let _ = write!(
            body,
            "    type {assoc}<'a>: Iterator<Item = Self::{assoc}Item<'a>> where Self: 'a;\n    type {assoc}Item<'a>: Encode{child} where Self: 'a;\n"
        );
    } else {
        let item = encode_scalar_type(&field.kind, field.type_name.as_deref(), true);
        let _ = writeln!(
            body,
            "    type {assoc}<'a>: Iterator<Item = {item}> where Self: 'a;"
        );
    }
}

fn append_message_encode_handler(
    body: &mut String,
    method: &str,
    assoc: &str,
    field: &FieldPolicy,
) {
    let child = field.type_name.as_deref().unwrap_or("Message");
    let _ = writeln!(body, "    type {assoc}<'a>: Encode{child} where Self: 'a;");
    if field.presence == "optional" || field.oneof.is_some() {
        let _ = writeln!(body, "    fn {method}(&self) -> Option<Self::{assoc}<'_>>;");
    } else {
        let _ = writeln!(body, "    fn {method}(&self) -> Self::{assoc}<'_>;");
    }
}
