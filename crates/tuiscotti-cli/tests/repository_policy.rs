#![doc = "Repository-only dependency policy checks for the packaged CLI."]

#[path = "functional/dependencies.rs"]
mod dependencies;

mod helpers {
    use std::path::Path;

    pub(crate) use crate::dependencies::check_direct_dependencies;

    #[expect(
        clippy::expect_used,
        reason = "repository policy test directories must build"
    )]
    pub(crate) fn make_dir(path: &Path) {
        std::fs::create_dir_all(path).expect("repository policy test directory builds");
    }

    #[expect(
        clippy::expect_used,
        reason = "repository policy test files must persist"
    )]
    pub(crate) fn write_bytes(path: &Path, bytes: &[u8]) {
        std::fs::write(path, bytes).expect("repository policy test file writes");
    }

    #[expect(
        clippy::expect_used,
        reason = "repository policy test directory must build"
    )]
    pub(crate) fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("repository policy test directory builds")
    }
}

#[path = "functional/dep_rules.rs"]
mod dep_rules;

#[test]
fn direct_dependency_policy_matches_workspace_and_manifest() {
    let candidates = helpers::temp_dir();
    dep_rules::phase_direct_dep_rules(candidates.path());
}
