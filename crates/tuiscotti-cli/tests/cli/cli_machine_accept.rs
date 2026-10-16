//! CLI: machine mode, `--` passthrough, accept (split from `cli.rs`; shared helpers live in the root).

use super::{code, run_cli, run_cli_cwd, stdout};
use tuiscotti::proto::{EXIT_OP_ERROR, Envelope};

// ---------------------------------------------------------------------------
// CLI: machine mode over stdio
// ---------------------------------------------------------------------------

#[test]
fn cli_machine_mode() {
    let input = "{\"type\":\"version\"}\n{\"type\":\"assert\",\"check\":\"text-equals\",\"actual\":\"a\",\"expected\":\"a\"}\n";
    let out = run_cli(&["machine"], &[], Some(input)).expect("run tuiscotti");
    assert_eq!(
        code(&out).expect("exit code"),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = stdout(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    for line in lines {
        let env: Envelope = serde_json::from_str(line).expect("envelope json");
        assert!(env.ok);
    }
    let out = run_cli(&["machine"], &[], Some("{\"type\":\"version\"}\ngarbage\n"))
        .expect("run tuiscotti");
    assert_eq!(code(&out).expect("exit code"), EXIT_OP_ERROR);
    let text = stdout(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    let second: Envelope = serde_json::from_str(lines[1]).expect("envelope");
    assert!(!second.ok);
}

// ---------------------------------------------------------------------------
// CLI: the parent parser never consumes an argument after `--` (G6).
// ---------------------------------------------------------------------------

#[test]
fn child_receives_double_dash_machine() {
    // Exact regression: with the old hidden `--machine` pre-scan, the parent
    // stripped `--machine` ANYWHERE in argv — including the child's. Now the
    // child receives it byte-exact and machine mode is `tuiscotti machine`.
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("cap");
    let out_arg = out.to_str().expect("utf8 tempdir").to_string();
    let res = run_cli(
        &["capture", "--out", &out_arg, "--", "/bin/echo", "--machine"],
        &[],
        None,
    )
    .expect("run tuiscotti");
    assert_eq!(
        code(&res).expect("exit code"),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&res.stderr)
    );
    let captured = std::fs::read(out.join("stdout.bin")).expect("stdout.bin");
    assert_eq!(captured, b"--machine\n");
    // A literal `--` child argument survives too (clap consumes only the
    // separator; no post-filter may eat child values).
    let cap2 = tmp.path().join("cap2");
    let cap2_arg = cap2.to_str().expect("utf8 tempdir").to_string();
    let res = run_cli(
        &[
            "capture",
            "--out",
            &cap2_arg,
            "--",
            "/bin/echo",
            "--",
            "--machine",
        ],
        &[],
        None,
    )
    .expect("run tuiscotti");
    assert_eq!(code(&res).expect("exit code"), 0);
    let captured = std::fs::read(cap2.join("stdout.bin")).expect("stdout.bin");
    assert_eq!(captured, b"-- --machine\n");
    // A bare `--machine` flag is no longer machine mode: usage error (exit 2).
    let res = run_cli(&["--machine"], &[], None).expect("run tuiscotti");
    assert_eq!(code(&res).expect("exit code"), 2);
    // `machine --help` documents the explicit interface.
    let res = run_cli(&["machine", "--help"], &[], None).expect("run tuiscotti");
    assert_eq!(code(&res).expect("exit code"), 0);
    assert!(
        stdout(&res).contains("stdin"),
        "machine help: {}",
        stdout(&res)
    );
}

// ---------------------------------------------------------------------------
// CLI: accept (explicit per-name approval; frozen roots reject)
// ---------------------------------------------------------------------------

fn accept_frame() -> tuiscotti::Frame {
    let screen = tuiscotti::Screen::blank(30, 6);
    tuiscotti::assert::frame_from_screen(&screen)
}

fn accept_setup() -> std::io::Result<(
    tempfile::TempDir,
    tuiscotti::snapshot::Store,
    tuiscotti::Profile,
    tuiscotti::Frame,
)> {
    let tmp = tempfile::tempdir()?;
    let store = tuiscotti::snapshot::Store::new(&tmp.path().join("shots"));
    let profile = tuiscotti::Profile::default_profile();
    let frame = accept_frame();
    Ok((tmp, store, profile, frame))
}

#[test]
fn cli_accept_round_trip() {
    let (tmp, store, profile, frame) = accept_setup().expect("accept setup");
    let store_dir = tmp.path().join("shots");

    // Missing approval fails closed and advertises the exact CLI invocation.
    let o1 = store
        .check("home", &frame, &profile, &tuiscotti::VENDORED_FACES, 1.0)
        .expect("check");
    assert!(!o1.status.matched());
    let err = o1
        .ensure_matched()
        .expect_err("missing approval must fail closed")
        .to_string();
    assert!(err.contains("tuiscotti accept home"), "{err}");

    // CI=1 refuses before touching classic store.
    let ci_classic = run_cli(
        &[
            "accept",
            "--store",
            store_dir.to_str().expect("utf8 path"),
            "home",
        ],
        &[("CI", "1")],
        None,
    )
    .expect("run tuiscotti under CI=1");
    assert_eq!(code(&ci_classic), Some(EXIT_OP_ERROR));
    assert!(String::from_utf8_lossy(&ci_classic.stderr).contains("CI is set"));
    assert!(!store_dir.join("approved").join("home.png").exists());

    // The CLI blesses exactly one name; the gate then matches.
    let out = run_cli(
        &[
            "accept",
            "--store",
            store_dir.to_str().expect("utf8 path"),
            "home",
        ],
        &[],
        None,
    )
    .expect("run tuiscotti");
    assert_eq!(code(&out).expect("exit code"), 0);
    assert!(store_dir.join("approved").join("home.frame.json").is_file());
    assert!(store_dir.join("approved").join("home.png").is_file());
    let o2 = store
        .check("home", &frame, &profile, &tuiscotti::VENDORED_FACES, 1.0)
        .expect("re-check");
    assert!(o2.status.matched());
    o2.ensure_matched().expect("gate matches after accept");
}

#[test]
fn cli_accept_message_invocation_from_store_root() {
    let (tmp, store, profile, frame) = accept_setup().expect("accept setup");
    let store_dir = tmp.path().join("shots");
    let o3 = store
        .check("away", &frame, &profile, &tuiscotti::VENDORED_FACES, 1.0)
        .expect("check");
    assert!(!o3.status.matched());
    let out = run_cli_cwd(&store_dir, &["accept", "away"]).expect("run tuiscotti in dir");
    assert_eq!(code(&out).expect("exit code"), 0);
    let o4 = store
        .check("away", &frame, &profile, &tuiscotti::VENDORED_FACES, 1.0)
        .expect("re-check");
    assert!(o4.status.matched());
}

#[test]
fn cli_accept_nested_name() {
    let (tmp, store, profile, frame) = accept_setup().expect("accept setup");
    let store_dir = tmp.path().join("shots");
    let o5 = store
        .check(
            "pages/overview",
            &frame,
            &profile,
            &tuiscotti::VENDORED_FACES,
            1.0,
        )
        .expect("check");
    assert!(!o5.status.matched());
    let out = run_cli(
        &[
            "accept",
            "--store",
            store_dir.to_str().expect("utf8 path"),
            "pages/overview",
        ],
        &[],
        None,
    )
    .expect("run tuiscotti");
    assert_eq!(code(&out).expect("exit code"), 0);
    let o6 = store
        .check(
            "pages/overview",
            &frame,
            &profile,
            &tuiscotti::VENDORED_FACES,
            1.0,
        )
        .expect("re-check");
    assert!(o6.status.matched());
}

#[test]
fn cli_accept_invalid() {
    let (tmp, store, profile, frame) = accept_setup().expect("accept setup");
    let store_dir = tmp.path().join("shots");
    let _seed = store
        .check("home", &frame, &profile, &tuiscotti::VENDORED_FACES, 1.0)
        .expect("seed check");
    let seed = run_cli(
        &[
            "accept",
            "--store",
            store_dir.to_str().expect("utf8 path"),
            "home",
        ],
        &[],
        None,
    )
    .expect("seed accept via cli");
    assert_eq!(code(&seed), Some(0));

    let out = run_cli(
        &[
            "accept",
            "--store",
            store_dir.to_str().expect("utf8 path"),
            "never-checked",
        ],
        &[],
        None,
    )
    .expect("run tuiscotti");
    assert_eq!(code(&out).expect("exit code"), EXIT_OP_ERROR);
    assert!(String::from_utf8_lossy(&out.stderr).contains("nothing to accept"));

    let out = run_cli(
        &[
            "accept",
            "--store",
            store_dir.to_str().expect("utf8 path"),
            "../evil",
        ],
        &[],
        None,
    )
    .expect("run tuiscotti");
    assert_eq!(code(&out).expect("exit code"), EXIT_OP_ERROR);

    let out = run_cli(
        &[
            "accept",
            "--store",
            store_dir.to_str().expect("utf8 path"),
            "--all",
            "home",
        ],
        &[],
        None,
    )
    .expect("run tuiscotti");
    assert_eq!(code(&out).expect("exit code"), 2);
}

#[test]
fn cli_accept_rejects_frozen() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let screen = tuiscotti::Screen::blank(30, 6);
    let frozen = tmp.path().join("frozen");
    std::fs::create_dir(&frozen).expect("mkdir");
    std::fs::write(
        frozen.join("home.canonical.txt"),
        tuiscotti::screen::canonical_string(&screen),
    )
    .expect("canonical");
    tuiscotti::assert::check_frozen_snapshot(&frozen, "home", &screen)
        .expect("genuine frozen root");
    let scratch = tuiscotti::snapshot::Store::new(&tmp.path().join("scratch"));
    let profile = tuiscotti::Profile::default_profile();
    let frame = tuiscotti::assert::frame_from_screen(&screen);
    let outcome = scratch
        .check("home", &frame, &profile, &tuiscotti::VENDORED_FACES, 1.0)
        .expect("check");
    let actual_dir = frozen.join("actual");
    std::fs::create_dir(&actual_dir).expect("mkdir");
    std::fs::copy(&outcome.actual_frame, actual_dir.join("home.frame.json")).expect("copy frame");
    std::fs::copy(&outcome.actual_png, actual_dir.join("home.png")).expect("copy png");
    let before = std::fs::read(frozen.join("home.canonical.txt")).expect("read");

    let out = run_cli(
        &[
            "accept",
            "--store",
            frozen.to_str().expect("utf8 path"),
            "home",
        ],
        &[],
        None,
    )
    .expect("run tuiscotti");
    assert_eq!(code(&out).expect("exit code"), EXIT_OP_ERROR);
    assert!(String::from_utf8_lossy(&out.stderr).contains("rejects acceptance"));
    assert!(!frozen.join("approved").exists());
    assert_eq!(
        std::fs::read(frozen.join("home.canonical.txt")).expect("read"),
        before
    );
}

#[test]
fn cli_accept_grouped_and_ansi_cursor_plus_render_from_ansi() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let approved = tmp.path().join("snapshots");
    let actual = tmp.path().join("target/tuiscotti/actual");
    let st = tuiscotti::grouped::GroupedStore::new(&approved)
        .with_artifacts(tuiscotti::grouped::ArtifactSet::AnsiCursor)
        .with_actual_root(&actual);
    let profile = tuiscotti::Profile::default_profile();
    let frame = accept_frame();
    for n in ["suite/a", "suite/b"] {
        let o = st
            .check(n, &frame, &profile, &tuiscotti::VENDORED_FACES, 1.0)
            .expect("check");
        assert!(!o.matched());
    }
    let act_s = actual.to_str().expect("utf8");
    let rev_args = ["review", "--dir", act_s, "--filter", "suite/a"];
    let rev = run_cli(&rev_args, &[], None).expect("rev");
    assert_eq!(code(&rev), Some(tuiscotti::proto::EXIT_VERIFY_FAIL));
    assert!(stdout(&rev).contains("FAIL suite/a") && !stdout(&rev).contains("suite/b"));

    let app_s = approved.to_str().expect("utf8");
    let no_act = run_cli(&["accept", "--store", app_s, "suite/*"], &[], None).expect("no_act");
    assert_eq!(code(&no_act), Some(EXIT_OP_ERROR));

    let acc_args = ["accept", "--store", app_s, "--actual", act_s, "suite/*"];
    let ci_grp = run_cli(&acc_args, &[("CI", "1")], None).expect("ci_grp");
    assert_eq!(code(&ci_grp), Some(EXIT_OP_ERROR));
    assert!(String::from_utf8_lossy(&ci_grp.stderr).contains("CI is set"));
    assert!(!approved.join("suite/a.ansi").exists());

    let st_all = tuiscotti::grouped::GroupedStore::new(&approved).with_actual_root(&actual);
    st_all
        .check("suite/c", &frame, &profile, &tuiscotti::VENDORED_FACES, 1.0)
        .expect("check all");
    let mixed = run_cli(&acc_args, &[], None).expect("mixed");
    assert_eq!(code(&mixed), Some(EXIT_OP_ERROR));
    assert!(String::from_utf8_lossy(&mixed.stderr).contains("mix artifact sets"));
    std::fs::remove_file(actual.join("suite/c.ansi")).expect("rm c");

    let acc = run_cli(&acc_args, &[], None).expect("accept");
    assert_eq!(code(&acc), Some(0));
    assert!(approved.join("suite/a.ansi").is_file() && approved.join("suite/b.ansi").is_file());
    assert!(!approved.join("suite/a.png").exists());

    let in_ansi = approved.join("suite/a.ansi");
    let out_prefix = tmp.path().join("rendered/a");
    let in_s = in_ansi.to_str().expect("utf8");
    let out_s = out_prefix.to_str().expect("utf8");
    let ren_args = [
        "render", "--input", in_s, "--format", "png", "--format", "html", "--out", out_s,
    ];
    let ren = run_cli(&ren_args, &[], None).expect("render from ansi");
    assert_eq!(code(&ren), Some(0));
    assert!(tmp.path().join("rendered/a.png").is_file());
    assert!(tmp.path().join("rendered/a.html").is_file());
}
