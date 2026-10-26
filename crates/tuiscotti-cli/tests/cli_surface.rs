//! C17 CLI surface: help, unknown commands, missing arguments, removed
//! aliases, stdout/stderr separation, broken pipes, and `NO_COLOR`.
//!
//! Every test runs the compiled executable with no interactive
//! terminal and with `CI` set: none of these flows need approval, so
//! both lanes prove the surface behavior under `CI`. Argument-parsing
//! failures exit 2 with clap's stderr text and no JSON result on
//! stdout.

#[path = "support/spawn.rs"]
mod spawn;

use std::process::Stdio;

use spawn::{assert_no_color, bin, code, parse_json, run_cli_ci, spawn_locked};

/// `--help` lists exactly the four Section 16 commands.
#[test]
fn help_lists_four_commands() {
    let out = run_cli_ci(&["--help"]);
    assert_eq!(code(&out), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    for command in ["check", "accept", "render", "report"] {
        assert!(stdout.contains(command), "help names {command}:\n{stdout}");
    }
    assert!(out.stderr.is_empty(), "help is quiet on stderr");
    assert_no_color(&out, "help");
}

/// Every command documents its Section 16 flags.
#[test]
fn each_command_help_names_its_flags() {
    for (command, flags) in [
        ("check", &["--sample", "--baselines", "--artifacts"][..]),
        (
            "accept",
            &[
                "--candidate",
                "--candidate-sha256",
                "--expect-baseline",
                "--baselines",
            ][..],
        ),
        ("render", &["--sample", "--out"][..]),
        ("report", &["--artifacts", "--out"][..]),
    ] {
        let out = run_cli_ci(&[command, "--help"]);
        assert_eq!(code(&out), Some(0), "{command} help exits 0");
        let stdout = String::from_utf8_lossy(&out.stdout);
        for flag in flags {
            assert!(
                stdout.contains(flag),
                "{command} help names {flag}:\n{stdout}"
            );
        }
        assert_no_color(&out, &format!("{command} help"));
    }
}

/// Unknown commands exit 2 with no JSON on stdout.
#[test]
fn unknown_command_is_usage_error_without_json() {
    let out = run_cli_ci(&["frobnicate"]);
    assert_eq!(code(&out), Some(2));
    assert!(out.stdout.is_empty(), "usage errors emit no stdout");
    assert!(!out.stderr.is_empty(), "usage errors explain on stderr");
    assert_no_color(&out, "unknown command");
}

/// Removed live-execution surfaces stay removed: every previous command
/// name is an exit-2 usage error, never a silent skip.
#[test]
fn removed_commands_stay_removed() {
    for name in [
        "attach", "capture", "diff", "doctor", "import", "init", "input", "inspect", "machine",
        "observe", "record", "review", "schema", "session", "start", "stop", "trace",
    ] {
        let out = run_cli_ci(&[name]);
        assert_eq!(code(&out), Some(2), "{name} exits 2");
        assert!(out.stdout.is_empty(), "{name} emits no stdout");
    }
}

/// Missing required arguments exit 2 with no JSON result.
#[test]
fn missing_arguments_are_usage_errors_without_json() {
    for argv in [
        vec!["check"],
        vec!["check", "--sample", "x"],
        vec!["accept", "--candidate", "x"],
        vec!["render", "--sample", "x"],
        vec!["report"],
    ] {
        let out = run_cli_ci(&argv);
        assert_eq!(code(&out), Some(2), "{argv:?} exits 2");
        assert!(out.stdout.is_empty(), "{argv:?} emits no stdout JSON");
        assert!(!out.stderr.is_empty(), "{argv:?} explains on stderr");
    }
}

/// `accept` takes no `--all`, glob, or CI override: unknown flags are
/// exit-2 usage errors.
#[test]
fn accept_takes_no_batch_or_override_flags() {
    for argv in [
        vec!["accept", "--all"],
        vec!["accept", "--glob", "x"],
        vec!["accept", "--ci-ok"],
    ] {
        let out = run_cli_ci(&argv);
        assert_eq!(code(&out), Some(2), "{argv:?} exits 2");
        assert!(out.stdout.is_empty(), "{argv:?} emits no stdout");
    }
}

/// `--json` results parse as one object; failures split streams: the
/// JSON result on stdout, the diagnostic on stderr, no escapes anywhere.
#[test]
fn json_error_splits_stdout_and_stderr() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let sample = dir.path().join("missing.tui.json");
    let out = run_cli_ci(&[
        "--json",
        "check",
        "--sample",
        &sample.to_string_lossy(),
        "--baselines",
        &dir.path().join("b").to_string_lossy(),
        "--artifacts",
        &dir.path().join("a").to_string_lossy(),
    ]);
    assert_eq!(code(&out), Some(3));
    let value = parse_json(&out);
    assert_eq!(value["schema"], "tuiscotti.cli.v1");
    assert_eq!(value["command"], "check");
    assert_eq!(value["status"], "error");
    assert!(!out.stderr.is_empty(), "diagnostic goes to stderr");
    assert_no_color(&out, "json error");
}

/// A closed stdout preserves the operation result, never a panic:
/// pass exits 0, mismatch exits 1, operational errors exit 3.
#[test]
fn broken_pipe_preserves_the_operation_result() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    std::fs::write(dir.path().join("empty.json"), b"{}").expect("fixture writes");
    let check = |sample: &str| {
        vec![
            "--json".to_string(),
            "check".to_string(),
            "--sample".to_string(),
            sample.to_string(),
            "--baselines".to_string(),
            dir.path().join("b").to_string_lossy().into_owned(),
            "--artifacts".to_string(),
            dir.path().join("a").to_string_lossy().into_owned(),
        ]
    };
    let cases: Vec<(Vec<String>, i32)> = vec![
        (vec!["--help".to_string()], 0),
        // Garbage bytes fail sample validation: an error outcome.
        (check(&dir.path().join("empty.json").to_string_lossy()), 3),
        // A missing sample file is an I/O error outcome.
        (
            check(&dir.path().join("absent.tui.json").to_string_lossy()),
            3,
        ),
        (
            vec![
                "report".to_string(),
                "--artifacts".to_string(),
                dir.path().to_string_lossy().into_owned(),
                "--out".to_string(),
                dir.path().join("index.html").to_string_lossy().into_owned(),
            ],
            0,
        ),
    ];
    for (argv, expected) in cases {
        let owned: Vec<&str> = argv.iter().map(String::as_str).collect();
        let mut child = std::process::Command::new(bin());
        child
            .args(&owned)
            .env("CI", "true")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = spawn_locked(&mut child).expect("CLI child spawns");
        // Drop the read end before the child writes: the next stdout
        // write fails with EPIPE (Rust ignores SIGPIPE).
        drop(child.stdout.take());
        let out = child.wait_with_output().expect("CLI child waits");
        assert_eq!(
            code(&out),
            Some(expected),
            "{owned:?} exits {expected} on a closed pipe"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !stderr.contains("panicked"),
            "{owned:?} never panics: {stderr}"
        );
    }
}
