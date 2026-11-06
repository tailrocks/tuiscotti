use std::collections::BTreeMap;
use std::fmt::Write as _;

use super::{
    codegen::{format_rust, ordered_fields, pascal, sanitize, snake},
    model::{FieldPolicy, Policy},
};

pub(super) fn render_accessors(policy: &Policy) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut output = BTreeMap::new();
    let mut root = String::from("include!(\"accessors_core.rs\");\n");
    output.insert(
        "accessors_core.rs".into(),
        format_rust(ACCESSOR_CORE.as_bytes())?,
    );
    for (message, definition) in &policy.messages {
        let mut body = format!(
            "// @generated typed borrowed accessors for {message}.\nimpl<'a> Validated{message}<'a> {{\n"
        );
        let empty_message = definition.fields.is_empty();
        if empty_message {
            body = format!(
                "// @generated typed borrowed accessors for empty message {message}.\nimpl Validated{message} {{\n"
            );
        }
        let mut iterators = String::new();
        for (field_name, field) in ordered_fields(definition) {
            if field.cardinality == "repeated" {
                if field.kind != "message" {
                    return Err(format!(
                        "repeated scalar accessor is not reviewed: {message}.{field_name}"
                    ));
                }
                render_repeated(&mut body, &mut iterators, message, field_name, field)?;
            } else {
                render_singular(&mut body, field_name, field, &policy.messages)?;
            }
        }
        render_visitor(&mut body, policy, message, definition, empty_message);
        body.push_str("}\n");
        body.push_str(&iterators);
        let file = format!("accessors_{}.rs", snake(message));
        output.insert(file.clone(), format_rust(body.as_bytes())?);
        let _ = writeln!(root, "include!({file:?});");
    }
    let mut union = String::from(
        "// @generated closed oneof view for Color.\npub(super) enum ValidatedColorValue<'a> {\n    DefaultColor(ValidatedDefaultColor),\n    Indexed(u32),\n    Rgb(ValidatedRgb<'a>),\n}\nimpl<'a> ValidatedColor<'a> { pub(super) fn value(&self) -> Result<ValidatedColorValue<'a>, DecodeError> {\n",
    );
    union.push_str("    if let Some(value) = self.default_color()? { return Ok(ValidatedColorValue::DefaultColor(value)); }\n    if let Some(value) = self.indexed()? { return Ok(ValidatedColorValue::Indexed(value)); }\n    if let Some(value) = self.rgb()? { return Ok(ValidatedColorValue::Rgb(value)); }\n    Err(DecodeError::Invariant { detail: \"validated color has no selected branch\" })\n} }\n");
    output.insert(
        "accessors_color_union.rs".into(),
        format_rust(union.as_bytes())?,
    );
    root.push_str("include!(\"accessors_color_union.rs\");\n");
    output.insert("accessors.rs".into(), format_rust(root.as_bytes())?);
    Ok(output)
}

pub(super) fn render_visitor(
    out: &mut String,
    policy: &Policy,
    message: &str,
    definition: &super::model::MessagePolicy,
    empty_message: bool,
) {
    let trait_name = pascal(message);
    if empty_message {
        let _ = writeln!(
            out,
            "    pub(super) fn visit<H: Decode{trait_name}>(_handler: &mut H) {{}}"
        );
        return;
    }
    let _ = writeln!(
        out,
        "    pub(super) fn visit<H: Decode{trait_name}<'a>>(&self, handler: &mut H) -> Result<(), H::Error> {{",
    );
    let oneofs: std::collections::BTreeSet<_> = definition
        .fields
        .values()
        .filter_map(|field| field.oneof.as_deref())
        .collect();
    for oneof in &oneofs {
        let union = sanitize(oneof);
        let union_type = format!("Validated{message}Value");
        let _ = writeln!(
            out,
            "        match self.{union}().map_err(H::Error::from)? {{"
        );
        for (field_name, field) in definition
            .fields
            .iter()
            .filter(|(_, field)| field.oneof.as_deref() == Some(oneof))
        {
            let variant = pascal(field_name);
            let _ = writeln!(
                out,
                "            {union_type}::{}(value) => handler.{}(value)?,",
                variant,
                sanitize(&field.handler)
            );
        }
        out.push_str("        }\n");
    }
    for (field_name, field) in ordered_fields(definition) {
        if field.oneof.is_some() {
            continue;
        }
        let method = sanitize(field_name);
        let handler = sanitize(&field.handler);
        if field.cardinality == "repeated" {
            let _ = writeln!(out, "        handler.{handler}(self.{method}())?;");
        } else {
            let _ = writeln!(
                out,
                "        handler.{handler}(self.{method}().map_err(H::Error::from)?)?;"
            );
        }
    }
    for constraint in &definition.constraints {
        if policy.constraints[constraint].phase == "mapping" {
            let method = snake(constraint);
            let _ = writeln!(out, "        handler.{method}(*self)?;");
        }
    }
    out.push_str("        Ok(())\n    }\n");
}

fn render_singular(
    out: &mut String,
    field_name: &str,
    field: &FieldPolicy,
    messages: &BTreeMap<String, super::model::MessagePolicy>,
) -> Result<(), String> {
    let method = sanitize(field_name);
    let base = scalar_type(field, messages)?;
    let optional = field.presence == "optional" || field.oneof.is_some();
    let result_type = if optional {
        format!("Option<{base}>")
    } else {
        base.clone()
    };
    let _ = writeln!(
        out,
        "    pub(super) fn {method}(&self) -> Result<{result_type}, DecodeError> {{"
    );
    let _ = writeln!(
        out,
        "        let raw = find_raw(self.span, {}u32)?;",
        field.tag
    );
    if optional {
        out.push_str("        let Some(raw) = raw else { return Ok(None); };\n");
    } else {
        out.push_str("        let raw = raw.ok_or(DecodeError::Invariant { detail: \"required field missing after scan\" })?;\n");
    }
    let value = decode_raw(field, messages)?;
    if optional {
        let _ = write!(out, "        Ok(Some({value}))\n    }}\n");
    } else {
        let _ = write!(out, "        Ok({value})\n    }}\n");
    }
    Ok(())
}

fn render_repeated(
    out: &mut String,
    iterators: &mut String,
    message: &str,
    field_name: &str,
    field: &FieldPolicy,
) -> Result<(), String> {
    let method = sanitize(field_name);
    let iterator = format!("{}{}Iter", pascal(message), pascal(field_name));
    let child = field
        .type_name
        .as_deref()
        .ok_or_else(|| format!("repeated message lacks child kind: {message}.{field_name}"))?;
    let _ = writeln!(
        out,
        "    pub(super) fn {method}(&self) -> {iterator}<'a> {{ {iterator} {{ span: self.span, cursor: self.span.start, finished: false }} }}"
    );
    let _ = write!(
        iterators,
        "pub(super) struct {iterator}<'a> {{ span: Span<'a>, cursor: usize, finished: bool }}\nimpl<'a> Iterator for {iterator}<'a> {{ type Item = Result<Validated{child}<'a>, DecodeError>; fn next(&mut self) -> Option<Self::Item> {{\n"
    );
    let _ = write!(
        iterators,
        "    if self.finished {{ return None; }}\n    loop {{ match next_raw(self.span, &mut self.cursor) {{\n        Ok(None) => {{ self.finished = true; return None; }},\n        Ok(Some(({}, raw))) => {{\n            let RawField::Bytes(span) = raw else {{ self.finished = true; return Some(Err(DecodeError::Invariant {{ detail: \"repeated message changed after scan\" }})); }};\n            return Some(Ok(Validated{child} {{ span }}));\n        }},\n        Ok(Some(_)) => {{}},\n        Err(error) => {{ self.finished = true; return Some(Err(error)); }}\n    }} }}\n}} }}\n",
        field.tag
    );
    Ok(())
}

fn scalar_type(
    field: &FieldPolicy,
    messages: &BTreeMap<String, super::model::MessagePolicy>,
) -> Result<String, String> {
    let ty = match field.kind.as_str() {
        "uint32" => "u32".to_owned(),
        "uint64" => "u64".to_owned(),
        "bool" => "bool".to_owned(),
        "float" => "f32".to_owned(),
        "double" => "f64".to_owned(),
        "string" => "&'a str".to_owned(),
        "bytes" => "&'a [u8]".to_owned(),
        "enum" => field.type_name.clone().ok_or("enum lacks type name")?,
        "message" => {
            let child = field
                .type_name
                .as_deref()
                .ok_or("message lacks type name")?;
            if messages[child].fields.is_empty() {
                format!("Validated{child}")
            } else {
                format!("Validated{child}<'a>")
            }
        }
        kind => return Err(format!("unsupported accessor type {kind}")),
    };
    Ok(ty)
}

fn decode_raw(
    field: &FieldPolicy,
    messages: &BTreeMap<String, super::model::MessagePolicy>,
) -> Result<String, String> {
    let expression = match field.kind.as_str() {
        "uint32" => "u32::try_from(value).map_err(|_| DecodeError::Invariant { detail: \"validated uint32 changed\" })?".to_owned(),
        "uint64" => "value".to_owned(), "bool" => "match value { 0 => false, 1 => true, _ => return Err(DecodeError::Invariant { detail: \"validated bool changed\" }) }".to_owned(),
        "float" => "f32::from_bits(value)".to_owned(), "double" => "f64::from_bits(value)".to_owned(),
        "enum" => format!("{}::from_wire(i32::try_from(value).map_err(|_| DecodeError::Invariant {{ detail: \"validated enum changed\" }})?).ok_or(DecodeError::Invariant {{ detail: \"validated enum changed\" }})?", field.type_name.as_deref().ok_or("enum lacks type name")?),
        "string" => "std::str::from_utf8(&span.bytes[span.start..span.end]).map_err(|_| DecodeError::Invariant { detail: \"validated UTF-8 changed\" })?".to_owned(),
        "bytes" => "&span.bytes[span.start..span.end]".to_owned(),
        "message" => {
            let child = field.type_name.as_deref().ok_or("message lacks type name")?;
            if messages[child].fields.is_empty() {
                format!("Validated{child} {{ _private: () }}")
            } else {
                format!("Validated{child} {{ span }}")
            }
        }
        other => return Err(format!("unsupported accessor type {other}")),
    };
    match field.kind.as_str() {
        "uint32" | "uint64" | "bool" | "enum" => Ok(format!(
            "match raw {{ RawField::Varint(value) => {expression}, _ => return Err(DecodeError::Invariant {{ detail: \"validated scalar changed\" }}) }}"
        )),
        "float" => Ok(format!(
            "match raw {{ RawField::Fixed32(value) => {expression}, _ => return Err(DecodeError::Invariant {{ detail: \"validated float changed\" }}) }}"
        )),
        "double" => Ok(format!(
            "match raw {{ RawField::Fixed64(value) => {expression}, _ => return Err(DecodeError::Invariant {{ detail: \"validated double changed\" }}) }}"
        )),
        "string" | "bytes" | "message" => {
            let bytes_pattern = if field.kind == "message"
                && field
                    .type_name
                    .as_deref()
                    .is_some_and(|name| messages[name].fields.is_empty())
            {
                "RawField::Bytes(_)"
            } else {
                "RawField::Bytes(span)"
            };
            Ok(format!(
                "match raw {{ {bytes_pattern} => {expression}, _ => return Err(DecodeError::Invariant {{ detail: \"validated bytes changed\" }}) }}"
            ))
        }
        _ => Err("unsupported accessor return mapping".into()),
    }
}

const ACCESSOR_CORE: &str = r#"#[derive(Clone, Copy)]
enum RawField<'a> { Varint(u64), Fixed64(u64), Bytes(Span<'a>), Fixed32(u32) }
fn find_raw(span: Span<'_>, tag: u32) -> Result<Option<RawField<'_>>, DecodeError> {
    let mut cursor = span.start;
    while let Some((found, raw)) = next_raw(span, &mut cursor)? { if found == tag { return Ok(Some(raw)); } }
    Ok(None)
}
fn next_raw<'a>(span: Span<'a>, cursor: &mut usize) -> Result<Option<(u32, RawField<'a>)>, DecodeError> {
    if *cursor == span.end { return Ok(None); }
    let path = WirePath::new();
    let key = read_varint(span.bytes, cursor, span.end, path).map_err(to_decode_error)?;
    let tag = u32::try_from(key >> 3).map_err(|_| DecodeError::Invariant { detail: "validated field tag changed" })?;
    let raw = match key & 7 {
        0 => RawField::Varint(read_varint(span.bytes, cursor, span.end, path).map_err(to_decode_error)?),
        1 => { let end = cursor.checked_add(8).ok_or(DecodeError::Invariant { detail: "validated fixed64 changed" })?; let bytes = span.bytes.get(*cursor..end).ok_or(DecodeError::Invariant { detail: "validated fixed64 changed" })?; *cursor = end; RawField::Fixed64(u64::from_le_bytes(bytes.try_into().map_err(|_| DecodeError::Invariant { detail: "validated fixed64 changed" })?)) },
        2 => { let (start, end) = read_length(span.bytes, cursor, span.end, path).map_err(to_decode_error)?; RawField::Bytes(Span { bytes: span.bytes, start, end }) },
        5 => { let end = cursor.checked_add(4).ok_or(DecodeError::Invariant { detail: "validated fixed32 changed" })?; let bytes = span.bytes.get(*cursor..end).ok_or(DecodeError::Invariant { detail: "validated fixed32 changed" })?; *cursor = end; RawField::Fixed32(u32::from_le_bytes(bytes.try_into().map_err(|_| DecodeError::Invariant { detail: "validated fixed32 changed" })?)) },
        _ => return Err(DecodeError::Invariant { detail: "validated wire type changed" }),
    };
    Ok(Some((tag, raw)))
}
"#;
