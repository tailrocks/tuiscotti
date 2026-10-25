//! D06 executable-examples lane: the five GOAL Section 17 examples run green.
//!
//! Each `examples/0N-*.rs` binary must exit 0 and print its `EXAMPLE-0N-...`
//! marker. Binaries resolve at runtime from this test's own location
//! (`<profile>/deps/<test>` → `<profile>/examples/<name>`), so the lane
//! works under `cargo nextest run`, custom target dirs, and release
//! profiles. Build the examples first with the toolchain-guard command:
//! `rust-build-guard -- mise exec -- cargo build --examples` (preceded by
//! `disk-check`). This test never shells out to `cargo` itself.

use std::path::PathBuf;
use std::process::Command;

/// (example binary, acceptable stdout markers — first is the normal one).
const CASES: &[(&str, &[&str])] = &[
    ("01-pure-view", &["EXAMPLE-01-OK"]),
    // Without a Unix PTY the live session reports SKIP, still exit 0.
    ("02-live-session", &["EXAMPLE-02-OK", "EXAMPLE-02-SKIP"]),
    ("03-missing-baseline", &["EXAMPLE-03-OK"]),
    ("04-frozen-baseline", &["EXAMPLE-04-OK"]),
    ("05-state-only", &["EXAMPLE-05-OK"]),
];

fn examples_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let exe = std::env::current_exe()?;
    // .../target/<profile>/deps/examples_lane-<hash> → .../target/<profile>
    let profile = exe
        .parent()
        .and_then(|dir| dir.parent())
        .ok_or("profile dir above deps/")?
        .to_path_buf();
    Ok(profile.join("examples"))
}

#[test]
fn examples_lane_executes_everything() {
    let dir = examples_dir().expect("examples_dir succeeds");
    let mut failures = Vec::new();
    for (name, markers) in CASES {
        let path = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        if !path.is_file() {
            failures.push(format!(
                "binary missing at {}: build it with \
                 `rust-build-guard -- mise exec -- cargo build --examples`",
                path.display()
            ));
            continue;
        }
        match Command::new(&path).output() {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
                if !out.status.success() {
                    failures.push(format!(
                        "{name}: exit {}:\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
                        out.status
                    ));
                } else if !markers.iter().any(|marker| stdout.contains(marker)) {
                    failures.push(format!(
                        "{name}: exit 0 but no marker {markers:?} in stdout:\n{stdout}\n--- stderr ---\n{stderr}"
                    ));
                }
            }
            Err(error) => failures.push(format!("{name}: cannot run {}: {error}", path.display())),
        }
    }
    assert!(
        failures.is_empty(),
        "{} example(s) failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
