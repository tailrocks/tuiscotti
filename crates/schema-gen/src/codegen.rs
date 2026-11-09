use std::{env, fs, process::Command};

use super::model::{FieldPolicy, MessagePolicy};

pub(super) fn snake(s: &str) -> String {
    let mut out = String::new();
    let chars: Vec<_> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let previous_is_lower_or_digit = i > 0
            && (chars[i - 1].is_ascii_lowercase() || chars[i - 1].is_ascii_digit());
        let acronym_ends = i > 0
            && c.is_ascii_uppercase()
            && chars
                .get(i + 1)
                .is_some_and(|next| next.is_ascii_lowercase())
            && chars[i - 1].is_ascii_uppercase();
        if c.is_ascii_uppercase() && (previous_is_lower_or_digit || acronym_ends) {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}
pub(super) fn format_rust(raw: &[u8]) -> Result<Vec<u8>, String> {
    let temp = env::temp_dir().join(format!(
        "tuiscotti-schema-generated-{}.rs",
        std::process::id()
    ));
    fs::write(&temp, raw).map_err(|e| e.to_string())?;
    let result = Command::new("rustfmt")
        .args(["--edition", "2024"])
        .arg(&temp)
        .output()
        .map_err(|e| format!("cannot run rustfmt: {e}"))?;
    if !result.status.success() {
        let _ = fs::remove_file(&temp);
        return Err(format!(
            "rustfmt failed: {}",
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    let formatted = fs::read(&temp).map_err(|e| e.to_string())?;
    let _ = fs::remove_file(temp);
    Ok(formatted)
}
pub(super) fn encode_scalar_type(
    kind: &str,
    type_name: Option<&str>,
    repeated_item: bool,
) -> String {
    match kind {
        "uint32" => "u32".into(),
        "bool" => "bool".into(),
        "float" => "f32".into(),
        "double" => "f64".into(),
        "string" => {
            if repeated_item {
                "&'a str".into()
            } else {
                "&str".into()
            }
        }
        "bytes" => {
            if repeated_item {
                "&'a [u8]".into()
            } else {
                "&[u8]".into()
            }
        }
        "enum" => type_name.unwrap_or("i32").into(),
        _ => "u64".into(),
    }
}
pub(super) fn ordered_fields(m: &MessagePolicy) -> Vec<(&String, &FieldPolicy)> {
    let mut v: Vec<_> = m.fields.iter().collect();
    v.sort_by_key(|(_, f)| f.tag);
    v
}
pub(super) fn handler_type(
    message: &str,
    field_name: &str,
    field: &FieldPolicy,
    messages: &std::collections::BTreeMap<String, MessagePolicy>,
) -> String {
    if field.cardinality == "repeated" {
        return format!("{}{}Iter<'a>", pascal(message), pascal(field_name));
    }
    let scalar = match field.kind.as_str() {
        "uint32" => "u32".to_owned(),
        "bool" => "bool".to_owned(),
        "float" => "f32".to_owned(),
        "double" => "f64".to_owned(),
        "string" => "&'a str".to_owned(),
        "bytes" => "&'a [u8]".to_owned(),
        "enum" => field.type_name.as_deref().unwrap_or("i32").to_owned(),
        "message" => {
            let child = field.type_name.as_deref().unwrap_or("Message");
            if messages.get(child).is_some_and(|message| message.fields.is_empty()) {
                format!("Validated{child}")
            } else {
                format!("Validated{child}<'a>")
            }
        }
        _ => "u64".to_owned(),
    };
    if field.presence == "optional" {
        format!("Option<{scalar}>")
    } else {
        scalar
    }
}
pub(super) fn enum_variant(s: &str, enumeration: &str) -> String {
    let prefix = format!("{}_", snake(enumeration).to_ascii_uppercase());
    let name = s.strip_prefix(&prefix).unwrap_or(s);
    name.split('_')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut c = p.chars();
            c.next()
                .map(|x| {
                    x.to_ascii_uppercase().to_string() + c.as_str().to_ascii_lowercase().as_str()
                })
                .unwrap_or_default()
        })
        .collect()
}
pub(super) fn to_upper(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}
pub(super) fn pascal(s: &str) -> String {
    s.split('_')
        .map(|p| {
            let mut c = p.chars();
            c.next()
                .map(|x| x.to_ascii_uppercase().to_string() + c.as_str())
                .unwrap_or_default()
        })
        .collect()
}
pub(super) fn sanitize(s: &str) -> String {
    if [
        "type", "match", "ref", "loop", "self", "Self", "enum", "struct", "fn", "mod", "use",
        "where", "async", "await", "move", "impl", "trait", "const", "static", "pub", "crate",
        "super", "in", "for", "if", "else", "return", "break", "continue", "let", "mut", "as",
        "dyn", "unsafe", "extern", "false", "true",
    ]
    .contains(&s)
    {
        format!("r#{s}")
    } else {
        s.to_owned()
    }
}
