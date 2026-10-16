//! Shared integration-test helpers: Insta update-mode predicates.
//!
//! `set_var` is an `unsafe fn` in edition 2024 and cannot be used under the
//! workspace lints, so tests can no longer force `INSTA_UPDATE`. These
//! predicates replicate insta 1.48's resolution (`snapshot_update_behavior`
//! and `is_ci`, safe env reads only) so mode-dependent assertions can guard
//! or skip instead. Assumes no insta config file sets `behavior.update`
//! (true: this repo ships none).

#![allow(dead_code, reason = "each test binary uses a different subset")]

use std::env;

/// Whether Insta writes `.snap.new` pendings (and fails) on mismatch.
pub(crate) fn insta_writes_new_files() -> bool {
    match env::var("INSTA_UPDATE").ok().as_deref() {
        Some("new") => true,
        // `auto` (and unset): pendings locally, nothing on CI.
        Some("auto" | "") | None => !is_ci(),
        // `no` never writes; `always`/`1`/`unseen`/`force` bless in place;
        // unknown values make Insta itself error.
        Some(_) => false,
    }
}

/// Whether Insta blesses approvals in place on mismatch.
pub(crate) fn insta_updates_in_place() -> bool {
    matches!(
        env::var("INSTA_UPDATE").ok().as_deref(),
        Some("always" | "1" | "unseen" | "force")
    )
}

/// Whether Insta writes nothing at all on mismatch (fails clean).
pub(crate) fn insta_writes_nothing() -> bool {
    !insta_writes_new_files() && !insta_updates_in_place()
}

/// Replica of insta's `is_ci`: `CI` false/0/empty → false, unset → whether
/// `TF_BUILD` is set, anything else → true.
fn is_ci() -> bool {
    match env::var("CI").ok().as_deref() {
        Some("false" | "0" | "") => false,
        None => env::var("TF_BUILD").is_ok(),
        Some(_) => true,
    }
}

/// Re-execute the current test in a child process with `CI` removed when the
/// outer test runner (`cargo test` / `cargo nextest` on GitHub Actions) set
/// `CI` in the ambient environment.
pub(crate) fn rerun_without_ci() -> bool {
    if !tuiscotti::grouped::is_ci_env() {
        return false;
    }
    let exact = std::thread::current()
        .name()
        .unwrap_or_default()
        .to_string();
    assert!(!exact.is_empty(), "libtest sets test thread name");
    let exe = env::current_exe().unwrap_or_default();
    assert!(!exe.as_os_str().is_empty(), "current_exe failed");
    let out = std::process::Command::new(exe)
        .args(["--exact", &exact, "--nocapture"])
        .env_remove("CI")
        .output();
    assert!(out.is_ok(), "spawn child without CI failed");
    if let Ok(out) = out {
        assert!(
            out.status.success(),
            "child `{exact}` failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
    }
    true
}
