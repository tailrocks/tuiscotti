//! CLI render and report: stored-PNG validation and extraction,
//! state-only rendering through the built-in profile, extension
//! rejection, the static evidence index, and `EncodedPath` round-trips.
//!
//! Every test runs the compiled executable in tempdirs with no
//! interactive terminal, and every child runs with `CI` set: none of
//! these flows need approval, so both lanes prove the outcomes under
//! `CI`.

#[path = "support/spawn.rs"]
mod spawn;

#[path = "support/fixtures.rs"]
mod fixtures;

use fixtures::{
    all_files, base_snapshot, candidate_id, pixel_sample_bytes, plain_cell, snapshot_with_cells,
    state_sample_bytes, temp_store, write_baseline_file, write_file,
};
use spawn::{assert_no_color, bin, code, parse_json, run_cli_ci, spawn_locked};

/// Render extracts a stored PNG byte-identically after validating it.
#[test]
fn render_extracts_stored_png() {
    use base64::Engine as _;

    let (dir, _baselines, _artifacts) = temp_store();
    let (test_id, bytes) = pixel_sample_bytes("cli/extract", &base_snapshot());
    let sample = write_file(dir.path(), "candidate.tui.json", &bytes);
    let out = dir.path().join("shot.png");

    let result = run_cli_ci(&[
        "--json",
        "render",
        "--sample",
        &sample.to_string_lossy(),
        "--out",
        &out.to_string_lossy(),
    ]);
    assert_eq!(code(&result), Some(0));
    let value = parse_json(&result);
    assert_eq!(value["command"], "render");
    assert_eq!(value["status"], "pass");
    assert_eq!(value["test_id"], test_id);
    assert_eq!(value["candidate_sha256"], candidate_id(&bytes));
    assert!(value["reason"].is_null(), "extraction reports no profile");
    assert!(value["baseline_sha256"].is_null());
    assert_eq!(value["output_path"]["encoding"], "utf8");

    let stored: serde_json::Value =
        serde_json::from_slice(&bytes).expect("candidate parses as JSON");
    let expected = base64::engine::general_purpose::STANDARD
        .decode(
            stored["image"]["png_base64"]
                .as_str()
                .expect("stored PNG present")
                .as_bytes(),
        )
        .expect("stored PNG decodes");
    assert_eq!(
        std::fs::read(&out).expect("rendered PNG reads"),
        expected,
        "extraction writes the validated stored bytes"
    );
    assert_no_color(&result, "render extract");
}

/// Render replays a state-only sample through the built-in D02 profile
/// and reports that profile in the result.
#[test]
fn render_state_only_reports_builtin_profile() {
    let (dir, _baselines, _artifacts) = temp_store();
    let bytes = state_sample_bytes("cli/state-render", &base_snapshot());
    let sample = write_file(dir.path(), "state.tui.json", &bytes);
    let out = dir.path().join("state.png");

    let result = run_cli_ci(&[
        "--json",
        "render",
        "--sample",
        &sample.to_string_lossy(),
        "--out",
        &out.to_string_lossy(),
    ]);
    assert_eq!(code(&result), Some(0));
    let value = parse_json(&result);
    assert_eq!(value["status"], "pass");
    assert_eq!(
        value["reason"],
        tuiscotti::render_profile::PROFILE_NAME,
        "state-only render reports the built-in profile"
    );

    let png = std::fs::read(&out).expect("rendered PNG reads");
    let parsed = image::load_from_memory(&png).expect("rendered PNG parses");
    assert!(parsed.width() > 0 && parsed.height() > 0);
}

/// Render writes a static `.html` page embedding the image for both
/// stored-image and state-only samples.
#[test]
fn render_html_embeds_image_and_metadata() {
    let (dir, _baselines, _artifacts) = temp_store();
    let (test_id, pixel) = pixel_sample_bytes("cli/html", &base_snapshot());
    let state = state_sample_bytes("cli/html-state", &base_snapshot());
    let pixel_file = write_file(dir.path(), "pixel.tui.json", &pixel);
    let state_file = write_file(dir.path(), "state.tui.json", &state);

    for (file, id, profile) in [
        (&pixel_file, test_id.as_str(), "stored image"),
        (
            &state_file,
            "cli/html-state",
            tuiscotti::render_profile::PROFILE_NAME,
        ),
    ] {
        let out = dir.path().join(format!("{}.html", id.replace('/', "__")));
        let result = run_cli_ci(&[
            "--json",
            "render",
            "--sample",
            &file.to_string_lossy(),
            "--out",
            &out.to_string_lossy(),
        ]);
        assert_eq!(code(&result), Some(0), "{id} renders to HTML");
        let page = std::fs::read_to_string(&out).expect("HTML reads");
        assert!(page.contains(id), "page names the test");
        assert!(page.contains(profile), "page names the profile");
        assert!(
            page.contains("data:image/png;base64,"),
            "page embeds the image"
        );
        assert!(
            !page.contains(dir.path().to_string_lossy().as_ref()),
            "page carries no absolute developer path"
        );
    }
}

/// Output extensions other than `.png` and `.html` are clap usage
/// errors: exit 2 with no JSON result on stdout, an explanation on
/// stderr, and no output file.
#[test]
fn render_rejects_other_extensions() {
    let (dir, _baselines, _artifacts) = temp_store();
    let bytes = state_sample_bytes("cli/ext", &base_snapshot());
    let sample = write_file(dir.path(), "state.tui.json", &bytes);

    for name in ["shot.txt", "shot.jpg", "shot", "shot.PNG"] {
        let out = dir.path().join(name);
        let result = run_cli_ci(&[
            "--json",
            "render",
            "--sample",
            &sample.to_string_lossy(),
            "--out",
            &out.to_string_lossy(),
        ]);
        assert_eq!(code(&result), Some(2), "{name} exits 2");
        assert!(result.stdout.is_empty(), "{name} emits no stdout JSON");
        assert!(!result.stderr.is_empty(), "{name} explains on stderr");
        assert!(!out.exists(), "rejected renders write nothing");
    }
}

/// Corrupt sample content exits 3: garbage JSON is `INVALID_SAMPLE`,
/// matching `check`. Only the `--out` extension rejection above exits 2.
#[test]
fn render_garbage_sample_exits_three_with_invalid_sample() {
    let (dir, _baselines, _artifacts) = temp_store();
    let garbage = write_file(dir.path(), "garbage.tui.json", b"{not json");
    let result = run_cli_ci(&[
        "--json",
        "render",
        "--sample",
        &garbage.to_string_lossy(),
        "--out",
        &dir.path().join("garbage.png").to_string_lossy(),
    ]);
    assert_eq!(code(&result), Some(3));
    let value = parse_json(&result);
    assert_eq!(value["status"], "error");
    assert_eq!(value["error_code"], "INVALID_SAMPLE");
    assert_no_color(&result, "garbage render");
}

/// An oversized sample exits 3 with `RESOURCE_LIMIT`: one byte past
/// `MAX_SAMPLE_FILE_BYTES` never loads.
#[test]
fn render_oversized_sample_exits_three_with_resource_limit() {
    let (dir, _baselines, _artifacts) = temp_store();
    let len = usize::try_from(tuiscotti::sample::MAX_SAMPLE_FILE_BYTES + 1)
        .expect("sample limit fits in usize");
    let big = vec![b'x'; len];
    let sample = write_file(dir.path(), "big.tui.json", &big);
    let result = run_cli_ci(&[
        "--json",
        "render",
        "--sample",
        &sample.to_string_lossy(),
        "--out",
        &dir.path().join("big.png").to_string_lossy(),
    ]);
    assert_eq!(code(&result), Some(3));
    let value = parse_json(&result);
    assert_eq!(value["status"], "error");
    assert_eq!(value["error_code"], "RESOURCE_LIMIT");
    assert_no_color(&result, "oversized render");
}

/// A stored PNG that does not decode exits 3 with `INVALID_IMAGE`.
#[test]
fn render_rejects_undecodable_stored_png() {
    use base64::Engine as _;
    let (dir, _baselines, _artifacts) = temp_store();
    let (_, bytes) = pixel_sample_bytes("cli/bad-png", &base_snapshot());
    let mut value: serde_json::Value =
        serde_json::from_slice(&bytes).expect("candidate parses as JSON");
    value["image"]["png_base64"] =
        serde_json::json!(base64::engine::general_purpose::STANDARD.encode(b"not a png"));
    value["image"]["png_sha256"] =
        serde_json::json!("9c4d3b8b9a4b3c2d1e0f9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a0b9c8");
    value["image"]["rgba_sha256"] =
        serde_json::json!("9c4d3b8b9a4b3c2d1e0f9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a0b9c8");
    let bad = serde_json::to_vec(&value).expect("bad candidate serializes");
    let sample = write_file(dir.path(), "bad.tui.json", &bad);

    let result = run_cli_ci(&[
        "--json",
        "render",
        "--sample",
        &sample.to_string_lossy(),
        "--out",
        &dir.path().join("bad.png").to_string_lossy(),
    ]);
    assert_eq!(code(&result), Some(3));
    let value = parse_json(&result);
    assert_eq!(value["error_code"], "INVALID_IMAGE");
}

/// A missing sample file exits 3 with `IO`.
#[test]
fn render_missing_sample_exits_three_with_io() {
    let (dir, _baselines, _artifacts) = temp_store();
    let result = run_cli_ci(&[
        "--json",
        "render",
        "--sample",
        &dir.path().join("absent.tui.json").to_string_lossy(),
        "--out",
        &dir.path().join("absent.png").to_string_lossy(),
    ]);
    assert_eq!(code(&result), Some(3));
    assert_eq!(parse_json(&result)["error_code"], "IO");
}

/// Non-UTF8 Unix sample paths encode as `unix-bytes-base64` and decode
/// back to the exact native path. The path travels in argv bytes, so no
/// filesystem entry is needed (macOS rejects non-UTF8 names at
/// creation): the missing-file `IO` result still carries the encoded
/// candidate path.
#[cfg(unix)]
#[test]
fn encoded_path_round_trips_non_utf8_unix_paths() {
    use base64::Engine as _;
    use std::ffi::OsString;
    use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};

    let (_dir, baselines, _artifacts) = temp_store();
    let raw = b"/tmp/tuiscotti-cli-\xff.tui.json".to_vec();
    let sample = OsString::from_vec(raw);

    let mut cmd = std::process::Command::new(bin());
    cmd.arg("--json")
        .arg("check")
        .arg("--sample")
        .arg(&sample)
        .arg("--baselines")
        .arg(&baselines)
        .arg("--artifacts")
        .arg(baselines.join("artifacts").as_os_str())
        .env("CI", "true")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let child = spawn_locked(&mut cmd).expect("CLI child spawns");
    let out = child.wait_with_output().expect("CLI child waits");
    assert_eq!(code(&out), Some(3));
    let value = parse_json(&out);
    assert_eq!(value["error_code"], "IO");
    assert_eq!(value["candidate_path"]["encoding"], "unix-bytes-base64");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(
            value["candidate_path"]["value"]
                .as_str()
                .expect("encoded value")
                .as_bytes(),
        )
        .expect("path Base64 decodes");
    assert_eq!(decoded, sample.as_os_str().as_bytes());
}

/// Report builds a static index from saved candidate evidence: two
/// revisions produce two records, baselines stay byte-identical, and
/// the artifact tree gains no files.
#[test]
fn report_indexes_two_revisions_without_touching_baselines() {
    let (dir, baselines, artifacts) = temp_store();
    let first = state_sample_bytes("cli/indexed", &base_snapshot());
    let mut changed = Vec::new();
    for _ in 0..7 {
        changed.push(plain_cell("a", 1));
    }
    changed.push(plain_cell("b", 1));
    let second = state_sample_bytes("cli/indexed", &snapshot_with_cells(changed));
    let first_file = write_file(dir.path(), "v1.tui.json", &first);
    let second_file = write_file(dir.path(), "v2.tui.json", &second);

    let check = |file: &std::path::Path| {
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
    assert_eq!(parse_json(&check(&second_file))["status"], "missing");
    write_baseline_file(&baselines, "cli/indexed", &first);
    assert_eq!(code(&check(&first_file)), Some(0));
    assert_eq!(parse_json(&check(&second_file))["status"], "different");
    let mut baseline = baselines.join("cli/indexed");
    baseline.set_extension("tui.json");
    assert_eq!(
        std::fs::read(&baseline).expect("baseline reads"),
        first,
        "report changes no baseline"
    );
    let before = all_files(&artifacts);

    let index = dir.path().join("index.html");
    let out = run_cli_ci(&[
        "--json",
        "report",
        "--artifacts",
        &artifacts.to_string_lossy(),
        "--out",
        &index.to_string_lossy(),
    ]);
    assert_eq!(code(&out), Some(0));
    let value = parse_json(&out);
    assert_eq!(value["status"], "pass");
    assert!(value["test_id"].is_null());
    assert_eq!(value["output_path"]["encoding"], "utf8");

    let page = std::fs::read_to_string(&index).expect("index reads");
    assert!(page.contains("cli/indexed"), "index names the test");
    assert!(page.contains("missing"), "index records the missing run");
    assert!(page.contains("state"), "index records the state run");
    assert_eq!(all_files(&artifacts), before, "report writes no evidence");
    assert_no_color(&out, "report");
}

/// Report over an empty artifact root succeeds with zero records.
#[test]
fn report_empty_artifacts_succeeds() {
    let (dir, _baselines, artifacts) = temp_store();
    std::fs::create_dir_all(&artifacts).expect("artifact root creates");
    let index = dir.path().join("index.html");
    let out = run_cli_ci(&[
        "--json",
        "report",
        "--artifacts",
        &artifacts.to_string_lossy(),
        "--out",
        &index.to_string_lossy(),
    ]);
    assert_eq!(code(&out), Some(0));
    assert_eq!(parse_json(&out)["status"], "pass");
    let page = std::fs::read_to_string(&index).expect("index reads");
    assert!(page.contains("records: 0"));
}

/// Report over a missing artifact root exits 3 with `IO`.
#[test]
fn report_missing_artifacts_exits_three_with_io() {
    let (dir, _baselines, _artifacts) = temp_store();
    let out = run_cli_ci(&[
        "--json",
        "report",
        "--artifacts",
        &dir.path().join("absent").to_string_lossy(),
        "--out",
        &dir.path().join("index.html").to_string_lossy(),
    ]);
    assert_eq!(code(&out), Some(3));
    assert_eq!(parse_json(&out)["error_code"], "IO");
}
