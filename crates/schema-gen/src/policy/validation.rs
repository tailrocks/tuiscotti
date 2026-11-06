use std::collections::{BTreeMap, BTreeSet};

use prost_types::{
    DescriptorProto, FileDescriptorSet,
    field_descriptor_proto::{Label, Type},
};

use super::super::model::{FieldPolicy, MessagePolicy, Policy};
use constraints::{validate_constraint_catalog, validate_constraints};

mod constraints;

fn add_messages<'a>(
    prefix: &str,
    descriptors: &'a [DescriptorProto],
    output: &mut BTreeMap<String, &'a DescriptorProto>,
) {
    for descriptor in descriptors {
        let name = descriptor.name.as_deref().unwrap_or("");
        let full_name = if prefix.is_empty() {
            name.to_owned()
        } else {
            format!("{prefix}.{name}")
        };
        output.insert(full_name.clone(), descriptor);
        add_messages(&full_name, &descriptor.nested_type, output);
    }
}
pub(crate) fn validate(set: &FileDescriptorSet, p: &Policy) -> Result<(), String> {
    if p.version != 1 {
        return Err("unsupported policy version".into());
    }
    if set.file.len() != 1 {
        return Err("expected exactly one descriptor file".into());
    }
    let file = &set.file[0];
    if file.package.as_deref() != Some(&p.package) {
        return Err("package differs from policy".into());
    }
    validate_messages(file, p)?;
    validate_enums(file, p)?;
    validate_constraint_catalog(p)?;
    validate_repeated_exact_count_constraints(p)?;
    validate_image_pixel_grid_constraint(p)
}

fn validate_repeated_exact_count_constraints(p: &Policy) -> Result<(), String> {
    for (message_name, message) in &p.messages {
        for constraint in &message.constraints {
            let Some((field_name, count)) = constraint.split_once("_COUNT_EXACT_") else {
                continue;
            };
            let count = count
                .parse::<u64>()
                .map_err(|_| format!("invalid exact-count constraint {constraint}"))?;
            if count == 0 {
                return Err(format!(
                    "exact-count constraint {constraint} must be positive"
                ));
            }
            let field_name = field_name.to_ascii_lowercase();
            if message
                .fields
                .get(&field_name)
                .is_none_or(|field| field.cardinality != "repeated")
            {
                return Err(format!(
                    "exact-count constraint {constraint} does not name a repeated field in {message_name}"
                ));
            }
        }
    }
    Ok(())
}

fn validate_image_pixel_grid_constraint(p: &Policy) -> Result<(), String> {
    let Some(image) = p.messages.get("Image") else {
        return Ok(());
    };
    let required = "IMAGE_PIXEL_DIMS_MATCH_GRID_10X20";
    if !image
        .constraints
        .iter()
        .any(|constraint| constraint == required)
    {
        return Err(format!("Image message must declare {required}"));
    }
    Ok(())
}

fn validate_messages(file: &prost_types::FileDescriptorProto, p: &Policy) -> Result<(), String> {
    let mut messages = BTreeMap::new();
    add_messages(&p.package, &file.message_type, &mut messages);
    let actual: BTreeSet<_> = messages.keys().cloned().collect();
    let declared: BTreeSet<_> = p
        .messages
        .keys()
        .map(|n| format!("{}.{}", p.package, n))
        .collect();
    if actual != declared {
        return Err(format!(
            "message set mismatch; descriptor={actual:?}, policy={declared:?}"
        ));
    }
    for (short, d) in &p.messages {
        if d.fields.len() > 16 {
            return Err(format!(
                "message {short} exceeds the generated 16-field scan summary"
            ));
        }
        let fq = format!("{}.{}", p.package, short);
        let descriptor = messages[&fq];
        validate_message_fields(d, descriptor, p, &fq)?;
        validate_retired_inventory(short, descriptor, p, &fq)?;
    }
    Ok(())
}

fn validate_message_fields(
    message: &MessagePolicy,
    descriptor: &DescriptorProto,
    p: &Policy,
    fq: &str,
) -> Result<(), String> {
    let mut found = BTreeSet::new();
    for field in &descriptor.field {
        let name = field.name.as_deref().ok_or("unnamed descriptor field")?;
        let field_policy = message
            .fields
            .get(name)
            .ok_or_else(|| format!("unclassified field {fq}.{name}"))?;
        validate_field(field, field_policy, descriptor, p, fq, name)?;
        found.insert(name.to_owned());
    }
    let declared: BTreeSet<_> = message.fields.keys().cloned().collect();
    if found != declared {
        return Err(format!("field set mismatch for {fq}"));
    }
    validate_constraints(&message.constraints, p)
}

fn validate_field(
    field: &prost_types::FieldDescriptorProto,
    field_policy: &FieldPolicy,
    descriptor: &DescriptorProto,
    p: &Policy,
    fq: &str,
    name: &str,
) -> Result<(), String> {
    let number = field.number.ok_or("field without number")?;
    let kind = field_kind(field);
    let repeated = field.label() == Label::Repeated;
    let oneof_index = field.oneof_index.map(|i| {
        usize::try_from(i)
            .ok()
            .and_then(|index| descriptor.oneof_decl.get(index))
            .and_then(|oneof| oneof.name.clone())
            .unwrap_or_default()
    });
    let synthetic = field.proto3_optional.unwrap_or(false);
    let cardinality = if repeated {
        "repeated"
    } else if oneof_index.is_some() && !synthetic {
        "oneof"
    } else {
        "singular"
    };
    let presence_valid = if repeated {
        field_policy.presence == "repeated"
    } else if oneof_index.is_some() && !synthetic {
        field_policy.presence == "oneof"
    } else {
        ["required", "optional"].contains(&field_policy.presence.as_str())
    };
    let scalar = !matches!(
        field.r#type(),
        Type::Message | Type::Group | Type::Bytes | Type::String
    );
    let explicit_presence_valid = !scalar || repeated || oneof_index.is_some() || synthetic;
    if field_policy.tag != number
        || field_policy.kind != kind
        || field_policy.cardinality != cardinality
        || !presence_valid
        || !explicit_presence_valid
        || field_policy.oneof != oneof_index.filter(|_| !synthetic)
        || field_policy.type_name
            != field
                .type_name
                .as_deref()
                .and_then(|type_name| type_name.rsplit('.').next())
                .map(str::to_owned)
    {
        return Err(format!("policy descriptor mismatch at {fq}.{name}"));
    }
    if field_policy.handler.is_empty() {
        return Err(format!("empty handler at {fq}.{name}"));
    }
    validate_constraints(&field_policy.constraints, p)?;
    validate_base_constraint(field, field_policy, fq, name)?;
    if !["singular", "repeated", "oneof"].contains(&field_policy.cardinality.as_str())
        || !["required", "optional", "repeated", "oneof"].contains(&field_policy.presence.as_str())
    {
        return Err(format!("invalid presence/cardinality at {fq}.{name}"));
    }
    Ok(())
}

fn validate_base_constraint(
    field: &prost_types::FieldDescriptorProto,
    policy: &FieldPolicy,
    fq: &str,
    name: &str,
) -> Result<(), String> {
    let base = match field.r#type() {
        Type::String => Some("UTF8"),
        Type::Uint32 => Some("UINT32"),
        Type::Uint64 => Some("UINT64"),
        Type::Bool => Some("BOOL_0_1"),
        Type::Float => Some("FLOAT32_BITS"),
        Type::Double => Some("FLOAT64_BITS"),
        Type::Enum => Some("ENUM_NONZERO_KNOWN"),
        _ => None,
    };
    if base.is_some_and(|id| !policy.constraints.iter().any(|value| value == id)) {
        return Err(format!("missing base constraint {base:?} at {fq}.{name}"));
    }
    Ok(())
}

fn validate_retired_inventory(
    short: &str,
    descriptor: &DescriptorProto,
    p: &Policy,
    fq: &str,
) -> Result<(), String> {
    let reserved_nums: BTreeSet<_> = descriptor
        .reserved_range
        .iter()
        .flat_map(|range| range.start.unwrap_or(0)..range.end.unwrap_or(0))
        .collect();
    let reserved_names: BTreeSet<_> = descriptor.reserved_name.iter().cloned().collect();
    let policy_retired: BTreeSet<_> = p
        .retired
        .iter()
        .filter(|retired| retired.message == short)
        .map(|retired| (retired.name.clone(), retired.number))
        .collect();
    let proto_retired: BTreeSet<_> = reserved_names
        .iter()
        .map(|name| (name.clone(), -1))
        .chain(reserved_nums.iter().map(|number| (String::new(), *number)))
        .collect();
    let policy_inventory: BTreeSet<_> = policy_retired
        .iter()
        .flat_map(|(name, number)| [(name.clone(), *number), (String::new(), *number)])
        .filter(|entry| proto_retired.contains(entry))
        .collect();
    if policy_inventory != proto_retired {
        return Err(format!("retired name/number inventory mismatch for {fq}"));
    }
    Ok(())
}

fn validate_enums(file: &prost_types::FileDescriptorProto, p: &Policy) -> Result<(), String> {
    let mut actual_enums = BTreeMap::new();
    for e in &file.enum_type {
        actual_enums.insert(e.name.clone().unwrap_or_default(), &e.value);
    }
    let actual: BTreeSet<_> = actual_enums.keys().cloned().collect();
    let declared: BTreeSet<_> = p.enums.keys().cloned().collect();
    if actual != declared {
        return Err(format!(
            "enum set mismatch; descriptor={actual:?}, policy={declared:?}"
        ));
    }
    for (name, values) in &p.enums {
        let desc: Vec<_> = actual_enums[name]
            .iter()
            .map(|v| {
                (
                    v.number.unwrap_or_default(),
                    v.name.clone().unwrap_or_default(),
                )
            })
            .collect();
        let pol: Vec<_> = values.iter().map(|v| (v.number, v.name.clone())).collect();
        if desc != pol {
            return Err(format!("enum descriptor mismatch for {name}"));
        }
        if values.first().is_none_or(|v| v.number != 0 || v.accepted) {
            return Err(format!("enum {name} must reject zero"));
        }
        if values.iter().skip(1).any(|v| !v.accepted) {
            return Err(format!("known nonzero values of {name} must be accepted"));
        }
    }
    Ok(())
}

fn field_kind(f: &prost_types::FieldDescriptorProto) -> String {
    let kind = match f.r#type() {
        Type::Double => "double",
        Type::Float => "float",
        Type::Int64 => "int64",
        Type::Uint64 => "uint64",
        Type::Int32 => "int32",
        Type::Fixed64 => "fixed64",
        Type::Fixed32 => "fixed32",
        Type::Bool => "bool",
        Type::String => "string",
        Type::Group => "group",
        Type::Message => "message",
        Type::Bytes => "bytes",
        Type::Uint32 => "uint32",
        Type::Enum => "enum",
        Type::Sfixed32 => "sfixed32",
        Type::Sfixed64 => "sfixed64",
        Type::Sint32 => "sint32",
        Type::Sint64 => "sint64",
    };
    kind.to_owned()
}

// rendering implementation is below
