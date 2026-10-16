use super::*;
use tuiscotti::VENDORED_FACES;
use tuiscotti::frame::{Cell, Color, Cursor, CursorStyle, Frame, Mods, Rgb, UnderlineStyle};
use tuiscotti::grouped::{ArtifactSet, GroupedStore};
use tuiscotti::render::ansi_dump_with_cursor;
use tuiscotti::snapshot::Status;

#[test]
fn ansi_cursor_round_trips_every_attribute_wide_char_and_cursor_state() {
    let mut frame = Frame::blank(12, 3, prov());
    // Wide CJK + emoji + combining mark + every SGR modifier + indexed/RGB fg/bg/ul + all underline styles
    let mut c0 = Cell::blank(0, 0);
    c0.symbol = "界".into();
    c0.width = 2;
    c0.fg = Color::Indexed(196);
    c0.bg = Color::Rgb(Rgb::new(10, 20, 30));
    c0.underline_color = Color::Indexed(42);
    c0.mods = Mods {
        hidden: true,
        blink: true,
        bold: true,
        dim: true,
        italic: true,
        underline: true,
        underline_style: UnderlineStyle::Curly,
        strikethrough: true,
        reverse: true,
    };
    let mut c1 = c0.clone();
    c1.x = 1;
    c1.symbol = String::new();
    c1.width = 0;
    c1.continuation = true;
    frame.set(c0);
    frame.set(c1);

    let mut c2 = Cell::blank(2, 0);
    c2.symbol = "e\u{0301}".into();
    c2.fg = Color::Rgb(Rgb::new(255, 128, 0));
    c2.underline_color = Color::Rgb(Rgb::new(1, 2, 3));
    c2.mods.underline = true;
    c2.mods.underline_style = UnderlineStyle::Double;
    frame.set(c2);

    for (idx, style) in [
        UnderlineStyle::Single,
        UnderlineStyle::Double,
        UnderlineStyle::Curly,
        UnderlineStyle::Dotted,
        UnderlineStyle::Dashed,
    ]
    .into_iter()
    .enumerate()
    {
        let x = u16::try_from(idx).expect("fits u16");
        let mut cell = Cell::blank(x, 1);
        cell.symbol = "u".into();
        cell.mods.underline = true;
        cell.mods.underline_style = style;
        frame.set(cell);
    }

    // Check hidden cursor + all 6 visible DECSCUSR (CursorStyle, blinking) combinations
    let mut cursors = vec![Cursor::default()];
    for style in [CursorStyle::Block, CursorStyle::Underline, CursorStyle::Bar] {
        for blinking in [true, false] {
            cursors.push(Cursor {
                x: 4,
                y: 1,
                visible: true,
                style,
                blinking,
            });
        }
    }

    for cursor in cursors {
        frame.cursor = cursor;
        frame.validate().expect("fixture frame valid");
        let encoded = ansi_dump_with_cursor(&frame);
        tuiscotti::formats::ansi::assert_normalized_sgr(&encoded)
            .expect("canonical cursor trailer passes assert_normalized_sgr");
        let decoded =
            tuiscotti::formats::ansi::frame_from_ansi(&encoded, prov()).expect("decode succeeds");
        assert_eq!(decoded.cols, frame.cols);
        assert_eq!(decoded.rows, frame.rows);
        assert_eq!(decoded.cells, frame.cells);
        assert_eq!(decoded.cursor, frame.cursor);
        assert_eq!(ansi_dump_with_cursor(&decoded), encoded);
    }
}

#[test]
fn ansi_cursor_rejects_malformed_trailers_and_midstream_escapes() {
    let frame = frame_with("strict");
    let valid = ansi_dump_with_cursor(&frame);
    tuiscotti::formats::ansi::assert_normalized_sgr(&valid).expect("valid passes");

    for bad in [
        format!("{valid}\x1b[?25l"),
        "\x1b[?25l\x1b[0m\n\x1b[?25l".to_string(),
        "hello\x1b[0m\n\x1b[?25h\x1b[0;1H\x1b[1 q".to_string(),
        "hello\x1b[0m\n\x1b[?25h\x1b[1;1H\x1b[7 q".to_string(),
        "hello\x1b[0m\n\x1b[?25h\x1b[1;1H\x1b[0 q".to_string(),
        "hello\x1b[0m\n\x1b[?25x".to_string(),
        "hello\x1b[2J\x1b[0m\n\x1b[?25l".to_string(),
    ] {
        assert!(
            tuiscotti::formats::ansi::assert_normalized_sgr(&bad).is_err(),
            "assert_normalized_sgr must reject {bad:?}"
        );
        assert!(
            tuiscotti::formats::ansi::frame_from_ansi(&bad, prov()).is_err(),
            "frame_from_ansi must reject {bad:?}"
        );
    }
}

#[test]
fn ansi_cursor_store_lifecycle_fast_pass_mismatch_and_stale_cleanup() {
    let dir = tempfile::tempdir().expect("tempdir");
    let approved = dir.path().join("snapshots");
    let actual = dir.path().join("target/tuiscotti/actual");
    let diff = dir.path().join("target/tuiscotti/diff");
    let st = GroupedStore::new(&approved)
        .with_artifacts(ArtifactSet::AnsiCursor)
        .with_actual_root(&actual)
        .with_diff_root(&diff);
    assert_eq!(st.artifacts(), ArtifactSet::AnsiCursor);
    assert_eq!(
        st.expected_root(),
        dir.path().join("target/tuiscotti/expected")
    );

    let name = "holla/parity/discovery/120x40/truecolor";
    let mut f1 = frame_with("minimal baseline");
    f1.cursor = Cursor {
        x: 3,
        y: 2,
        visible: true,
        style: CursorStyle::Bar,
        blinking: true,
    };

    // 1. Missing approval writes actual .ansi + .frame.json + review .png/.html.
    let o1 = st
        .check(name, &f1, &profile(), &VENDORED_FACES, 1.0)
        .expect("missing check");
    assert_eq!(o1.status(), Status::MissingApproval);
    assert!(
        o1.actual.ansi.is_file()
            && o1.actual.frame_json.is_file()
            && o1.actual.png.is_file()
            && o1.actual.html.is_file()
            && !o1.actual.txt.exists()
    );

    // Plant legacy 4-artifact + fidelity files in approved/ to verify accept cleans them up.
    for ext in ["txt", "png", "html", "png.fidelity.json"] {
        let p = approved.join(format!("{name}.{ext}"));
        std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        std::fs::write(&p, b"legacy").expect("write legacy");
    }

    // 2. Accept commits ONLY <name>.ansi and deletes stale .{txt,png,html}.
    st.accept(name).expect("accept");
    assert_eq!(
        tree_files(&approved).expect("tree_files"),
        vec![format!("{name}.ansi")]
    );
    std::fs::remove_file(&o1.actual.png).expect("rm png");
    std::fs::remove_file(&o1.actual.html).expect("rm html");

    // 3. Fast pass: byte-identical .ansi -> Matched, no PNG/HTML rendered!
    let o2 = st
        .check(name, &f1, &profile(), &VENDORED_FACES, 1.0)
        .expect("pass check");
    assert_eq!(o2.status(), Status::Matched);
    assert_eq!(
        (o2.ansi_match, o2.txt_match, o2.html_match),
        (Some(true), None, None)
    );
    assert!(!o2.actual.png.exists() && !o2.actual.html.exists());

    // 4. Cursor-only or cell mismatch decodes approved .ansi, renders expected & diff.
    let mut f2 = frame_with("minimal changed!");
    f2.cursor = f1.cursor;
    let o3 = st
        .check(name, &f2, &profile(), &VENDORED_FACES, 1.0)
        .expect("mismatch check");
    assert_eq!(
        (o3.status(), o3.ansi_match),
        (Status::CellsDiffer, Some(false))
    );
    assert!(
        !o3.outcome.cell_diffs.is_empty() && o3.actual.png.is_file() && o3.actual.html.is_file()
    );
    let exp_png = o3.outcome.expected_png.as_ref().expect("expected_png set");
    let diff_png = o3.outcome.diff_png.as_ref().expect("diff_png set");
    assert!(exp_png.is_file() && exp_png.starts_with(st.expected_root()));
    assert!(diff_png.is_file() && diff_png.starts_with(st.diff_root()));

    // 4b. Re-running a passing check after a mismatch cleans up stale actual/expected/diff PNGs/HTML!
    let o_repass = st
        .check(name, &f1, &profile(), &VENDORED_FACES, 1.0)
        .expect("repass check");
    let fid = o_repass.actual.png.with_extension("png.fidelity.json");
    assert_eq!(o_repass.status(), Status::Matched);
    assert!(!o_repass.actual.png.exists() && !o_repass.actual.html.exists() && !fid.exists());
    assert!(!exp_png.exists() && !diff_png.exists());

    st.check(name, &f2, &profile(), &VENDORED_FACES, 1.0)
        .expect("mismatch check 2");
    verify_accept_matching_and_ci(&st, name, &f2).expect("accept_matching and ci");
}

fn verify_accept_matching_and_ci(
    st: &GroupedStore,
    name: &str,
    f2: &Frame,
) -> Result<(), Box<dyn std::error::Error>> {
    let exact = "ansi_cursor::ansi_cursor_store_lifecycle_fast_pass_mismatch_and_stale_cleanup";
    if tuiscotti::grouped::is_ci_env() {
        let res = st.accept_matching("holla/parity/**");
        assert!(
            res.is_err()
                && res
                    .err()
                    .is_some_and(|e| e.to_string().contains("CI is set"))
        );
        let exe = std::env::current_exe()?;
        let out = std::process::Command::new(exe)
            .args(["--exact", exact, "--nocapture"])
            .env_remove("CI")
            .env("TUISCOTTI_SKIP_CI_SUBTEST", "1")
            .output()?;
        assert!(out.status.success(), "child failed");
        return Ok(());
    }
    if std::env::var_os("TUISCOTTI_SKIP_CI_SUBTEST").is_none() {
        let exe = std::env::current_exe()?;
        let out = std::process::Command::new(exe)
            .args(["--exact", exact, "--nocapture"])
            .env("CI", "1")
            .output()?;
        assert!(out.status.success(), "CI=1 child failed");
    }
    assert!(st.accept_matching("nonexistent/*").is_err());
    assert!(st.accept_matching("holla/parity/*").is_err());
    assert!(st.accept_matching("holla/parity/disc").is_err());
    assert_eq!(st.accept_matching("holla/parity/**")?, vec![name]);
    let o4 = st.check(name, f2, &profile(), &VENDORED_FACES, 1.0)?;
    assert_eq!(o4.status(), Status::Matched);
    assert!(st.accept_matching("holla/parity/**")?.is_empty());
    Ok(())
}

/// Spawn `/bin/sh -c printf ...` in a real PTY and stamp synthetic host paths
/// into `Provenance` to verify end-to-end path sanitization.
#[cfg(feature = "pty")]
fn capture_live_binary(banner: &str) -> Result<Frame, Box<dyn std::error::Error>> {
    use std::time::Duration;
    let script = format!("printf '\\033[1;32m{banner}\\033[0m\\r\\n'; sleep 30");
    let app = tuiscotti::Tui::new(["/bin/sh", "-c", &script])
        .size(40, 6)
        .spawn()?;
    app.get_by_text(banner)
        .expect_visible_within(Duration::from_secs(5))?;
    let screen = app.snapshot()?;
    app.close()?;
    let mut frame = tuiscotti::render::frame_from_screen(&screen, "tuiscotti-default");
    frame.provenance = Provenance::now(
        "tuiscotti-default",
        "/Users/alice/work/repo/tests/visual_baseline.rs",
        vec![
            "/Users/alice/work/repo/target/debug/holla".to_string(),
            "--config=/Users/alice/.config/holla.toml".to_string(),
        ],
    );
    Ok(frame)
}

/// Spawns `/bin/sh -c printf` over a real PTY and verifies `ArtifactSet::AnsiCursor`
/// writes zero host-specific absolute paths across all artifact roots.
#[test]
#[cfg(feature = "pty")]
fn e2e_real_binary_through_ansi_cursor_store_with_zero_absolute_paths() {
    let dir = tempfile::tempdir().expect("tempdir");
    let approved = dir.path().join("snapshots");
    let actual = dir.path().join("target/tuiscotti/actual");
    let diff = dir.path().join("target/tuiscotti/diff");
    let st = GroupedStore::new(&approved)
        .with_artifacts(ArtifactSet::AnsiCursor)
        .with_actual_root(&actual)
        .with_diff_root(&diff);
    let name = "holla/e2e/banner/40x6/truecolor";

    let f_v1 = capture_live_binary("E2E_BANNER_V1").expect("capture v1");
    let o_missing = st
        .check(name, &f_v1, &profile(), &VENDORED_FACES, 1.0)
        .expect("initial check");
    assert_eq!(o_missing.status(), Status::MissingApproval);
    st.accept(name).expect("accept initial");
    assert_eq!(
        tree_files(&approved).expect("tree_files"),
        vec![format!("{name}.ansi")]
    );
    std::fs::remove_file(&o_missing.actual.png).expect("rm png");
    std::fs::remove_file(&o_missing.actual.html).expect("rm html");

    let o_pass1 = st
        .check(name, &f_v1, &profile(), &VENDORED_FACES, 1.0)
        .expect("pass check 1");
    assert_eq!(o_pass1.status(), Status::Matched);
    assert!(!o_pass1.actual.png.exists() && !o_pass1.actual.html.exists());

    let f_v2 = capture_live_binary("E2E_BANNER_V2").expect("capture v2");
    let o_mismatch = st
        .check(name, &f_v2, &profile(), &VENDORED_FACES, 1.0)
        .expect("mismatch check");
    assert_eq!(o_mismatch.status(), Status::CellsDiffer);
    assert!(o_mismatch.actual.png.is_file() && o_mismatch.actual.html.is_file());
    let out = &o_mismatch.outcome;
    assert!(out.expected_png.as_ref().expect("exp").is_file());
    assert!(out.diff_png.as_ref().expect("diff").is_file());

    let tmp_prefix = dir.path().to_string_lossy().into_owned();
    let roots = [
        st.approved_root(),
        st.actual_root(),
        st.expected_root(),
        st.diff_root(),
    ];
    for root in roots {
        for rel in tree_files(root).expect("tree_files") {
            let full = root.join(&rel);
            let text = String::from_utf8_lossy(&std::fs::read(&full).expect("read")).into_owned();
            for bad in ["/Users/alice", &tmp_prefix, "/private/var", "/var/folders"] {
                assert!(!text.contains(bad), "{} leaked {bad:?}", full.display());
            }
        }
    }

    st.accept(name).expect("accept v2");
    let o_pass2 = st
        .check(name, &f_v2, &profile(), &VENDORED_FACES, 1.0)
        .expect("pass check 2");
    assert_eq!(o_pass2.status(), Status::Matched);
}

#[test]
fn ansi_cursor_without_review_evidence_skips_rasterization_and_accepts() {
    let (_dir, st) = tmp_store("no_review_ev").expect("tmp_store");
    let st = st
        .with_artifacts(ArtifactSet::AnsiCursor)
        .with_review_evidence(false);
    assert!(!st.review_evidence());

    let name = "fast/sweep/page";
    let f1 = frame_with("v1");
    let o_miss = st
        .check(name, &f1, &profile(), &VENDORED_FACES, 1.0)
        .expect("missing check");
    assert_eq!(o_miss.status(), Status::MissingApproval);
    assert!(o_miss.actual.ansi.is_file() && o_miss.actual.frame_json.is_file());
    assert!(!o_miss.actual.png.exists() && !o_miss.actual.html.exists());
    st.accept(name).expect("accept v1");

    let f2 = frame_with("v2");
    let o_diff = st
        .check(name, &f2, &profile(), &VENDORED_FACES, 1.0)
        .expect("mismatch check");
    assert_eq!(o_diff.status(), Status::CellsDiffer);
    assert!(!o_diff.outcome.cell_diffs.is_empty());
    assert!(!o_diff.actual.png.exists() && !o_diff.actual.html.exists());
    assert!(o_diff.outcome.expected_png.is_none() && o_diff.outcome.diff_png.is_none());
    if tuiscotti::grouped::is_ci_env() {
        st.accept(name).expect("accept v2");
    } else {
        assert_eq!(
            st.accept_matching("fast/sweep/*").expect("accept_matching"),
            vec![name.to_string()]
        );
    }
    let o_pass = st
        .check(name, &f2, &profile(), &VENDORED_FACES, 1.0)
        .expect("recheck");
    assert_eq!(o_pass.status(), Status::Matched);
}
