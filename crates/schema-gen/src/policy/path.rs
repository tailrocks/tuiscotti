use std::collections::BTreeMap;

use prost_types::{
    DescriptorProto, FileDescriptorSet,
    field_descriptor_proto::{Label, Type},
};

use super::super::model::Policy;

type MessageGraph = BTreeMap<String, Vec<(Option<String>, bool)>>;

fn visit(
    name: &str,
    graph: &MessageGraph,
    active: &mut Vec<String>,
    field_depth: usize,
    repeated_depth: usize,
) -> Result<(), String> {
    if active.iter().any(|item| item == name) {
        return Err(format!("recursive message cycle reaches {name}"));
    }
    active.push(name.to_owned());
    let fields = graph
        .get(name)
        .ok_or_else(|| format!("missing referenced message {name}"))?;
    for (child, repeated) in fields {
        let depth = field_depth + 1;
        let repeated = repeated_depth + usize::from(*repeated);
        if depth > 5 {
            return Err(format!("wire path exceeds five fields at {name}"));
        }
        if repeated > 1 {
            return Err(format!(
                "wire path contains more than one repeated segment at {name}"
            ));
        }
        if let Some(child) = child {
            visit(child, graph, active, depth, repeated)?;
        }
    }
    active.pop();
    Ok(())
}

pub(crate) fn validate_compact_path_policy(
    set: &FileDescriptorSet,
    policy: &Policy,
) -> Result<(), String> {
    let file = set.file.first().ok_or("descriptor has no file")?;
    let mut messages = BTreeMap::new();
    add_messages(&policy.package, &file.message_type, &mut messages);
    let field_count = messages
        .values()
        .map(|descriptor| descriptor.field.len())
        .sum::<usize>();
    if field_count > usize::from(u8::MAX) + 1 {
        return Err(format!(
            "wire path field ID capacity exceeded: {field_count} fields, maximum 256"
        ));
    }

    // Type names from descriptors are short names in this schema; resolve nested
    // names against the package map before traversing.
    let mut graph = MessageGraph::new();
    for (name, descriptor) in &messages {
        let mut resolved = Vec::new();
        for field in &descriptor.field {
            let child = matches!(field.r#type(), Type::Message | Type::Group)
                .then(|| field.type_name.as_deref())
                .flatten()
                .map(|type_name| {
                    let full_name = type_name.trim_start_matches('.');
                    if messages.contains_key(full_name) {
                        full_name.to_owned()
                    } else {
                        let short = full_name.rsplit('.').next().unwrap_or(full_name);
                        format!("{}.{}", policy.package, short)
                    }
                });
            resolved.push((child, field.label() == Label::Repeated));
        }
        graph.insert(name.clone(), resolved);
    }
    for name in graph.keys() {
        visit(name, &graph, &mut Vec::new(), 0, 0)?;
    }
    Ok(())
}

fn add_messages<'a>(
    prefix: &str,
    descriptors: &'a [DescriptorProto],
    output: &mut BTreeMap<String, &'a DescriptorProto>,
) {
    for descriptor in descriptors {
        let name = descriptor.name.as_deref().unwrap_or("");
        let full_name = format!("{prefix}.{name}");
        output.insert(full_name.clone(), descriptor);
        add_messages(&full_name, &descriptor.nested_type, output);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use prost_types::{
        DescriptorProto, FieldDescriptorProto, FileDescriptorProto, FileDescriptorSet,
        field_descriptor_proto::{Label, Type},
    };

    use crate::model::Policy;

    use super::validate_compact_path_policy;

    fn message(name: &str, fields: Vec<FieldDescriptorProto>) -> DescriptorProto {
        DescriptorProto {
            name: Some(name.to_owned()),
            field: fields,
            ..DescriptorProto::default()
        }
    }

    fn field(name: &str, number: i32, child: Option<&str>, repeated: bool) -> FieldDescriptorProto {
        FieldDescriptorProto {
            name: Some(name.to_owned()),
            number: Some(number),
            label: Some(if repeated {
                Label::Repeated as i32
            } else {
                Label::Optional as i32
            }),
            r#type: Some(if child.is_some() {
                Type::Message as i32
            } else {
                Type::String as i32
            }),
            type_name: child.map(str::to_owned),
            ..FieldDescriptorProto::default()
        }
    }

    fn prove(messages: Vec<DescriptorProto>) -> Result<(), String> {
        let set = FileDescriptorSet {
            file: vec![FileDescriptorProto {
                package: Some("private".into()),
                message_type: messages,
                ..FileDescriptorProto::default()
            }],
        };
        let policy = Policy {
            version: 1,
            package: "private".into(),
            constraints: BTreeMap::new(),
            messages: BTreeMap::new(),
            enums: BTreeMap::new(),
            retired: Vec::new(),
        };
        validate_compact_path_policy(&set, &policy)
    }

    #[test]
    fn rejects_cycles() {
        let sample = message(
            "Sample",
            vec![field("next", 1, Some(".private.Sample"), false)],
        );
        assert!(prove(vec![sample]).is_err_and(|e| e.contains("cycle")));
    }

    #[test]
    fn rejects_six_field_paths() {
        let mut messages = Vec::new();
        for index in 0..6 {
            let name = if index == 0 {
                "Sample".to_owned()
            } else {
                format!("M{index}")
            };
            let child = (index < 5).then(|| format!(".private.M{}", index + 1));
            messages.push(message(
                &name,
                vec![field("next", 1, child.as_deref(), false)],
            ));
        }
        assert!(prove(messages).is_err_and(|e| e.contains("five fields")));
    }

    #[test]
    fn rejects_two_repeated_segments_on_one_path() {
        let sample = message("Sample", vec![field("items", 1, Some(".private.M"), true)]);
        let nested = message("M", vec![field("values", 1, None, true)]);
        assert!(prove(vec![sample, nested]).is_err_and(|e| e.contains("repeated segment")));
    }

    #[test]
    fn rejects_field_id_overflow() {
        let fields = (1..=257)
            .map(|number| field(&format!("f{number}"), number, None, false))
            .collect();
        assert!(prove(vec![message("Sample", fields)]).is_err_and(|e| e.contains("capacity")));
    }
}
