//! Verifies the standalone schema generator through the repository's MBX Cargo path.

use std::{error::Error, fs, path::PathBuf, process::Command};

use prost::Message as _;
use prost_types::FileDescriptorSet;

#[test]
fn standalone_schema_generator_tests_and_freshness_pass() -> Result<(), Box<dyn Error>> {
    let mut workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(workspace.pop());
    assert!(workspace.pop());

    let schema = workspace.join("crates/tuiscotti/src/sample/wire/schema");
    let descriptor = fs::read(schema.join("tuiscotti-v1.desc"))?;
    let descriptor = FileDescriptorSet::decode(descriptor.as_slice())?;
    assert!(
        descriptor
            .file
            .iter()
            .any(|file| { file.name.as_deref() == Some("tuiscotti-v1.proto") })
    );
    let policy = fs::read_to_string(schema.join("tuiscotti-v1.policy.toml"))?;
    let _: toml::Value = parse_toml(&policy)?;

    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    let version = Command::new(protoc).arg("--version").output()?;
    assert!(version.status.success(), "vendored protoc did not run");
    assert_eq!(String::from_utf8(version.stdout)?.trim(), "libprotoc 36.2");

    let test = Command::new("mbx")
        .args([
            "nextest",
            "run",
            "--manifest-path",
            "crates/schema-gen/Cargo.toml",
            "--locked",
            "--offline",
        ])
        .current_dir(&workspace)
        .env_remove("NEXTEST_PROFILE")
        .status()?;
    assert!(test.success(), "schema generator tests failed: {test}");

    let check = Command::new("mbx")
        .args([
            "run",
            "--manifest-path",
            "crates/schema-gen/Cargo.toml",
            "--locked",
            "--offline",
            "--",
            "check",
        ])
        .current_dir(&workspace)
        .env_remove("PROTOC")
        .status()?;
    assert!(
        check.success(),
        "schema generator freshness check failed: {check}"
    );
    Ok(())
}

fn parse_toml<T: serde::de::DeserializeOwned>(source: &str) -> Result<T, toml::de::Error> {
    toml::from_str(source)
}
