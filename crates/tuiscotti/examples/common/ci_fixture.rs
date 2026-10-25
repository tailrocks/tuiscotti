//! CI frozen-fixture helper shared by the GOAL Section 17 examples.
//!
//! Approval is refused while `CI` is set, so under `CI` the examples
//! plant the candidate bytes as a frozen baseline fixture and run
//! check-only flows instead of approving. Nothing here changes the
//! runner environment.

use std::path::{Path, PathBuf};

/// True when the `CI` variable is present with any value, including
/// empty.
#[must_use]
pub(crate) fn ci_is_set() -> bool {
    std::env::var_os("CI").is_some()
}

/// Plant candidate bytes as a frozen baseline fixture: writes them to
/// the store-relative `<test_id>.tui.json` path under the baseline root,
/// creating parent directories. Check-only flows use this instead of
/// approval under `CI`; it writes exactly one baseline file and no
/// evidence records.
pub(crate) fn plant_frozen_baseline(
    baselines: &Path,
    test_id: &str,
    bytes: &[u8],
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut relative = PathBuf::from(test_id);
    relative.set_extension("tui.json");
    let path = baselines.join(&relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, bytes)?;
    Ok(path)
}
