use std::{fs, path::Path};

use super::model::Policy;

mod depth;
mod path;
mod validation;

pub(crate) use depth::validate_production_scan_depth;
pub(crate) use path::validate_compact_path_policy;
pub(crate) use validation::validate;

pub(super) fn parse_policy(text: &str) -> Result<Policy, String> {
    toml::from_str(text).map_err(|error| format!("invalid policy: {error}"))
}

pub(super) fn check_file(path: &Path, want: &[u8]) -> Result<(), String> {
    let got = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if got != want {
        return Err(format!(
            "{} is stale; run `schema-gen generate`",
            path.display()
        ));
    }
    Ok(())
}
