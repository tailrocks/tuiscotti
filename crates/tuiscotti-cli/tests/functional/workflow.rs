use std::path::{Path, PathBuf};

use image::GenericImageView as _;

use super::helpers::{
    ChildEnv, accept_candidate, baseline_samples, case_dir, ci_lane, decode_png, exit_code,
    glyph_probe_sample, log_sample, make_dir, open_lane, parse_json_line, pixel_test_id,
    report_file_name, run_cli, sha256_file, utf8, walk_files, write_bytes,
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

pub(super) fn live_workflow() {
    let dirs = workflow_dirs();
    let lane = ci_lane();
    let open = open_lane();
    let live_id = pixel_test_id("live-workflow");
    let candidate_v1 = dirs.candidates.join("v1.tui.pb");
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
    let candidate_v2 = dirs.candidates.join("v2.tui.pb");
    write_bytes(&candidate_v2, &log_sample(&live_id, 30, 1, 8));
    let (code, body) = check_sample(&candidate_v2, &dirs.baselines, &dirs.artifacts, &lane);
    assert_eq!(code, 1);
    assert_eq!(body["status"], "different");
    assert_eq!(body["candidate_sha256"], sha256_file(&candidate_v2));
    let candidate_bytes = std::fs::read(&candidate_v2).unwrap_or_default();
    assert!(walk_files(&dirs.artifacts).iter().any(|path| {
        path.file_name().is_some_and(|name| name == "sample.tui.pb")
            && std::fs::read(path).is_ok_and(|saved| saved == candidate_bytes)
    }));
    let report_name = report_file_name(&body);
    assert!(walk_files(&dirs.artifacts).iter().any(|path| {
        path.file_name()
            .and_then(std::ffi::OsStr::to_str)
            .is_some_and(|name| name == report_name)
    }));
    let candidate_v3 = dirs.candidates.join("v3.tui.pb");
    write_bytes(&candidate_v3, &log_sample(&live_id, 40, 1, 8));
    let (code, _) = check_sample(&candidate_v3, &dirs.baselines, &dirs.artifacts, &lane);
    assert_eq!(code, 1);
    phase_render_report(&dirs, &candidate_v2, &report_name, &live_id, &lane);
    phase_missing_glyph(&dirs, &lane);
    #[cfg(unix)]
    phase_report_symlink_loop();
    #[cfg(unix)]
    phase_report_relative_href(&dirs.baselines, &candidate_v2, &lane);
}

#[cfg(unix)]
fn phase_report_symlink_loop() {
    let root = case_dir("report-symlink-loop");
    let baselines = root.path().join("baselines");
    let artifacts = root.path().join("artifacts");
    let candidates = root.path().join("candidates");
    for dir in [&baselines, &artifacts, &candidates] {
        make_dir(dir);
    }
    let lane = ci_lane();
    let open = open_lane();
    let live_id = pixel_test_id("report-symlink-loop");
    let candidate_v1 = candidates.join("v1.tui.pb");
    write_bytes(&candidate_v1, &log_sample(&live_id, 30, 0, 8));
    let sha_v1 = sha256_file(&candidate_v1);
    let (code, _) = accept_candidate(&candidate_v1, &sha_v1, "missing", &baselines, &open);
    assert_eq!(code, 0);
    let candidate_v2 = candidates.join("v2.tui.pb");
    write_bytes(&candidate_v2, &log_sample(&live_id, 30, 1, 8));
    let (code, _) = check_sample(&candidate_v2, &baselines, &artifacts, &lane);
    assert_eq!(code, 1);
    let index = candidates.join("index.html");
    let out = run_cli(
        &[
            "report",
            "--artifacts",
            utf8(&artifacts),
            "--out",
            utf8(&index),
            "--json",
        ],
        &lane,
    );
    assert_eq!(exit_code(&out), 0);
    let before = std::fs::read(&index).unwrap_or_default();
    let before_text = String::from_utf8_lossy(&before);
    assert!(before_text.contains("<p>records: 1</p>"));
    assert_eq!(before_text.matches(">report</a>").count(), 1);
    let sub = artifacts.join("sub");
    make_dir(&sub);
    assert!(std::os::unix::fs::symlink(&artifacts, sub.join("back")).is_ok());
    let out = run_cli(
        &[
            "report",
            "--artifacts",
            utf8(&artifacts),
            "--out",
            utf8(&index),
            "--json",
        ],
        &lane,
    );
    assert_eq!(exit_code(&out), 0);
    let after = std::fs::read(&index).unwrap_or_default();
    let after_text = String::from_utf8_lossy(&after);
    assert!(after_text.contains("<p>records: 1</p>"));
    assert_eq!(after_text.matches(">report</a>").count(), 1);
    assert_eq!(after, before);
}

#[cfg(unix)]
#[expect(
    clippy::expect_used,
    reason = "canonical temp parent and report JSON shape are assertions"
)]
fn phase_report_relative_href(baselines: &Path, candidate_v2: &Path, lane: &ChildEnv<'_>) {
    let parent = case_dir("report-relative-href");
    let canonical = std::fs::canonicalize(parent.path()).expect("canonical temp parent");
    for (dirname, encoded_first) in [("runs-local", "runs-local"), ("runs:local", "runs%3Alocal")] {
        let artifacts = canonical.join(dirname);
        make_dir(&artifacts);
        let (code, body) = check_sample(candidate_v2, baselines, &artifacts, lane);
        assert_eq!(code, 1);
        assert_eq!(body["status"], "different");
        let report_path = PathBuf::from(
            body.get("output_path")
                .and_then(|path| path.get("value"))
                .and_then(serde_json::Value::as_str)
                .expect("report path present"),
        );
        assert!(report_path.is_file());
        let suffix = report_path
            .strip_prefix(&artifacts)
            .expect("report under artifact root");
        let slug = dirname.replace(':', "-");
        let index = canonical.join(format!("index-{slug}.html"));
        let out = run_cli(
            &[
                "report",
                "--artifacts",
                utf8(&artifacts),
                "--out",
                utf8(&index),
                "--json",
            ],
            lane,
        );
        assert_eq!(exit_code(&out), 0);
        let index_bytes = std::fs::read(&index).unwrap_or_default();
        let text = String::from_utf8_lossy(&index_bytes);
        assert!(text.contains("<p>records: 1</p>"));
        assert_eq!(text.matches(">report</a>").count(), 1);
        let href = text
            .split("href=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("report anchor href");
        let mut expected = std::ffi::OsString::from(encoded_first);
        expected.push("/");
        expected.push(suffix);
        assert_eq!(href, expected.to_str().expect("href utf8"));
    }
}

fn phase_missing_glyph(dirs: &WorkflowDirs, lane: &ChildEnv<'_>) {
    let sample = dirs.candidates.join("glyph.tui.pb");
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
