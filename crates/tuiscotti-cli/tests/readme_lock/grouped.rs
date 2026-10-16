//! README grouped store + fallback chain (split from `readme_lock.rs`; shared helpers live in the root).

use super::home_frame;
use tuiscotti::{Profile, VENDORED_FACES};

// ---------------------------------------------------------------------------
// Grouped fences: check_with + report_with + accept_all + root overrides.
// ---------------------------------------------------------------------------

#[test]
fn readme_grouped_store() {
    fn check_page(
        store: &tuiscotti::grouped::GroupedStore,
        profile: &tuiscotti::Profile,
        frame: &tuiscotti::Frame,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut renderer = profile.renderer(&tuiscotti::VENDORED_FACES)?;
        let outcome = store.check_with(&mut renderer, "pages/overview", frame, 1.0)?;
        outcome.ensure_matched()?;
        store.report_with(&mut renderer, 1.0, "my suite")?;
        Ok(())
    }

    let tmp = tempfile::tempdir().expect("tempdir");
    let store = tuiscotti::grouped::GroupedStore::new(&tmp.path().join("snapshots"));
    // Documented defaults live outside the approved tree.
    assert_eq!(store.actual_root(), tmp.path().join("snapshots.actual"));
    assert_eq!(store.diff_root(), tmp.path().join("snapshots.diff"));
    assert_eq!(
        store.report_path(),
        tmp.path().join("snapshots.actual").join("report.html")
    );
    // Overrides resolve.
    let custom = tuiscotti::grouped::GroupedStore::new(&tmp.path().join("s2"))
        .with_actual_root(&tmp.path().join("a"))
        .with_diff_root(&tmp.path().join("d"))
        .with_report_path(&tmp.path().join("r.html"));
    assert_eq!(custom.actual_root(), tmp.path().join("a"));
    assert_eq!(custom.diff_root(), tmp.path().join("d"));
    assert_eq!(custom.report_path(), tmp.path().join("r.html"));

    let profile = Profile::default_profile();
    let frame = home_frame();
    let first = store
        .check("pages/overview", &frame, &profile, &VENDORED_FACES, 1.0)
        .expect("check");
    assert_eq!(first.status(), tuiscotti::snapshot::Status::MissingApproval);

    // Bless recursively from Rust (the documented accept_all fence).
    let accepted = store.accept_all().expect("accept all");
    assert_eq!(accepted, vec!["pages/overview".to_string()]);
    // Approved tree holds exactly the four artifacts.
    for ext in ["ansi", "txt", "png", "html"] {
        assert!(
            tmp.path()
                .join(format!("snapshots/pages/overview.{ext}"))
                .is_file(),
            "missing approved .{ext}"
        );
    }
    assert!(
        !tmp.path()
            .join("snapshots/pages/overview.frame.json")
            .exists(),
        "approved tree must not hold .frame.json"
    );
    check_page(&store, &profile, &frame).expect("check page");
    assert!(store.report_path().is_file());
}

// ---------------------------------------------------------------------------
// Font-fallback fence: FallbackFace + Renderer::with_fallbacks.
// ---------------------------------------------------------------------------

#[test]
fn readme_fallback_chain() {
    fn custom_chain(
        profile: &tuiscotti::Profile,
    ) -> Result<tuiscotti::Renderer, Box<dyn std::error::Error>> {
        let chain = [tuiscotti::FallbackFace {
            bytes: tuiscotti::VENDORED_SYMBOLS2_FONT,
            sha256: tuiscotti::VENDORED_SYMBOLS2_FONT_SHA256,
            desc: "my extra symbols",
        }];
        Ok(tuiscotti::render::Renderer::with_fallbacks(
            profile,
            &tuiscotti::VENDORED_FACES,
            &chain,
        )?)
    }

    let profile = Profile::default_profile();
    let mut r = custom_chain(&profile).expect("custom chain");
    let rendered = r.render(&home_frame()).expect("render");
    assert!(!rendered.png.is_empty());
    // A wrong pin refuses to render.
    let bad = tuiscotti::render::Renderer::with_fallbacks(
        &profile,
        &VENDORED_FACES,
        &[tuiscotti::FallbackFace {
            bytes: tuiscotti::VENDORED_SYMBOLS2_FONT,
            sha256: &"0".repeat(64),
            desc: "bad pin",
        }],
    );
    assert!(bad.is_err());
}

#[test]
#[cfg(feature = "pty")]
fn readme_workflow_2_to_files_ansi_cursor() {
    use std::time::{Duration, Instant};
    use tuiscotti::grouped::{ArtifactSet, GroupedStore};
    use tuiscotti::render;
    use tuiscotti::tui::{CancelToken, MouseButton, MouseMods, Tui};

    let tmp = tempfile::tempdir().expect("tempdir");
    // Enable DEC 1003 + 1006 in the child so hover and press-and-hold succeed.
    let session = Tui::new([
        "/bin/sh",
        "-c",
        "printf '\\033[?1003h\\033[?1006hReady\\n'; sleep 30",
    ])
    .size(40, 8)
    .env_clear(true)
    .env("TERM", "xterm-256color")
    .env("TUISCOTTI_FIXED_TIME", "2026-01-01T00:00:00Z")
    .spawn()
    .expect("spawn shell");

    session
        .get_by_text("Ready")
        .any()
        .expect_visible()
        .expect("expect Ready");
    let cancel = CancelToken::new();
    session
        .wait_stable_quiet(
            Instant::now() + Duration::from_secs(2),
            Duration::from_millis(50),
            &cancel,
        )
        .expect("wait stable quiet");

    session
        .mouse_move(10, 5, MouseMods::NONE)
        .expect("mouse_move");
    session
        .mouse_down(MouseButton::Left, 10, 5, MouseMods::NONE)
        .expect("mouse_down");
    session
        .mouse_up(MouseButton::Left, 10, 5, MouseMods::NONE)
        .expect("mouse_up");

    let frame =
        render::frame_from_screen(&session.snapshot().expect("snapshot"), "tuiscotti-default");
    let store = GroupedStore::new(&tmp.path().join("snapshots/visual"))
        .with_artifacts(ArtifactSet::AnsiCursor)
        .with_actual_root(&tmp.path().join("target/tuiscotti/actual"))
        .with_expected_root(&tmp.path().join("target/tuiscotti/expected"))
        .with_diff_root(&tmp.path().join("target/tuiscotti/diff"));
    let profile = Profile::default_profile();
    let first = store
        .check(
            "dashboard/hover_40x8",
            &frame,
            &profile,
            &VENDORED_FACES,
            1.0,
        )
        .expect("first check");
    assert_eq!(first.status(), tuiscotti::snapshot::Status::MissingApproval);
    if tuiscotti::grouped::is_ci_env() {
        let err = store
            .accept_matching("dashboard/*")
            .expect_err("accept_matching must refuse under CI");
        assert!(err.to_string().contains("CI is set"), "{err}");
        session.close().expect("close");
        let exe = std::env::current_exe().expect("current_exe");
        let out = std::process::Command::new(exe)
            .args([
                "--exact",
                "grouped::readme_workflow_2_to_files_ansi_cursor",
                "--nocapture",
            ])
            .env_remove("CI")
            .output()
            .expect("spawn child without CI");
        assert!(
            out.status.success(),
            "child failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        return;
    }
    store
        .accept_matching("dashboard/*")
        .expect("accept matching");
    store
        .check(
            "dashboard/hover_40x8",
            &frame,
            &profile,
            &VENDORED_FACES,
            1.0,
        )
        .expect("second check")
        .ensure_matched()
        .expect("matched after accept");

    session.close().expect("close");
}
