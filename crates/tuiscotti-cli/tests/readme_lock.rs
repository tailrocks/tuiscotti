//! README lock (C10): every README code fence compiles and the documented
//! CLI surface matches `--help`.
//!
//! Each test mirrors one README fence statement-for-statement (same calls,
//! temp dirs / runnable programs substituted for placeholders), or asserts
//! one documented CLI command/flag appears in the real help text. If the
//! README drifts from the code again, this file goes red first.

use std::path::PathBuf;
use std::process::Command;

use ratatui::widgets::Paragraph;
use tuiscotti::Provenance;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tuiscotti"))
}

fn help(args: &[&str]) -> std::io::Result<String> {
    let out = Command::new(bin()).args(args).output()?;
    assert!(out.status.success(), "help {args:?} failed");
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn home_frame() -> tuiscotti::Frame {
    tuiscotti::ratatui::draw_frame(
        120,
        40,
        Provenance::now("tuiscotti-default", "home", vec![]),
        |f| f.render_widget(Paragraph::new("home"), f.area()),
    )
}

fn rerun_without_ci() -> bool {
    if !tuiscotti::grouped::is_ci_env() {
        return false;
    }
    let exact = std::thread::current()
        .name()
        .unwrap_or_default()
        .to_string();
    assert!(!exact.is_empty(), "libtest sets test thread name");
    let exe = std::env::current_exe().unwrap_or_default();
    assert!(!exe.as_os_str().is_empty(), "current_exe failed");
    let out = Command::new(exe)
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

#[path = "readme_lock/fences.rs"]
mod fences;

#[path = "readme_lock/api.rs"]
mod api;

#[path = "readme_lock/grouped.rs"]
mod grouped;

#[path = "readme_lock/cli.rs"]
mod cli;
