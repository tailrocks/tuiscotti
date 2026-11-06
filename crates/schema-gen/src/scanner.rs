use std::collections::BTreeMap;
use std::fmt::Write as _;

mod runtime;
use runtime::SCANNER_RUNTIME;

use super::{
    codegen::{format_rust, to_upper},
    model::Policy,
};

pub(super) fn render_scanner(policy: &Policy) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut code = String::from(
        "include!(\"scanner_core.rs\");\ninclude!(\"scanner_engine.rs\");\ninclude!(\"scanner_validate.rs\");\ninclude!(\"scanner_finish.rs\");\nfn fields_for(kind: &str) -> Option<&'static [FieldSpec]> { match kind {\n",
    );
    for message in policy.messages.keys() {
        let _ = writeln!(
            code,
            "    {message:?} => Some(FIELDS_{}),",
            to_upper(message)
        );
    }
    code.push_str("    _ => None, } }\n");
    emit_repeated_count_helpers(&mut code, policy);
    code.push_str("fn enum_accepts(kind: &str, value: i32) -> bool { match kind {\n");
    let mut enum_groups: BTreeMap<Vec<i32>, Vec<&str>> = BTreeMap::new();
    for (name, values) in &policy.enums {
        let mut accepted_values = values
            .iter()
            .filter(|value| value.accepted)
            .map(|value| value.number)
            .collect::<Vec<_>>();
        accepted_values.sort_unstable();
        enum_groups
            .entry(accepted_values)
            .or_default()
            .push(name);
    }
    for (accepted_values, names) in enum_groups {
        let accepted = if accepted_values.windows(2).all(|pair| pair[1] == pair[0] + 1) {
            match (accepted_values.first(), accepted_values.last()) {
                (Some(first), Some(last)) if first < last => format!("{}..={last}", first),
                _ => accepted_values.iter().map(ToString::to_string).collect::<Vec<_>>().join(" | "),
            }
        } else {
            accepted_values.iter().map(ToString::to_string).collect::<Vec<_>>().join(" | ")
        };
        let names = names
            .iter()
            .map(|name| format!("{name:?}"))
            .collect::<Vec<_>>()
            .join(" | ");
        let _ = writeln!(code, "    {names} => matches!(value, {accepted}),");
    }
    code.push_str("    _ => false, } }\n");
    code.push_str("fn image_grid_scale() -> (u32, u32) { (10, 20) }\n");
    let mut output = BTreeMap::new();
    output.insert("scanner.rs".into(), format_rust(code.as_bytes())?);
    let (root_source, rest) = SCANNER_RUNTIME
        .split_once("fn scan(root")
        .ok_or("scanner runtime split marker missing")?;
    let (engine, validate) = rest
        .split_once("fn check_varint(")
        .ok_or("scanner validation split marker missing")?;
    let (validate, finish) = validate
        .split_once("fn check_test_id(")
        .ok_or("scanner finish split marker missing")?;
    output.insert(
        "scanner_core.rs".into(),
        format_rust(root_source.as_bytes())?,
    );
    output.insert(
        "scanner_engine.rs".into(),
        format_rust(format!("fn scan(root{engine}").as_bytes())?,
    );
    output.insert(
        "scanner_validate.rs".into(),
        format_rust(format!("fn check_varint({validate}").as_bytes())?,
    );
    output.insert(
        "scanner_finish.rs".into(),
        format_rust(format!("fn check_test_id({finish}").as_bytes())?,
    );
    Ok(output)
}

fn emit_repeated_count_helpers(code: &mut String, policy: &Policy) {
    for function in ["repeated_limit", "repeated_exact_count"] {
        let _ = writeln!(
            code,
            "fn {function}(message: &str, field: &str) -> Option<u64> {{ match (message, field) {{"
        );
        for (message_name, message) in &policy.messages {
            for constraint in &message.constraints {
                let Some((field_name, count)) = constraint.split_once("_COUNT_EXACT_") else {
                    continue;
                };
                let Ok(count) = count.parse::<u64>() else {
                    continue;
                };
                let field_name = field_name.to_ascii_lowercase();
                if message
                    .fields
                    .get(&field_name)
                    .is_some_and(|field| field.cardinality == "repeated")
                {
                    let _ = writeln!(
                        code,
                        "    ({message_name:?}, {field_name:?}) => Some({count}),"
                    );
                }
            }
        }
        let _ = writeln!(code, "    _ => None, }} }}");
    }
}
