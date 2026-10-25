//! C18 (D07): no duplicate live engine, renderer, or input encoder remains.
//!
//! After the section-3.3 removal, the `tuiscotti` facade must own only its
//! section-3.1 responsibilities (view capture, samples, frozen checks,
//! approval, reports). This test trips if a removed local engine,
//! renderer, locator, encoder, daemon, recorder, store, or adapter crate
//! re-enters the workspace, the facade manifest, the facade surface, or
//! the locked dependency graph as a workspace-local package.
//!
//! PTY support through the upstream `tui-test-rs` dependency stays
//! permitted: only a workspace-local PTY/runtime package is rejected.

use std::path::{Path, PathBuf};

/// Workspace root derived from this crate's manifest directory.
#[expect(clippy::expect_used, reason = "test path setup reads fixed locations")]
fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("tuiscotti crate directory has a parent")
        .parent()
        .expect("crates directory has a parent")
        .to_path_buf()
}

#[expect(clippy::expect_used, reason = "test path setup reads fixed locations")]
fn read_file(path: &Path) -> String {
    std::fs::read_to_string(path).expect("migration tripwire input reads")
}

/// Removed local crates that must never return as workspace members or
/// facade dependencies. `tui-test-rs` is the only permitted engine source.
const REMOVED_LOCAL_CRATES: &[&str] = &[
    "tuiscotti-core",
    "tuiscotti-render",
    "tuiscotti-runtime",
    "tuiscotti-insta",
    "tuiscotti-fixtures",
    "tuiscotti-bench",
];

/// Direct dependencies that encode a removed contract: a local PTY stack
/// (`termpane`), a local input stack (`crossterm`), a local rasterizer
/// (`swash`), a local comparator (`image-compare`), or a second approval
/// engine (`insta`).
const REMOVED_DIRECT_DEPS: &[&str] = &["termpane", "crossterm", "swash", "image-compare", "insta"];

/// Identifiers from the removed live surface that must not reappear in the
/// facade source: sessions, locators, encoders, waits, runners, daemons,
/// recorders, stores, renderers, frozen-tree helpers, and adapters.
const REMOVED_SURFACE_MARKERS: &[&str] = &[
    "tuiscotti_core",
    "tuiscotti_render",
    "tuiscotti_runtime",
    "tuiscotti_insta",
    "BoundLocator",
    "GroupedStore",
    "KeyChord",
    "assert_snapshot",
    "assert_screenshot",
];

/// Live-session type and module names that belong to `tui-test` now.
/// `Session` and `Renderer` are deliberately absent: `Session` appears in
/// doc prose ("no tuiscotti Session") and `Renderer` in render-contract
/// prose, so the module-path markers below carry the signal instead.
const REMOVED_MODULE_MARKERS: &[&str] = &[
    "bound_locator",
    "tui_shell",
    "import_compat",
    "insta_proto",
    "observe::",
    "proto::",
    "runner::",
    "snapshot::",
    "mcp::",
];

/// Only these packages may exist in the workspace.
#[test]
fn workspace_members_are_the_two_shipping_crates() {
    let root = workspace_root();
    let manifest = read_file(&root.join("Cargo.toml"));
    for removed in REMOVED_LOCAL_CRATES {
        assert!(
            !manifest.contains(&format!("\"crates/{removed}\"")),
            "removed crate {removed} is still a workspace member",
        );
    }
    for entry in ["\"crates/tuiscotti\",", "\"crates/tuiscotti-cli\","] {
        assert!(
            manifest.contains(entry),
            "shipping crate entry {entry} is missing from workspace members",
        );
    }
}

/// The facade manifest carries no removed contract: no local engine,
/// renderer, store, fixture, or adapter dependency, no PTY/input/raster
/// dependency, and no feature table that could hide removed code behind
/// a disabled feature.
#[test]
fn facade_manifest_has_no_removed_dependencies_or_features() {
    let root = workspace_root();
    let manifest = read_file(&root.join("crates/tuiscotti/Cargo.toml"));
    for removed in REMOVED_LOCAL_CRATES
        .iter()
        .chain(REMOVED_DIRECT_DEPS.iter())
    {
        assert!(
            !manifest.contains(removed),
            "removed dependency {removed} is still named in crates/tuiscotti/Cargo.toml",
        );
    }
    assert!(
        !manifest.contains("[features]"),
        "crates/tuiscotti/Cargo.toml must not grow a [features] table that hides removed code",
    );
}

/// The facade source re-exports none of the removed live surface.
#[test]
fn facade_surface_has_no_live_engine_renderer_or_encoder() {
    let root = workspace_root();
    let src = root.join("crates/tuiscotti/src");
    let lib = read_file(&src.join("lib.rs"));
    let error = read_file(&src.join("error.rs"));
    for marker in REMOVED_SURFACE_MARKERS
        .iter()
        .chain(REMOVED_MODULE_MARKERS.iter())
    {
        assert!(
            !lib.contains(marker),
            "removed surface marker {marker} is still present in crates/tuiscotti/src/lib.rs",
        );
        assert!(
            !error.contains(marker),
            "removed surface marker {marker} is still present in crates/tuiscotti/src/error.rs",
        );
    }
}

/// Every remaining facade module owns a section-3.1 responsibility.
#[test]
fn facade_modules_match_the_responsibility_map() {
    let root = workspace_root();
    let src = root.join("crates/tuiscotti/src");
    for module in [
        "view.rs",
        "sample.rs",
        "check.rs",
        "approve.rs",
        "report.rs",
        "error.rs",
        "profile.rs",
    ] {
        assert!(
            src.join(module).is_file(),
            "responsibility module crates/tuiscotti/src/{module} is missing",
        );
    }
}

/// Dependency-graph spot check without `cargo metadata` (denied by the
/// Rust guard; see `docs/migration/decisions.md` D-GUARD-1): the lockfile
/// carries no workspace-local engine package, and any `termpane` entry
/// must come from the crates.io registry (transitive through `tui-test`),
/// never from a local path.
#[test]
fn lockfile_has_no_local_engine_or_renderer_package() {
    let root = workspace_root();
    let lock = read_file(&root.join("Cargo.lock"));
    for removed in REMOVED_LOCAL_CRATES {
        assert!(
            !lock.contains(&format!("name = \"{removed}\"")),
            "removed package {removed} is still present in Cargo.lock",
        );
    }
    let mut in_termpane_block = false;
    let mut termpane_is_registry = false;
    let mut termpane_seen = false;
    for line in lock.lines() {
        if line == "name = \"termpane\"" {
            in_termpane_block = true;
            termpane_seen = true;
        } else if in_termpane_block {
            if line.starts_with("source = ") {
                termpane_is_registry = line.contains("registry");
                in_termpane_block = false;
            } else if line.starts_with("name = ") || line == "[[patch.unused]]" {
                in_termpane_block = false;
            }
        }
    }
    if termpane_seen {
        assert!(
            termpane_is_registry,
            "termpane in Cargo.lock must come from the registry (transitive through tui-test), not a local path",
        );
    }
}
