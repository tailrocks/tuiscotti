//! Rework-contract regression: nested snapshot codes (R20a), observed
//! baseline digests (R20b), clap usage errors without JSON (R20c),
//! exit-code mapping at the CLI boundary (R20d), closed-stdout status
//! preservation (R19), and index links relative to the index parent
//! (R23). Every row asserts the process status and the parsed JSON
//! fields together.
//!
//! Library delegation (R24) is covered implicitly: every render and
//! report row below runs through `render_sample` and
//! `write_report_index`.

#[path = "support/spawn.rs"]
mod spawn;

#[path = "support/fixtures.rs"]
mod fixtures;

use std::path::{Path, PathBuf};
use std::process::Stdio;

use fixtures::{
    base_snapshot, candidate_id, pixel_sample_bytes, state_sample_bytes, temp_store,
    write_baseline_file, write_file,
};
use spawn::{assert_no_color, bin, code, parse_json, run_cli, run_cli_env, spawn_locked};

/// One nested wire mutation plus its expected Section 16.3 code.
type NestedCase<'a> = (&'a str, Box<dyn Fn(&mut serde_json::Value) + 'a>, &'a str);

/// A state sample with its snapshot JSON mutated, re-serialized.
#[expect(clippy::expect_used, reason = "rework fixtures must serialize")]
fn mutated_sample(
    dir: &Path,
    name: &str,
    test_id: &str,
    mutate: impl FnOnce(&mut serde_json::Value),
) -> PathBuf {
    let bytes = state_sample_bytes(test_id, &base_snapshot());
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("fixture parses");
    mutate(&mut value);
    let mutated = serde_json::to_vec(&value).expect("mutated sample serializes");
    write_file(dir, name, &mutated)
}

/// Run `check --json` on one sample; return status and parsed JSON.
fn check_json(
    sample: &Path,
    baselines: &Path,
    artifacts: &Path,
) -> (Option<i32>, serde_json::Value) {
    let out = run_cli(&[
        "--json",
        "check",
        "--sample",
        &sample.to_string_lossy(),
        "--baselines",
        &baselines.to_string_lossy(),
        "--artifacts",
        &artifacts.to_string_lossy(),
    ]);
    assert_no_color(&out, "check json");
    let status = code(&out);
    let value = parse_json(&out);
    (status, value)
}

/// R20a: nested snapshot wire failures keep their typed Section 16.3
/// codes instead of flattening to `INVALID_SAMPLE`.
#[test]
fn nested_snapshot_failures_keep_typed_codes() {
    let (dir, baselines, artifacts) = temp_store();
    let big_text = "x".repeat(5_000);
    let big_link = "y".repeat(5_000);
    let big_title = "t".repeat(70_000);
    let cases: Vec<NestedCase<'_>> = vec![
        (
            "future schema",
            Box::new(|value| value["snapshot"]["schema"] = serde_json::json!(999)),
            "UNSUPPORTED_SCHEMA",
        ),
        (
            "missing schema",
            Box::new(|value| {
                value["snapshot"]
                    .as_object_mut()
                    .expect("snapshot is an object")
                    .remove("schema");
            }),
            "UNSUPPORTED_SCHEMA",
        ),
        (
            "oversize dimensions",
            Box::new(|value| {
                value["snapshot"]["cols"] = serde_json::json!(40_000);
                value["snapshot"]["rows"] = serde_json::json!(1);
                value["snapshot"]["cells"] = serde_json::json!([]);
            }),
            "RESOURCE_LIMIT",
        ),
        (
            "oversize cell text",
            Box::new(|value| value["snapshot"]["cells"][0]["text"] = serde_json::json!(big_text)),
            "RESOURCE_LIMIT",
        ),
        (
            "oversize cell link",
            Box::new(|value| {
                value["snapshot"]["cells"][0]["link_uri"] = serde_json::json!(big_link);
            }),
            "RESOURCE_LIMIT",
        ),
        (
            "oversize title",
            Box::new(|value| value["snapshot"]["title"] = serde_json::json!(big_title)),
            "RESOURCE_LIMIT",
        ),
    ];
    // The pixel gate shares the same wire preflight: a future nested
    // schema in a state-and-pixels sample is UNSUPPORTED_SCHEMA too.
    let (_, pixel_bytes) = pixel_sample_bytes("rework/pixel-nested", &base_snapshot());
    let mut pixel: serde_json::Value =
        serde_json::from_slice(&pixel_bytes).expect("fixture parses");
    pixel["snapshot"]["schema"] = serde_json::json!(999);
    let pixel_mutated = serde_json::to_vec(&pixel).expect("mutated sample serializes");
    let pixel_sample = write_file(dir.path(), "pixel-nested.tui.json", &pixel_mutated);
    let (status, parsed) = check_json(&pixel_sample, &baselines, &artifacts);
    assert_eq!(status, Some(3), "pixel future schema exits 3");
    assert_eq!(parsed["status"], "error");
    assert_eq!(parsed["error_code"], "UNSUPPORTED_SCHEMA");
    for (index, (name, mutate, expected)) in cases.into_iter().enumerate() {
        let file = format!("nested-{index}.tui.json");
        let sample = mutated_sample(dir.path(), &file, "rework/nested", mutate);
        let (status, value) = check_json(&sample, &baselines, &artifacts);
        assert_eq!(status, Some(3), "{name} exits 3");
        assert_eq!(value["command"], "check");
        assert_eq!(value["status"], "error");
        assert_eq!(value["error_code"], expected, "{name} keeps {expected}");
    }
}

/// Run `accept --json`; return status and parsed JSON.
fn accept_json(
    candidate: &Path,
    candidate_sha256: &str,
    expect_baseline: &str,
    baselines: &Path,
) -> (Option<i32>, serde_json::Value) {
    let out = run_cli(&[
        "--json",
        "accept",
        "--candidate",
        &candidate.to_string_lossy(),
        "--candidate-sha256",
        candidate_sha256,
        "--expect-baseline",
        expect_baseline,
        "--baselines",
        &baselines.to_string_lossy(),
    ]);
    assert_no_color(&out, "accept json");
    let status = code(&out);
    let value = parse_json(&out);
    (status, value)
}

/// R20b: failure JSON reports the observed baseline digest, R20d: the
/// exit mapping lives at the CLI boundary.
#[test]
fn accept_failures_report_the_observed_baseline() {
    let (dir, baselines, _artifacts) = temp_store();
    let first = state_sample_bytes("rework/observed", &base_snapshot());
    let first_file = write_file(dir.path(), "v1.tui.json", &first);
    let first_digest = candidate_id(&first);
    let wrong = "d".repeat(64);

    // A stale expected digest against an absent baseline observes
    // `missing`, never the requested digest.
    let (status, value) = accept_json(&first_file, &first_digest, &wrong, &baselines);
    assert_eq!(status, Some(3));
    assert_eq!(value["status"], "error");
    assert_eq!(value["error_code"], "STALE_BASELINE");
    assert_eq!(value["baseline_sha256"], "missing");

    // Approving the first revision, then refusing a stale second
    // approval against it, observes the present digest.
    assert_eq!(
        accept_json(&first_file, &first_digest, "missing", &baselines).0,
        Some(0)
    );
    let (status, value) = accept_json(&first_file, &first_digest, &wrong, &baselines);
    assert_eq!(status, Some(3));
    assert_eq!(value["error_code"], "STALE_BASELINE");
    assert_eq!(value["baseline_sha256"], first_digest);

    // A stale candidate never observed a baseline: null, not a guess.
    let (status, value) = accept_json(&first_file, &wrong, "missing", &baselines);
    assert_eq!(status, Some(3));
    assert_eq!(value["error_code"], "STALE_CANDIDATE");
    assert!(value["baseline_sha256"].is_null());

    // A CI refusal carries no observed baseline either.
    let out = run_cli_env(
        &[
            "--json",
            "accept",
            "--candidate",
            &first_file.to_string_lossy(),
            "--candidate-sha256",
            &first_digest,
            "--expect-baseline",
            "missing",
            "--baselines",
            &baselines.to_string_lossy(),
        ],
        &[("CI", "1")],
        false,
    );
    assert_eq!(code(&out), Some(3));
    let value = parse_json(&out);
    assert_eq!(value["error_code"], "CI_APPROVAL_FORBIDDEN");
    assert!(value["baseline_sha256"].is_null());
}

/// R20c: digest and output-extension validation are clap parsers: every
/// row exits 2 with no JSON result on stdout.
#[test]
fn usage_errors_exit_two_without_json() {
    let (dir, baselines, _artifacts) = temp_store();
    let bytes = state_sample_bytes("rework/usage", &base_snapshot());
    let sample = write_file(dir.path(), "candidate.tui.json", &bytes);
    let digest = candidate_id(&bytes);
    let zeros = "0".repeat(64);
    let sample_arg = sample.to_string_lossy().into_owned();
    let baselines_arg = baselines.to_string_lossy().into_owned();
    let accept = |candidate_sha256: &str, expect_baseline: &str| {
        vec![
            "--json".to_string(),
            "accept".to_string(),
            "--candidate".to_string(),
            sample_arg.clone(),
            "--candidate-sha256".to_string(),
            candidate_sha256.to_string(),
            "--expect-baseline".to_string(),
            expect_baseline.to_string(),
            "--baselines".to_string(),
            baselines_arg.clone(),
        ]
    };
    let mut cases: Vec<Vec<String>> = vec![
        accept("xyz", "missing"),
        accept(&zeros, "missing"),
        accept(&digest, &zeros),
        accept(&digest, "MISSING"),
    ];
    for name in ["shot.txt", "shot"] {
        let out = dir.path().join(name);
        cases.push(vec![
            "--json".to_string(),
            "render".to_string(),
            "--sample".to_string(),
            sample_arg.clone(),
            "--out".to_string(),
            out.to_string_lossy().into_owned(),
        ]);
    }
    for argv in &cases {
        let owned: Vec<&str> = argv.iter().map(String::as_str).collect();
        let out = run_cli(&owned);
        assert_eq!(code(&out), Some(2), "{argv:?} exits 2");
        assert!(out.stdout.is_empty(), "{argv:?} emits no stdout JSON");
        assert!(!out.stderr.is_empty(), "{argv:?} explains on stderr");
        assert_no_color(&out, "usage error");
    }
    assert!(
        fixtures::all_files(&baselines).is_empty(),
        "usage errors write nothing"
    );
}

/// Spawn a child with its stdout read end closed; the next stdout write
/// fails with EPIPE (Rust ignores SIGPIPE).
#[expect(clippy::expect_used, reason = "rework harness must spawn")]
fn run_with_closed_stdout(argv: &[&str]) -> std::process::Output {
    let mut child = std::process::Command::new(bin());
    child
        .args(argv)
        .env_remove("CI")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = spawn_locked(&mut child).expect("CLI child spawns");
    drop(child.stdout.take());
    child.wait_with_output().expect("CLI child waits")
}

/// R19: a closed stdout preserves the operation result for mismatch (1)
/// and error (3) outcomes, never a panic, never a masked zero.
#[test]
fn closed_stdout_preserves_mismatch_and_error() {
    let (dir, baselines, artifacts) = temp_store();
    let before = state_sample_bytes("rework/pipe", &base_snapshot());
    write_baseline_file(&baselines, "rework/pipe", &before);
    let changed_cells = vec![fixtures::plain_cell("z", 1); 8];
    let after = state_sample_bytes("rework/pipe", &fixtures::snapshot_with_cells(changed_cells));
    let candidate = write_file(dir.path(), "changed.tui.json", &after);
    let garbage = write_file(dir.path(), "garbage.tui.json", b"{not json");

    let bases = baselines.to_string_lossy().into_owned();
    let arts = artifacts.to_string_lossy().into_owned();
    for (file, expected) in [(&candidate, 1), (&garbage, 3)] {
        let sample = file.to_string_lossy().into_owned();
        let out = run_with_closed_stdout(&[
            "--json",
            "check",
            "--sample",
            &sample,
            "--baselines",
            &bases,
            "--artifacts",
            &arts,
        ]);
        assert_eq!(code(&out), Some(expected), "closed pipe exits {expected}");
        assert_no_panic(&out);
    }
}

/// The child never panics on a closed pipe.
fn assert_no_panic(out: &std::process::Output) {
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked"), "never panics: {stderr}");
}

/// R23: index links resolve to existing evidence from the index parent,
/// wherever the index lives.
#[test]
fn index_links_resolve_from_the_index_parent() {
    let (dir, baselines, artifacts) = temp_store();
    let bytes = state_sample_bytes("rework/indexed", &base_snapshot());
    let sample = write_file(dir.path(), "candidate.tui.json", &bytes);
    assert_eq!(
        check_json(&sample, &baselines, &artifacts).0,
        Some(1),
        "the missing run records evidence"
    );
    std::fs::create_dir_all(artifacts.join("nested")).expect("nested dir creates");

    let outs = [
        dir.path().join("sibling.html"),
        artifacts.join("inside.html"),
        artifacts.join("nested").join("deep.html"),
    ];
    for out in &outs {
        let result = run_cli(&[
            "--json",
            "report",
            "--artifacts",
            &artifacts.to_string_lossy(),
            "--out",
            &out.to_string_lossy(),
        ]);
        assert_eq!(code(&result), Some(0), "{} exits 0", out.display());
        assert_no_color(&result, "report json");
        let value = parse_json(&result);
        assert_eq!(value["status"], "pass");
        assert_eq!(value["output_path"]["encoding"], "utf8");
        assert_index_links_resolve(out);
    }
}

/// Every index href resolves to an existing file from the index parent,
/// uses forward slashes, and names an evidence report.
#[expect(clippy::expect_used, reason = "rework assertions must read")]
fn assert_index_links_resolve(out: &Path) {
    let page = std::fs::read_to_string(out).expect("index reads");
    assert!(page.contains("rework/indexed"), "index names the test");
    let parent = out.parent().expect("index has a parent");
    let mut links = 0;
    for part in page.split("href=\"").skip(1) {
        let href = part.split('"').next().expect("href closes");
        assert!(!href.contains('\\'), "hrefs use forward slashes");
        assert!(href.ends_with("report.html"), "href names evidence");
        let target = parent.join(href);
        assert!(
            target.is_file(),
            "link {href} resolves from {}",
            parent.display()
        );
        links += 1;
    }
    assert!(links > 0, "the index links its records");
}
