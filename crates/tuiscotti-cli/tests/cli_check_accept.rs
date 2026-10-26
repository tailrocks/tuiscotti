//! CLI forms of C09 through C12: missing baselines, approval by
//! digests, the two-revision evidence lifecycle, and frozen/CI roots.
//!
//! Every approval runs through the compiled executable in tempdirs.
//! `CI` values live only on spawned child environments, never in the
//! runner process. Approval (`accept`) success paths strip `CI` because
//! production refuses approval under `CI` by design; every `check`
//! assertion runs with `CI` explicitly set, so both lanes prove the
//! substantive outcomes under `CI`, and exactly one test
//! (`c12_ci_refuses_accept_but_not_check`) proves the refusal itself.

#[path = "support/spawn.rs"]
mod spawn;

#[path = "support/fixtures.rs"]
mod fixtures;

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use fixtures::{
    all_files, base_snapshot, candidate_id, pixel_sample_bytes, plain_cell, snapshot_with_cells,
    state_sample_bytes, temp_store, write_baseline_file, write_file,
};
use spawn::{assert_no_color, code, parse_json, run_cli_ci, run_cli_env};

/// C09 CLI: a missing baseline exits 1 with status `missing` and writes
/// no baseline; the explicit accept then succeeds and check passes.
#[test]
fn c09_missing_baseline_then_accept_then_pass() {
    let (dir, baselines, artifacts) = temp_store();
    let snapshot = base_snapshot();
    let bytes = state_sample_bytes("cli/missing", &snapshot);
    let sample = write_file(dir.path(), "candidate.tui.json", &bytes);

    // The missing check runs under `CI`: frozen comparison needs no
    // approval, so the outcome must hold in both lanes.
    let out = run_cli_ci(&[
        "--json",
        "check",
        "--sample",
        &sample.to_string_lossy(),
        "--baselines",
        &baselines.to_string_lossy(),
        "--artifacts",
        &artifacts.to_string_lossy(),
    ]);
    assert_eq!(code(&out), Some(1));
    let value = parse_json(&out);
    assert_eq!(value["schema"], "tuiscotti.cli.v1");
    assert_eq!(value["command"], "check");
    assert_eq!(value["status"], "missing");
    assert_eq!(value["test_id"], "cli/missing");
    assert_eq!(value["reason"], "missing");
    assert_eq!(value["candidate_sha256"], candidate_id(&bytes));
    assert_eq!(value["baseline_sha256"], "missing");
    assert!(value["error_code"].is_null());
    assert_no_color(&out, "missing baseline");

    assert!(all_files(&baselines).is_empty(), "check writes no baseline");
    let report = PathBuf::from(
        value["output_path"]["value"]
            .as_str()
            .expect("report path is text"),
    );
    assert!(report.is_file(), "missing evidence keeps its report");

    let digest = candidate_id(&bytes);
    let out = run_cli_env(
        &[
            "--json",
            "accept",
            "--candidate",
            &sample.to_string_lossy(),
            "--candidate-sha256",
            &digest,
            "--expect-baseline",
            "missing",
            "--baselines",
            &baselines.to_string_lossy(),
        ],
        &[],
        true,
    );
    assert_eq!(code(&out), Some(0));
    let value = parse_json(&out);
    assert_eq!(value["status"], "pass");
    assert_eq!(value["test_id"], "cli/missing");
    assert_eq!(value["baseline_sha256"], digest);

    let out = run_cli_ci(&[
        "--json",
        "check",
        "--sample",
        &sample.to_string_lossy(),
        "--baselines",
        &baselines.to_string_lossy(),
        "--artifacts",
        &artifacts.to_string_lossy(),
    ]);
    assert_eq!(code(&out), Some(0));
    assert_eq!(parse_json(&out)["status"], "pass");
}

/// C10 CLI: approval by digests with frame and image agreement.
#[test]
fn c10_accept_approves_matching_digests() {
    let (dir, baselines, artifacts) = temp_store();
    let (test_id, bytes) = pixel_sample_bytes("cli/approve", &base_snapshot());
    let sample = write_file(dir.path(), "candidate.tui.json", &bytes);
    let digest = candidate_id(&bytes);

    let out = run_cli_env(
        &[
            "--json",
            "accept",
            "--candidate",
            &sample.to_string_lossy(),
            "--candidate-sha256",
            &digest,
            "--expect-baseline",
            "missing",
            "--baselines",
            &baselines.to_string_lossy(),
        ],
        &[],
        true,
    );
    assert_eq!(code(&out), Some(0));
    let value = parse_json(&out);
    assert_eq!(value["test_id"], test_id);
    assert_eq!(value["candidate_sha256"], digest);
    assert_eq!(value["baseline_sha256"], digest);

    let out = run_cli_ci(&[
        "--json",
        "check",
        "--sample",
        &sample.to_string_lossy(),
        "--baselines",
        &baselines.to_string_lossy(),
        "--artifacts",
        &artifacts.to_string_lossy(),
    ]);
    assert_eq!(code(&out), Some(0));
    assert_eq!(parse_json(&out)["status"], "pass");
}

/// Baselines hold no approved or evidence files: only the stable
/// approval lock may persist (it is kept by design, never unlinked).
fn assert_no_baseline_writes(baselines: &Path) {
    let files = all_files(baselines);
    assert!(
        files.iter().all(|name| name == ".tuiscotti-approval.lock"),
        "refusals write no baselines or evidence, got {files:?}"
    );
}

/// C10 CLI: a wrong candidate digest exits 3 with `STALE_CANDIDATE` and
/// writes no baseline or evidence.
#[test]
fn c10_stale_candidate_digest_writes_nothing() {
    let (dir, baselines, _artifacts) = temp_store();
    let bytes = state_sample_bytes("cli/stale-candidate", &base_snapshot());
    let sample = write_file(dir.path(), "candidate.tui.json", &bytes);

    let out = run_cli_env(
        &[
            "--json",
            "accept",
            "--candidate",
            &sample.to_string_lossy(),
            "--candidate-sha256",
            &"f".repeat(64),
            "--expect-baseline",
            "missing",
            "--baselines",
            &baselines.to_string_lossy(),
        ],
        &[],
        true,
    );
    assert_eq!(code(&out), Some(3));
    let value = parse_json(&out);
    assert_eq!(value["status"], "error");
    assert_eq!(value["error_code"], "STALE_CANDIDATE");
    assert_no_baseline_writes(&baselines);
}

/// C10 CLI: missing, malformed, and zero digests are clap usage
/// errors: exit 2 with no JSON result on stdout, an explanation on
/// stderr, and nothing written.
#[test]
fn c10_malformed_digests_are_argument_errors() {
    let (dir, baselines, _artifacts) = temp_store();
    let bytes = state_sample_bytes("cli/bad-digest", &base_snapshot());
    let sample = write_file(dir.path(), "candidate.tui.json", &bytes);
    let digest = candidate_id(&bytes);
    let zeros = "0".repeat(64);
    let upper = "F".repeat(64);

    for (candidate_sha256, expect_baseline) in [
        ("xyz", "missing"),
        ("abc", "missing"),
        (zeros.as_str(), "missing"),
        (digest.as_str(), "xyz"),
        (digest.as_str(), zeros.as_str()),
        (digest.as_str(), upper.as_str()),
        (digest.as_str(), "MISSING"),
        ("", "missing"),
        (digest.as_str(), ""),
    ] {
        // Digest-shape validation is a clap parser: it rejects before the
        // production CI gate, so the exit-2 outcome must hold under `CI`.
        let out = run_cli_ci(&[
            "--json",
            "accept",
            "--candidate",
            &sample.to_string_lossy(),
            "--candidate-sha256",
            candidate_sha256,
            "--expect-baseline",
            expect_baseline,
            "--baselines",
            &baselines.to_string_lossy(),
        ]);
        assert_eq!(
            code(&out),
            Some(2),
            "digest pair ({candidate_sha256}, {expect_baseline}) exits 2"
        );
        assert!(
            out.stdout.is_empty(),
            "usage errors emit no stdout JSON for ({candidate_sha256}, {expect_baseline})"
        );
        assert!(
            !out.stderr.is_empty(),
            "usage errors explain on stderr for ({candidate_sha256}, {expect_baseline})"
        );
        assert!(
            all_files(&baselines).is_empty(),
            "malformed digests write nothing"
        );
    }
}

/// C10 CLI: frame and image data must agree; a tampered snapshot
/// with a recomputed digest fails the approval re-render with
/// `INCONSISTENT_SAMPLE` at exit 3 and writes no baseline.
#[test]
fn c10_tampered_image_rejects_on_frame_image_agreement() {
    let (dir, baselines, artifacts) = temp_store();
    let (test_id, bytes) = pixel_sample_bytes("cli/tamper", &base_snapshot());
    let mut value: serde_json::Value =
        serde_json::from_slice(&bytes).expect("candidate parses as JSON");
    value["snapshot"]["cells"][0]["text"] = serde_json::json!("b");
    let tampered = serde_json::to_vec(&value).expect("tampered candidate serializes");
    let sample = write_file(dir.path(), "tampered.tui.json", &tampered);

    let out = run_cli_env(
        &[
            "--json",
            "accept",
            "--candidate",
            &sample.to_string_lossy(),
            "--candidate-sha256",
            &candidate_id(&tampered),
            "--expect-baseline",
            "missing",
            "--baselines",
            &baselines.to_string_lossy(),
        ],
        &[],
        true,
    );
    assert_eq!(code(&out), Some(3));
    let value = parse_json(&out);
    assert_eq!(value["error_code"], "INCONSISTENT_SAMPLE");
    assert_eq!(
        value["baseline_sha256"], "missing",
        "a re-render failure with no baseline observes the missing literal"
    );
    assert_no_baseline_writes(&baselines);

    // The same tampered bytes against a frozen original baseline fail
    // the `check` comparison under `CI`: the transaction outcome
    // (different state) holds where approval itself is forbidden.
    write_baseline_file(&baselines, &test_id, &bytes);
    let out = run_cli_ci(&[
        "--json",
        "check",
        "--sample",
        &sample.to_string_lossy(),
        "--baselines",
        &baselines.to_string_lossy(),
        "--artifacts",
        &artifacts.to_string_lossy(),
    ]);
    assert_eq!(code(&out), Some(1));
    let value = parse_json(&out);
    assert_eq!(value["status"], "different");
    assert_eq!(value["reason"], "state");
}

/// C11 CLI: two revisions stay available; a stale expected digest
/// fails with `STALE_BASELINE` and leaves the baseline untouched.
#[test]
fn c11_two_revisions_with_stale_rejection() {
    let (dir, baselines, artifacts) = temp_store();
    let first = state_sample_bytes("cli/revisions", &base_snapshot());
    let mut changed = Vec::new();
    for _ in 0..7 {
        changed.push(plain_cell("a", 1));
    }
    changed.push(plain_cell("b", 1));
    let second = state_sample_bytes("cli/revisions", &snapshot_with_cells(changed));
    let first_file = write_file(dir.path(), "v1.tui.json", &first);
    let second_file = write_file(dir.path(), "v2.tui.json", &second);
    let first_digest = candidate_id(&first);
    let second_digest = candidate_id(&second);

    let accept = |file: &Path, digest: &str, expect: &str| {
        run_cli_env(
            &[
                "--json",
                "accept",
                "--candidate",
                &file.to_string_lossy(),
                "--candidate-sha256",
                digest,
                "--expect-baseline",
                expect,
                "--baselines",
                &baselines.to_string_lossy(),
            ],
            &[],
            true,
        )
    };
    // Frozen comparison needs no approval: every `check` here runs
    // under `CI`, while the `accept` steps stay on the non-CI path
    // production requires.
    let check = |file: &Path| {
        run_cli_ci(&[
            "--json",
            "check",
            "--sample",
            &file.to_string_lossy(),
            "--baselines",
            &baselines.to_string_lossy(),
            "--artifacts",
            &artifacts.to_string_lossy(),
        ])
    };

    assert_eq!(
        code(&accept(&first_file, &first_digest, "missing")),
        Some(0)
    );
    assert_eq!(parse_json(&check(&first_file))["status"], "pass");

    let out = check(&second_file);
    assert_eq!(code(&out), Some(1));
    let value = parse_json(&out);
    assert_eq!(value["status"], "different");
    assert_eq!(value["reason"], "state");

    assert_eq!(
        code(&accept(&second_file, &second_digest, &first_digest)),
        Some(0)
    );
    assert_eq!(parse_json(&check(&second_file))["status"], "pass");

    let out = accept(&first_file, &first_digest, &first_digest);
    assert_eq!(code(&out), Some(3));
    assert_eq!(parse_json(&out)["error_code"], "STALE_BASELINE");
    let mut baseline = baselines.join("cli/revisions");
    baseline.set_extension("tui.json");
    assert_eq!(
        std::fs::read(&baseline).expect("baseline reads"),
        second,
        "stale approval leaves the baseline untouched"
    );
}

/// Lowercase SHA-256 hex over bytes, matching the library digest shape.
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest as _, Sha256};
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 15) as usize] as char);
    }
    out
}

/// C11 CLI, image self-check: a PNG-only change (equal state, equal
/// dimensions, different pixels) fails with reason `pixels`, so the
/// suite fails on image change.
#[test]
fn c11_pixel_only_change_fails_with_pixels_reason() {
    use base64::Engine as _;

    let (dir, baselines, artifacts) = temp_store();
    let (test_id, first) = pixel_sample_bytes("cli/pixels-change", &base_snapshot());
    write_baseline_file(&baselines, &test_id, &first);

    let mut value: serde_json::Value =
        serde_json::from_slice(&first).expect("candidate parses as JSON");
    let encoded = value["image"]["png_base64"]
        .as_str()
        .expect("stored PNG present");
    let raw = base64::engine::general_purpose::STANDARD
        .decode(encoded.as_bytes())
        .expect("stored PNG decodes");
    let mut image = image::load_from_memory(&raw)
        .expect("stored PNG parses")
        .to_rgba8();
    let first_pixel = image.get_pixel(0, 0);
    image.put_pixel(
        0,
        0,
        image::Rgba([
            first_pixel[0] ^ 1,
            first_pixel[1],
            first_pixel[2],
            first_pixel[3],
        ]),
    );
    let mut changed_png = Vec::new();
    image::DynamicImage::ImageRgba8(image)
        .write_to(
            &mut std::io::Cursor::new(&mut changed_png),
            image::ImageFormat::Png,
        )
        .expect("changed PNG encodes");
    let changed_rgba = image::load_from_memory(&changed_png)
        .expect("changed PNG parses")
        .to_rgba8()
        .into_raw();
    value["image"]["png_base64"] =
        serde_json::json!(base64::engine::general_purpose::STANDARD.encode(&changed_png));
    value["image"]["png_sha256"] = serde_json::json!(sha256_hex(&changed_png));
    value["image"]["rgba_sha256"] = serde_json::json!(sha256_hex(&changed_rgba));
    let second = serde_json::to_vec(&value).expect("changed candidate serializes");
    let second_file = write_file(dir.path(), "changed.tui.json", &second);

    let out = run_cli_ci(&[
        "--json",
        "check",
        "--sample",
        &second_file.to_string_lossy(),
        "--baselines",
        &baselines.to_string_lossy(),
        "--artifacts",
        &artifacts.to_string_lossy(),
    ]);
    assert_eq!(code(&out), Some(1));
    let value = parse_json(&out);
    assert_eq!(value["status"], "different");
    assert_eq!(value["reason"], "pixels");
}

/// C12 CLI: a read-only baseline root still checks, but any approval
/// fails with `IO` and writes nothing (no baseline, no lock file).
#[test]
fn c12_readonly_baseline_root_checks_but_never_approves() {
    let (dir, baselines, artifacts) = temp_store();
    let bytes = state_sample_bytes("cli/readonly", &base_snapshot());
    write_baseline_file(&baselines, "cli/readonly", &bytes);
    let candidate = write_file(dir.path(), "candidate.tui.json", &bytes);
    let digest = candidate_id(&bytes);

    std::fs::set_permissions(&baselines, std::fs::Permissions::from_mode(0o555))
        .expect("baseline root locks");
    // Writability probe: a privileged runner ignores permission bits,
    // so skip instead of asserting a refusal that cannot happen.
    let probe = baselines.join(".writability-probe");
    let writable = std::fs::write(&probe, b"probe").is_ok();
    if std::fs::remove_file(&probe).is_err() {
        // Best-effort cleanup only; the tempdir owns the file either way.
    }
    if writable {
        std::fs::set_permissions(&baselines, std::fs::Permissions::from_mode(0o755))
            .expect("baseline root restores");
        eprintln!("skipped: runner writes through read-only permissions");
        return;
    }

    let out = run_cli_ci(&[
        "--json",
        "check",
        "--sample",
        &candidate.to_string_lossy(),
        "--baselines",
        &baselines.to_string_lossy(),
        "--artifacts",
        &artifacts.to_string_lossy(),
    ]);
    assert_eq!(code(&out), Some(0), "frozen roots still check");
    assert_eq!(parse_json(&out)["status"], "pass");

    let out = run_cli_env(
        &[
            "--json",
            "accept",
            "--candidate",
            &candidate.to_string_lossy(),
            "--candidate-sha256",
            &digest,
            "--expect-baseline",
            &digest,
            "--baselines",
            &baselines.to_string_lossy(),
        ],
        &[],
        true,
    );
    assert_eq!(code(&out), Some(3));
    assert_eq!(parse_json(&out)["error_code"], "IO");
    assert!(
        !baselines.join(".tuiscotti-approval.lock").exists(),
        "refused approval leaves no lock file"
    );

    std::fs::set_permissions(&baselines, std::fs::Permissions::from_mode(0o755))
        .expect("baseline root restores");
}

/// C12 CLI: a read-only artifact root fails the evidence write with
/// `IO` at exit 3 while the baseline stays untouched.
#[test]
fn c12_readonly_artifact_root_fails_evidence_with_io() {
    let (dir, baselines, artifacts) = temp_store();
    let bytes = state_sample_bytes("cli/readonly-artifacts", &base_snapshot());
    let sample = write_file(dir.path(), "candidate.tui.json", &bytes);
    std::fs::create_dir_all(&artifacts).expect("artifact root creates");

    std::fs::set_permissions(&artifacts, std::fs::Permissions::from_mode(0o555))
        .expect("artifact root locks");
    let probe = artifacts.join(".writability-probe");
    let writable = std::fs::write(&probe, b"probe").is_ok();
    if std::fs::remove_file(&probe).is_err() {
        // Best-effort cleanup only; the tempdir owns the file either way.
    }
    if writable {
        std::fs::set_permissions(&artifacts, std::fs::Permissions::from_mode(0o755))
            .expect("artifact root restores");
        eprintln!("skipped: runner writes through read-only permissions");
        return;
    }

    let out = run_cli_ci(&[
        "--json",
        "check",
        "--sample",
        &sample.to_string_lossy(),
        "--baselines",
        &baselines.to_string_lossy(),
        "--artifacts",
        &artifacts.to_string_lossy(),
    ]);
    assert_eq!(code(&out), Some(3));
    assert_eq!(parse_json(&out)["error_code"], "IO");

    std::fs::set_permissions(&artifacts, std::fs::Permissions::from_mode(0o755))
        .expect("artifact root restores");
}

/// C12 CLI: any set `CI` value refuses approval in the child process
/// with `CI_APPROVAL_FORBIDDEN` and writes nothing. This is the single
/// dedicated test that proves the public refusal; every other approval
/// test exercises the transaction outcomes through frozen fixtures plus
/// `check`, which needs no approval and runs under `CI`.
#[test]
fn c12_ci_refuses_accept_but_not_check() {
    let (dir, baselines, artifacts) = temp_store();
    let bytes = state_sample_bytes("cli/ci", &base_snapshot());
    write_baseline_file(&baselines, "cli/ci", &bytes);
    let digest = candidate_id(&bytes);
    let candidate = write_file(dir.path(), "candidate.tui.json", &bytes);

    for ci in ["1", "true", ""] {
        let out = run_cli_env(
            &[
                "--json",
                "accept",
                "--candidate",
                &candidate.to_string_lossy(),
                "--candidate-sha256",
                &digest,
                "--expect-baseline",
                &digest,
                "--baselines",
                &baselines.to_string_lossy(),
            ],
            &[("CI", ci)],
            false,
        );
        assert_eq!(code(&out), Some(3), "CI={ci:?} refuses approval");
        let value = parse_json(&out);
        assert_eq!(value["error_code"], "CI_APPROVAL_FORBIDDEN");
        assert!(
            value["baseline_sha256"].is_null(),
            "a CI refusal carries no observed baseline"
        );
        assert!(
            !baselines.join(".tuiscotti-approval.lock").exists(),
            "CI refusal leaves no lock file"
        );
    }
    assert_eq!(
        std::fs::read({
            let mut path = baselines.join("cli/ci");
            path.set_extension("tui.json");
            path
        })
        .expect("baseline reads"),
        bytes,
        "CI refusals change no baseline"
    );

    // Where approval is forbidden, frozen fixtures plus `check` carry
    // the substantive transaction outcomes under `CI`.
    assert_check_matrix_under_ci(dir.path(), &baselines, &artifacts, &candidate);
}

/// Frozen fixtures plus `check` under `CI`: pass, a state difference,
/// a missing baseline, and two operational errors. This is the
/// substantive transaction coverage for the lane where production
/// forbids approval itself.
fn assert_check_matrix_under_ci(dir: &Path, baselines: &Path, artifacts: &Path, candidate: &Path) {
    let check_ci = |sample: &Path| {
        run_cli_env(
            &[
                "--json",
                "check",
                "--sample",
                &sample.to_string_lossy(),
                "--baselines",
                &baselines.to_string_lossy(),
                "--artifacts",
                &artifacts.to_string_lossy(),
            ],
            &[("CI", "true")],
            false,
        )
    };
    let out = check_ci(candidate);
    assert_eq!(code(&out), Some(0), "check passes under CI");
    assert_eq!(parse_json(&out)["status"], "pass");

    let mut changed = Vec::new();
    for _ in 0..7 {
        changed.push(plain_cell("a", 1));
    }
    changed.push(plain_cell("b", 1));
    let other = state_sample_bytes("cli/ci", &snapshot_with_cells(changed));
    let other_file = write_file(dir, "changed.tui.json", &other);
    let out = check_ci(&other_file);
    assert_eq!(code(&out), Some(1), "check differs under CI");
    let value = parse_json(&out);
    assert_eq!(value["status"], "different");
    assert_eq!(value["reason"], "state");

    let absent_id = state_sample_bytes("cli/ci-absent", &base_snapshot());
    let absent_file = write_file(dir, "absent-id.tui.json", &absent_id);
    let out = check_ci(&absent_file);
    assert_eq!(code(&out), Some(1), "check misses under CI");
    let value = parse_json(&out);
    assert_eq!(value["status"], "missing");
    assert_eq!(value["reason"], "missing");

    let garbage = write_file(dir, "garbage.tui.json", b"{not json");
    let out = check_ci(&garbage);
    assert_eq!(code(&out), Some(3), "invalid samples error under CI");
    assert_eq!(parse_json(&out)["error_code"], "INVALID_SAMPLE");

    let missing = dir.join("no-such-sample.tui.json");
    let out = check_ci(&missing);
    assert_eq!(code(&out), Some(3), "missing files error under CI");
    assert_eq!(parse_json(&out)["error_code"], "IO");
}

/// Paths with spaces work for every store and file argument.
#[test]
fn paths_with_spaces_work_end_to_end() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let spaced = dir.path().join("dir with spaces");
    let baselines = spaced.join("base lines");
    let artifacts = spaced.join("arti facts");
    std::fs::create_dir_all(&baselines).expect("spaced roots create");
    let bytes = state_sample_bytes("cli/spaces", &base_snapshot());
    let sample = write_file(&spaced, "my candidate.tui.json", &bytes);
    let digest = candidate_id(&bytes);

    let out = run_cli_ci(&[
        "--json",
        "check",
        "--sample",
        &sample.to_string_lossy(),
        "--baselines",
        &baselines.to_string_lossy(),
        "--artifacts",
        &artifacts.to_string_lossy(),
    ]);
    assert_eq!(code(&out), Some(1));
    let value = parse_json(&out);
    assert_eq!(value["status"], "missing");
    assert_eq!(value["candidate_path"]["encoding"], "utf8");
    assert!(
        value["candidate_path"]["value"]
            .as_str()
            .expect("path value")
            .contains(' '),
        "encoded paths keep spaces"
    );

    let out = run_cli_env(
        &[
            "--json",
            "accept",
            "--candidate",
            &sample.to_string_lossy(),
            "--candidate-sha256",
            &digest,
            "--expect-baseline",
            "missing",
            "--baselines",
            &baselines.to_string_lossy(),
        ],
        &[],
        true,
    );
    assert_eq!(code(&out), Some(0));
}

/// Invalid sample data exits 3: unparseable files are `INVALID_SAMPLE`,
/// and unknown schemas are `UNSUPPORTED_SCHEMA`.
#[test]
fn invalid_sample_data_exits_three_with_codes() {
    let (dir, baselines, artifacts) = temp_store();
    let garbage = write_file(dir.path(), "garbage.tui.json", b"{not json");
    let (_, pixel) = pixel_sample_bytes("cli/schema", &base_snapshot());
    let mut value: serde_json::Value =
        serde_json::from_slice(&pixel).expect("candidate parses as JSON");
    value["schema"] = serde_json::json!("tuiscotti.sample.v9");
    let future = serde_json::to_vec(&value).expect("future schema serializes");
    let future_file = write_file(dir.path(), "future.tui.json", &future);

    for (file, error_code) in [
        (garbage, "INVALID_SAMPLE"),
        (future_file, "UNSUPPORTED_SCHEMA"),
    ] {
        let out = run_cli_ci(&[
            "--json",
            "check",
            "--sample",
            &file.to_string_lossy(),
            "--baselines",
            &baselines.to_string_lossy(),
            "--artifacts",
            &artifacts.to_string_lossy(),
        ]);
        assert_eq!(code(&out), Some(3), "{error_code} exits 3");
        let value = parse_json(&out);
        assert_eq!(value["status"], "error");
        assert_eq!(value["error_code"], error_code);
        assert_no_color(&out, error_code);
    }
}
