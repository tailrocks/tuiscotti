use std::{collections::BTreeMap, env, fs, path::PathBuf, process::Command};

use super::super::GENERATED;
use super::{fixtures, recording_handler, rustc_preamble};
use crate::model::{FieldPolicy, MessagePolicy};

#[test]
fn generated_module_compiles_and_fields_are_canonical() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let generated = root
        .join(GENERATED)
        .canonicalize()
        .expect("generated module");
    let temp = env::temp_dir().join(format!("schema-gen-valid-{}.rs", std::process::id()));
    let output = env::temp_dir().join(format!("schema-gen-valid-{}", std::process::id()));
    let mut source = format!(
        "{}#[path = {:?}] mod generated;\n",
        rustc_preamble(),
        generated.to_string_lossy()
    );
    source.push_str(include_str!("../../testdata/generated_behavior.rs.txt"));
    source.push_str("use generated::*;\n");
    let (_, policy, _) = fixtures();
    source.push_str(&recording_handler(&policy));
    fs::write(&temp, source).expect("write compile probe");
    let result = Command::new("rustc")
        .args([
            "--edition=2024",
            "--cfg=test",
            "--crate-name=schema_gen_valid",
        ])
        .arg(&temp)
        .arg("-o")
        .arg(&output)
        .output()
        .expect("run rustc");
    assert!(
        result.status.success(),
        "generated module failed to compile: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let run = Command::new(&output)
        .status()
        .expect("run canonical-order probe");
    let _ = fs::remove_file(&temp);
    let _ = fs::remove_file(&output);
    assert!(
        run.success(),
        "generated field metadata is not in canonical tag order"
    );
}

#[test]
fn validated_tokens_cannot_be_forged_outside_the_generated_module() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let generated = root
        .join(GENERATED)
        .canonicalize()
        .expect("generated module");
    let temp = env::temp_dir().join(format!("schema-gen-forge-{}.rs", std::process::id()));
    let output = env::temp_dir().join(format!("schema-gen-forge-{}.rlib", std::process::id()));
    let source = format!(
        "{}#[path = {:?}] mod generated;\nfn forge() {{ let _ = generated::ValidatedDefaultColor; }}\n",
        rustc_preamble(),
        generated.to_string_lossy()
    );
    fs::write(&temp, source).expect("write private-field probe");
    let result = Command::new("rustc")
        .args([
            "--edition=2024",
            "--crate-type=lib",
            "--crate-name=schema_gen_forge",
        ])
        .arg(&temp)
        .arg("-o")
        .arg(&output)
        .output()
        .expect("run rustc");
    let _ = fs::remove_file(&temp);
    let _ = fs::remove_file(&output);
    assert!(
        !result.status.success(),
        "validated token was externally constructible"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("_private") && stderr.contains("private"),
        "{stderr}"
    );
}

#[test]
fn generated_handler_omission_fails_compilation() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let generated = root
        .join(GENERATED)
        .canonicalize()
        .expect("generated module");
    let temp = env::temp_dir().join(format!("schema-gen-omit-{}.rs", std::process::id()));
    let output = env::temp_dir().join(format!("schema-gen-omit-{}.rlib", std::process::id()));
    let source = format!(
        r"{}
#[path = {:?}] mod generated;
fn forge() {{ let _ = generated::ValidatedDefaultColor; }}
struct Partial;
impl<'a> generated::DecodeColor<'a> for Partial {{
    type Error = ();
    fn default_color(&mut self, _: generated::ValidatedDefaultColor) -> Result<(), ()> {{ Ok(()) }}
}}
",
        rustc_preamble(),
        generated.to_string_lossy()
    );
    fs::write(&temp, source).expect("write compile-fail probe");
    let result = Command::new("rustc")
        .args([
            "--edition=2024",
            "--crate-type=lib",
            "--crate-name=schema_gen_omission",
        ])
        .arg(&temp)
        .arg("-o")
        .arg(&output)
        .output()
        .expect("run rustc");
    let _ = fs::remove_file(&temp);
    let _ = fs::remove_file(&output);
    assert!(
        !result.status.success(),
        "incomplete generated handler unexpectedly compiled"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("not all trait items implemented"));
}

#[test]
fn adding_a_field_to_empty_message_requires_mapper_update() {
    let (_, mut policy, _) = fixtures();
    let field = FieldPolicy {
        tag: 1,
        kind: "uint32".into(),
        cardinality: "singular".into(),
        presence: "implicit".into(),
        oneof: None,
        type_name: None,
        handler: "future_field".into(),
        constraints: Vec::new(),
    };
    let mut fields = BTreeMap::new();
    fields.insert("future_field".to_owned(), field);
    let message = MessagePolicy {
        fields,
        constraints: Vec::new(),
    };
    policy.messages.insert("DefaultColor".into(), message);

    let generated_handler = super::super::render::render_handler_file(
        "DefaultColor",
        &policy.messages["DefaultColor"],
        &policy,
    );
    let mut generated_visitor = String::new();
    super::super::accessors::render_visitor(
        &mut generated_visitor,
        &policy,
        "DefaultColor",
        &policy.messages["DefaultColor"],
        false,
    );
    let temp = env::temp_dir().join(format!("schema-gen-new-field-{}.rs", std::process::id()));
    let output = env::temp_dir().join(format!("schema-gen-new-field-{}", std::process::id()));
    let source = format!(
        "use std::marker::PhantomData;\n#[derive(Debug)] struct DecodeError;\nstruct ValidatedDefaultColor<'a>(PhantomData<&'a ()>);\nimpl<'a> ValidatedDefaultColor<'a> {{ fn future_field(&self) -> Result<u32, DecodeError> {{ Ok(1) }}\n{generated_visitor}}}\n{generated_handler}\nstruct Mapper;\nimpl<'a> DecodeDefaultColor<'a> for Mapper {{ type Error = DecodeError; }}\nfn main() {{}}\n"
    );
    fs::write(&temp, source).expect("write future-field compile probe");
    let result = Command::new("rustc")
        .args(["--edition=2024", "--crate-name=schema_gen_new_field"])
        .arg(&temp)
        .arg("-o")
        .arg(&output)
        .output()
        .expect("run rustc");
    let _ = fs::remove_file(&temp);
    let _ = fs::remove_file(&output);
    assert!(
        !result.status.success(),
        "mapper compiled after a generated message gained an unhandled field"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("not all trait items implemented"),
        "{stderr}"
    );
}

#[test]
fn mapping_constraint_hook_omission_fails_compilation() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let generated = root
        .join(GENERATED)
        .canonicalize()
        .expect("generated module");
    let temp = env::temp_dir().join(format!("schema-gen-mapping-{}.rs", std::process::id()));
    let output = env::temp_dir().join(format!("schema-gen-mapping-{}.rlib", std::process::id()));
    let source = format!(
        "{}#[path = {:?}] mod generated;\nstruct Partial;\nimpl<'a> generated::DecodeSnapshot<'a> for Partial {{ type Error = DecodeError; }}\n",
        rustc_preamble(),
        generated.to_string_lossy()
    );
    fs::write(&temp, source).expect("write mapping hook compile-fail probe");
    let result = Command::new("rustc")
        .args([
            "--edition=2024",
            "--crate-type=lib",
            "--crate-name=schema_gen_mapping",
        ])
        .arg(&temp)
        .arg("-o")
        .arg(&output)
        .output()
        .expect("run mapping hook compile-fail probe");
    let _ = fs::remove_file(&temp);
    let _ = fs::remove_file(&output);
    assert!(
        !result.status.success(),
        "mapping constraint hook omission compiled"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("upstream_snapshot_invariants"), "{stderr}");
}
