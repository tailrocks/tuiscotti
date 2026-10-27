#![doc = "End-to-end functional suite for the shipped tuiscotti binary."]
use std::path::{Path, PathBuf};

use image::GenericImageView as _;

#[path = "functional/helpers.rs"]
mod helpers;

use helpers::{
    ChildEnv, baseline_samples, case_dir, check_direct_dependencies, ci_lane, decode_png,
    exit_code, glyph_probe_sample, log_sample, make_dir, open_lane, parse_json_line, pixel_test_id,
    report_file_name, run_cli, sha256_bytes, sha256_file, utf8, walk_files, write_bytes,
};
#[derive(Debug)]
struct WorkflowDirs {
    _root: tempfile::TempDir,
    baselines: PathBuf,
    artifacts: PathBuf,
    candidates: PathBuf,
}

fn workflow_dirs() -> WorkflowDirs {
    let root = case_dir("live-workflow");
    let baselines = root.path().join("baselines");
    let artifacts = root.path().join("artifacts");
    let candidates = root.path().join("candidates");
    for dir in [&baselines, &artifacts, &candidates] {
        make_dir(dir);
    }
    WorkflowDirs {
        _root: root,
        baselines,
        artifacts,
        candidates,
    }
}

fn check_sample(
    sample: &Path,
    baselines: &Path,
    artifacts: &Path,
    env: &ChildEnv<'_>,
) -> (i32, serde_json::Value) {
    let args = [
        "check",
        "--sample",
        utf8(sample),
        "--baselines",
        utf8(baselines),
        "--artifacts",
        utf8(artifacts),
        "--json",
    ];
    let out = run_cli(&args, env);
    (exit_code(&out), parse_json_line(&out.stdout))
}

fn accept_candidate(
    candidate: &Path,
    candidate_sha256: &str,
    expect_baseline: &str,
    baselines: &Path,
    env: &ChildEnv<'_>,
) -> (i32, Vec<u8>) {
    let args = [
        "accept",
        "--candidate",
        utf8(candidate),
        "--candidate-sha256",
        candidate_sha256,
        "--expect-baseline",
        expect_baseline,
        "--baselines",
        utf8(baselines),
        "--json",
    ];
    let out = run_cli(&args, env);
    (exit_code(&out), out.stdout)
}

#[test]
fn live_workflow() {
    let dirs = workflow_dirs();
    let lane = ci_lane();
    let open = open_lane();
    let live_id = pixel_test_id("live-workflow");
    let candidate_v1 = dirs.candidates.join("v1.tui.json");
    write_bytes(&candidate_v1, &log_sample(&live_id, 30, 0, 8));
    let (code, body) = check_sample(&candidate_v1, &dirs.baselines, &dirs.artifacts, &lane);
    assert_eq!(code, 1);
    assert_eq!(body["status"], "missing");
    assert_eq!(body["baseline_sha256"], "missing");
    assert_eq!(walk_files(&dirs.baselines), [] as [std::path::PathBuf; 0]);
    let sha_v1 = sha256_file(&candidate_v1);
    let (code, _) = accept_candidate(&candidate_v1, &sha_v1, "missing", &dirs.baselines, &open);
    assert_eq!(code, 0);
    let stored = baseline_samples(&dirs.baselines);
    assert_eq!(stored.len(), 1);
    assert_eq!(sha256_file(&stored[0]), sha_v1);
    let (code, body) = check_sample(&candidate_v1, &dirs.baselines, &dirs.artifacts, &lane);
    assert_eq!(code, 0);
    assert_eq!(body["status"], "pass");
    assert_eq!(body["baseline_sha256"], sha_v1);
    let candidate_v2 = dirs.candidates.join("v2.tui.json");
    write_bytes(&candidate_v2, &log_sample(&live_id, 30, 1, 8));
    let (code, body) = check_sample(&candidate_v2, &dirs.baselines, &dirs.artifacts, &lane);
    assert_eq!(code, 1);
    assert_eq!(body["status"], "different");
    let report_name = report_file_name(&body);
    assert!(walk_files(&dirs.artifacts).iter().any(|path| {
        path.file_name()
            .and_then(std::ffi::OsStr::to_str)
            .is_some_and(|name| name == report_name)
    }));
    let candidate_v3 = dirs.candidates.join("v3.tui.json");
    write_bytes(&candidate_v3, &log_sample(&live_id, 40, 1, 8));
    let (code, _) = check_sample(&candidate_v3, &dirs.baselines, &dirs.artifacts, &lane);
    assert_eq!(code, 1);
    phase_render_report(&dirs, &candidate_v2, &report_name, &live_id, &lane);
    phase_missing_glyph(&dirs, &lane);
}

fn phase_missing_glyph(dirs: &WorkflowDirs, lane: &ChildEnv<'_>) {
    let sample = dirs.candidates.join("glyph.tui.json");
    write_bytes(&sample, &glyph_probe_sample("functional/missing-glyph"));
    let out = dirs.candidates.join("glyph.png");
    let result = run_cli(
        &[
            "render",
            "--sample",
            utf8(&sample),
            "--out",
            utf8(&out),
            "--json",
        ],
        lane,
    );
    assert_eq!(exit_code(&result), 3);
    assert!(!out.exists());
    let body = parse_json_line(&result.stdout);
    assert_eq!(body["status"], "error");
    assert_eq!(body["error_code"], "MISSING_GLYPH");
}

fn phase_render_report(
    dirs: &WorkflowDirs,
    candidate: &Path,
    report_name: &str,
    live_id: &str,
    lane: &ChildEnv<'_>,
) {
    let png = dirs.candidates.join("v2.png");
    let out = run_cli(
        &[
            "render",
            "--sample",
            utf8(candidate),
            "--out",
            utf8(&png),
            "--json",
        ],
        lane,
    );
    assert_eq!(exit_code(&out), 0);
    let png_bytes = std::fs::read(&png).unwrap_or_default();
    let png_image = decode_png(&png_bytes);
    assert_eq!(png_image.dimensions(), (300, 160));
    let html = dirs.candidates.join("v2.html");
    let out = run_cli(
        &[
            "render",
            "--sample",
            utf8(candidate),
            "--out",
            utf8(&html),
            "--json",
        ],
        lane,
    );
    assert_eq!(exit_code(&out), 0);
    let html_text = std::fs::read_to_string(&html).unwrap_or_default();
    assert!(html_text.contains("<!DOCTYPE html>"));
    assert!(html_text.contains(live_id));
    assert!(html_text.contains("data:image/png;base64,"));
    let index = dirs.candidates.join("index.html");
    let out = run_cli(
        &[
            "report",
            "--artifacts",
            utf8(&dirs.artifacts),
            "--out",
            utf8(&index),
            "--json",
        ],
        lane,
    );
    assert_eq!(exit_code(&out), 0);
    let index_text = std::fs::read_to_string(&index).unwrap_or_default();
    assert!(index_text.contains(report_name));
}

#[test]
fn invalid_input() {
    const OVERSIZED: usize = 64 * 1024 * 1024 + 1;
    let root = case_dir("invalid-input");
    let baselines = root.path().join("baselines");
    let artifacts = root.path().join("artifacts");
    let candidates = root.path().join("candidates");
    make_dir(&candidates);
    let lane = ci_lane();
    let sample = candidates.join("good.tui.json");
    write_bytes(
        &sample,
        &log_sample(&pixel_test_id("invalid-input"), 30, 0, 8),
    );
    let missing = candidates.join("absent.tui.json");
    let out = run_cli(
        &[
            "check",
            "--sample",
            utf8(&missing),
            "--baselines",
            utf8(&baselines),
            "--artifacts",
            utf8(&artifacts),
            "--json",
        ],
        &lane,
    );
    assert_eq!(exit_code(&out), 3);
    assert_eq!(parse_json_line(&out.stdout)["error_code"], "IO");
    let corrupt = candidates.join("corrupt.tui.json");
    write_bytes(&corrupt, b"{\"schema\": \"tuiscotti.sample.v9\"}");
    let out = run_cli(
        &[
            "check",
            "--sample",
            utf8(&corrupt),
            "--baselines",
            utf8(&baselines),
            "--artifacts",
            utf8(&artifacts),
            "--json",
        ],
        &lane,
    );
    assert_eq!(exit_code(&out), 3);
    assert!(parse_json_line(&out.stdout)["error_code"].is_string());
    let huge = candidates.join("huge.tui.json");
    write_bytes(&huge, &vec![b'x'; OVERSIZED]);
    let out = run_cli(
        &[
            "check",
            "--sample",
            utf8(&huge),
            "--baselines",
            utf8(&baselines),
            "--artifacts",
            utf8(&artifacts),
            "--json",
        ],
        &lane,
    );
    assert_eq!(exit_code(&out), 3);
    assert_eq!(parse_json_line(&out.stdout)["error_code"], "RESOURCE_LIMIT");
    phase_usage_errors(&candidates, &sample, &baselines, &lane);
    phase_direct_dep_rules(&candidates);
    assert_eq!(walk_files(&baselines), [] as [std::path::PathBuf; 0]);
}

const DEP_CRATE: &str =
    "[package]\nname = \"tuiscotti\"\nversion = \"0.2.0\"\nedition = \"2024\"\n";
const DEP_BARE_WS: &str = "[workspace]\n";
const DEP_WS_ALIAS: &str = "[workspace]\n[workspace.dependencies]\npty = { package = \"portable-pty\", version = \"0.9\" }\n";

fn dep_case(dir: &Path, seq: u32, workspace: Option<&str>, crate_toml: Option<&str>, want: i32) {
    let ws_path = dir.join(format!("workspace-{seq}.toml"));
    let crate_path = dir.join(format!("crate-{seq}.toml"));
    if let Some(text) = workspace {
        write_bytes(&ws_path, text.as_bytes());
    }
    if let Some(text) = crate_toml {
        write_bytes(&crate_path, text.as_bytes());
    }
    assert_eq!(check_direct_dependencies(&ws_path, &crate_path), want);
}

fn phase_direct_dep_rules(candidates: &Path) {
    let dir = candidates.join("dep-cases");
    make_dir(&dir);
    let manifest_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert_eq!(
        check_direct_dependencies(
            &manifest_root.join("Cargo.toml"),
            &manifest_root.join("crates/tuiscotti/Cargo.toml")
        ),
        0
    );
    dep_rule_flat_tables(&dir);
    dep_rule_dotted_tables(&dir);
    dep_rule_errors(&dir);
}

fn dep_rule_flat_tables(dir: &Path) {
    let mut seq = 0;
    let mut case = |workspace: Option<&str>, crate_toml: Option<&str>, want: i32| {
        seq += 1;
        dep_case(dir, seq, workspace, crate_toml, want);
    };
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\nportable-pty = \"0.9\"\n"
        )),
        1,
    );
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\npty = {{ package = \"portable-pty\", version = \"0.9\" }}\n"
        )),
        1,
    );
    case(
        Some(DEP_WS_ALIAS),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\npty.workspace = true\n"
        )),
        1,
    );
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[build-dependencies]\npty = {{ package = \"portable-pty\", version = \"0.9\" }}\n"
        )),
        1,
    );
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[dev-dependencies]\nterm = {{ package = \"termpane\", version = \"0.1\" }}\n"
        )),
        1,
    );
    case(
        Some(DEP_WS_ALIAS),
        Some(&format!(
            "{DEP_CRATE}[target.'cfg(windows)'.dependencies]\npty.workspace = true\n"
        )),
        1,
    );
    case(
        Some(DEP_WS_ALIAS),
        Some(&format!(
            "{DEP_CRATE}[dependencies.ok]\nversion = \"1.0\"\n"
        )),
        0,
    );
    case(Some(DEP_WS_ALIAS), Some(DEP_CRATE), 0);
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\npty = {{ package = 'portable-pty', version = '0.9' }}\n"
        )),
        1,
    );
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\n\"portable-pty\" = \"0.9\"\n"
        )),
        1,
    );
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\npty = {{ package = \"portable\\u002dpty\", path = \"../portable\" }}\n"
        )),
        1,
    );
}

fn dep_rule_dotted_tables(dir: &Path) {
    let mut seq = 100;
    let mut case = |workspace: Option<&str>, crate_toml: Option<&str>, want: i32| {
        seq += 1;
        dep_case(dir, seq, workspace, crate_toml, want);
    };
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[dependencies.pty]\npackage = \"portable-pty\"\nversion = \"0.9\"\n"
        )),
        1,
    );
    case(
        Some(DEP_BARE_WS),
        Some("dependencies.pty.package = \"portable-pty\"\ndependencies.pty.version = \"0.9\"\n"),
        1,
    );
    case(
        Some(DEP_BARE_WS),
        Some("dependencies.ok.version = \"1.0\"\n"),
        0,
    );
    case(
        Some("workspace.dependencies.pty = { package = \"portable-pty\", version = \"0.9\" }\n"),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\npty.workspace = true\n"
        )),
        1,
    );
    case(
        Some(
            "workspace.dependencies.pty.package = \"portable-pty\"\nworkspace.dependencies.pty.path = \"portable\"\n",
        ),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\npty.workspace = true\n"
        )),
        1,
    );
}

fn dep_rule_errors(dir: &Path) {
    let mut seq = 200;
    let mut case = |workspace: Option<&str>, crate_toml: Option<&str>, want: i32| {
        seq += 1;
        dep_case(dir, seq, workspace, crate_toml, want);
    };
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\npty.workspace = true\n"
        )),
        2,
    );
    case(Some(DEP_BARE_WS), Some("[[["), 2);
    let ws_path = dir.join("workspace-unreadable.toml");
    write_bytes(&ws_path, DEP_BARE_WS.as_bytes());
    assert_eq!(
        check_direct_dependencies(&ws_path, &dir.join("crate-absent.toml")),
        2
    );
}

fn phase_usage_errors(candidates: &Path, sample: &Path, baselines: &Path, lane: &ChildEnv<'_>) {
    let out = run_cli(
        &[
            "accept",
            "--candidate",
            utf8(sample),
            "--candidate-sha256",
            "xyz",
            "--expect-baseline",
            "missing",
            "--baselines",
            utf8(baselines),
            "--json",
        ],
        lane,
    );
    assert_eq!(exit_code(&out), 2);
    assert_eq!(out.stdout, [] as [u8; 0]);
    let bad_out = candidates.join("render.bad");
    let out = run_cli(
        &[
            "render",
            "--sample",
            utf8(sample),
            "--out",
            utf8(&bad_out),
            "--json",
        ],
        lane,
    );
    assert_eq!(exit_code(&out), 2);
    assert_eq!(out.stdout, [] as [u8; 0]);
}

#[test]
fn ci_refuses_approval() {
    let candidate_bytes = log_sample(&pixel_test_id("ci-refuses-approval"), 30, 0, 8);
    let digest = sha256_bytes(&candidate_bytes);
    for (name, vars, remove_ci, expect_code) in [
        ("unset", &[][..], true, 0),
        ("empty", &[("CI", "")][..], false, 3),
        ("false-value", &[("CI", "false")][..], false, 3),
        ("true-value", &[("CI", "true")][..], false, 3),
    ] {
        let root = case_dir(name);
        let baselines = root.path().join("baselines");
        make_dir(&baselines);
        let candidate = root.path().join("candidate.tui.json");
        write_bytes(&candidate, &candidate_bytes);
        let env = ChildEnv {
            vars,
            remove_ci,
            close_stdout: false,
        };
        let (code, stdout) =
            accept_candidate(&candidate, digest.as_str(), "missing", &baselines, &env);
        assert_eq!(code, expect_code);
        if expect_code == 0 {
            assert_eq!(baseline_samples(&baselines).len(), 1);
        } else {
            assert_eq!(walk_files(&baselines), [] as [std::path::PathBuf; 0]);
            assert_eq!(
                parse_json_line(&stdout)["error_code"],
                "CI_APPROVAL_FORBIDDEN"
            );
        }
    }
}

#[test]
fn closed_stdout() {
    let root = case_dir("closed-stdout");
    let baselines = root.path().join("baselines");
    let artifacts = root.path().join("artifacts");
    let candidates = root.path().join("candidates");
    for dir in [&baselines, &artifacts, &candidates] {
        make_dir(dir);
    }
    let good = candidates.join("good.tui.json");
    write_bytes(
        &good,
        &log_sample(&pixel_test_id("closed-stdout"), 30, 0, 8),
    );
    let digest = sha256_file(&good);
    let (code, _) = accept_candidate(&good, digest.as_str(), "missing", &baselines, &open_lane());
    assert_eq!(code, 0);
    let pass_env = ChildEnv {
        vars: &[("CI", "true")],
        remove_ci: false,
        close_stdout: true,
    };
    let out = run_cli(
        &[
            "check",
            "--sample",
            utf8(&good),
            "--baselines",
            utf8(&baselines),
            "--artifacts",
            utf8(&artifacts),
            "--json",
        ],
        &pass_env,
    );
    assert_eq!(exit_code(&out), 0);
    assert!(!String::from_utf8_lossy(&out.stderr).contains("panicked"));
    let absent = candidates.join("absent.tui.json");
    let out = run_cli(
        &[
            "check",
            "--sample",
            utf8(&absent),
            "--baselines",
            utf8(&baselines),
            "--artifacts",
            utf8(&artifacts),
            "--json",
        ],
        &pass_env,
    );
    assert_eq!(exit_code(&out), 3);
    assert!(!String::from_utf8_lossy(&out.stderr).contains("panicked"));
}
