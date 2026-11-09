use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use prost::Message;
use prost_types::FileDescriptorSet;

use super::{DESC, GENERATED_DIR, POLICY, PROTO, SCHEMA_DIR, policy, render};

pub(super) fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let generating = match args.as_slice() {
        [command] if command == "generate" => true,
        [command] if command == "check" => false,
        _ => return Err("usage: schema-gen <check|generate>".into()),
    };
    run_with_mode(generating, &args[0])
}

fn run_with_mode(generating: bool, command: &str) -> Result<(), String> {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").map_err(|e| e.to_string())?);
    let schema = root.join(SCHEMA_DIR);
    let descriptor = compile_descriptor(&schema, &schema.join(PROTO))?;
    let desc_path = schema.join(DESC);
    if generating {
        fs::write(&desc_path, &descriptor).map_err(|e| e.to_string())?;
    }
    check_descriptor(&desc_path, &descriptor)?;
    let policy_text = fs::read_to_string(schema.join(POLICY)).map_err(|e| e.to_string())?;
    let parsed_policy = policy::parse_policy(&policy_text)?;
    let descriptor_set = FileDescriptorSet::decode(descriptor.as_slice())
        .map_err(|e| format!("invalid generated descriptor: {e}"))?;
    policy::validate(&descriptor_set, &parsed_policy)?;
    policy::validate_compact_path_policy(&descriptor_set, &parsed_policy)?;
    let scan_depth = policy::validate_production_scan_depth(&parsed_policy)?;
    let output = render::render(&descriptor_set, &parsed_policy)?;
    if generating {
        write_outputs(&root, &output)?;
    } else {
        check_outputs(&root, &output)?;
    }
    println!(
        "schema-gen {}: descriptor and {} generated artifacts are current",
        command,
        output.len()
    );
    println!("schema-gen production message depth: {scan_depth}/32 frames");
    Ok(())
}

fn compile_descriptor(schema: &Path, proto: &Path) -> Result<Vec<u8>, String> {
    let temp_desc = env::temp_dir().join(format!(
        "tuiscotti-v1-generated-{}.desc",
        std::process::id()
    ));
    let protoc = match env::var_os("PROTOC") {
        Some(protoc) => PathBuf::from(protoc),
        None => protoc_bin_vendored::protoc_bin_path()
            .map_err(|error| format!("cannot locate vendored protoc: {error}"))?,
    };
    let version = Command::new(&protoc)
        .arg("--version")
        .output()
        .map_err(|e| format!("cannot query protoc version: {e}"))?;
    let version = String::from_utf8_lossy(&version.stdout);
    if version.trim() != "libprotoc 36.2" {
        return Err(format!("expected protoc 36.2, found {}", version.trim()));
    }
    let status = Command::new(protoc)
        .arg(format!("--proto_path={}", schema.display()))
        .arg(format!("--descriptor_set_out={}", temp_desc.display()))
        .arg(proto)
        .status()
        .map_err(|e| format!("cannot run protoc: {e}"))?;
    if !status.success() {
        return Err(format!("protoc exited with {status}"));
    }
    fs::read(temp_desc).map_err(|e| e.to_string())
}

fn check_descriptor(path: &Path, generated: &[u8]) -> Result<(), String> {
    let checked = fs::read(path).map_err(|e| format!("missing checked-in descriptor: {e}"))?;
    if generated != checked {
        return Err("descriptor differs from protoc output; run `schema-gen generate`".into());
    }
    Ok(())
}

fn expected_files(output: &BTreeMap<PathBuf, Vec<u8>>) -> BTreeSet<PathBuf> {
    output
        .keys()
        .filter_map(|path| path.strip_prefix(GENERATED_DIR).ok())
        .map(PathBuf::from)
        .collect()
}

fn write_outputs(root: &Path, output: &BTreeMap<PathBuf, Vec<u8>>) -> Result<(), String> {
    let generated_dir = root.join(GENERATED_DIR);
    fs::create_dir_all(&generated_dir).map_err(|e| e.to_string())?;
    for (relative, bytes) in output {
        if bytes.split(|byte| *byte == b'\n').count() > 401 {
            return Err(format!(
                "generated artifact {} exceeds 400 lines",
                relative.display()
            ));
        }
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(path, bytes).map_err(|e| e.to_string())?;
    }
    remove_stale_outputs(&generated_dir, &expected_files(output))
}

fn check_outputs(root: &Path, output: &BTreeMap<PathBuf, Vec<u8>>) -> Result<(), String> {
    for (relative, bytes) in output {
        policy::check_file(&root.join(relative), bytes)?;
    }
    let generated_dir = root.join(GENERATED_DIR);
    let expected = expected_files(output);
    for entry in fs::read_dir(&generated_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = PathBuf::from(entry.file_name());
        if entry.file_type().map_err(|e| e.to_string())?.is_file()
            && name.extension().is_some_and(|ext| ext == "rs")
            && !expected.contains(&name)
        {
            return Err(format!(
                "unexpected generated artifact {}",
                entry.path().display()
            ));
        }
    }
    Ok(())
}

fn remove_stale_outputs(generated_dir: &Path, expected: &BTreeSet<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(generated_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = PathBuf::from(entry.file_name());
        if entry.file_type().map_err(|e| e.to_string())?.is_file()
            && name.extension().is_some_and(|ext| ext == "rs")
            && !expected.contains(&name)
        {
            fs::remove_file(entry.path()).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
