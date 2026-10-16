use super::*;
use tuiscotti::VENDORED_FACES;
use tuiscotti::grouped::validate_name;
use tuiscotti::snapshot::Status;

#[test]
fn name_validation_rejects_unsafe_names() {
    for bad in [
        "",
        "/absolute/path",
        "a/../b",
        "..",
        "../escape",
        "a//b",
        "/",
        "a/",
        "a\\b",
        "C:\\snapshots\\x",
        "a/./b",
    ] {
        assert!(validate_name(bad).is_err(), "must reject {bad:?}");
        let (_dir, st) = tmp_store("validate").expect("tmp_store succeeds");
        let err = st
            .check(bad, &frame_with("x"), &profile(), &VENDORED_FACES, 1.0)
            .expect_err(
                "st .check(bad, &frame_with(\"x\"), &profile(), &VENDORED_FACES, 1.0) is an error",
            )
            .to_string();
        assert!(err.contains("invalid snapshot name"), "{err}");
        assert!(st.accept(bad).is_err(), "accept must reject {bad:?}");
    }
    for good in ["home", "a/b/c", "showcase/pages/overview_120x40_truecolor"] {
        validate_name(good).expect("validate_name(good) succeeds");
    }
}

#[test]
fn cell_change_fails_ansi_gate_as_cells_differ() {
    if rerun_without_ci() {
        return;
    }
    let name = "flows/checkout/step1";
    let (_dir, st) = tmp_store("cells").expect("tmp_store succeeds");
    st.check(
        name,
        &frame_with("before"),
        &profile(),
        &VENDORED_FACES,
        1.0,
    )
    .expect("check");
    st.accept(name).expect("st.accept(name) succeeds");
    let outcome = st
        .check(
            name,
            &frame_with("after!"),
            &profile(),
            &VENDORED_FACES,
            1.0,
        )
        .expect("check");
    assert_eq!(outcome.status(), Status::CellsDiffer);
    assert_eq!(outcome.ansi_match, Some(false));
    assert_eq!(outcome.txt_match, Some(false));
    assert_eq!(outcome.html_match, Some(false));
    // Pixel mismatch wrote a diff PNG under the DIFF root (not approved/).
    let diff = outcome.outcome.diff_png.clone().expect("diff_png");
    assert!(diff.exists());
    assert!(diff.starts_with(st.diff_root()), "{}", diff.display());
    let err = outcome
        .ensure_matched()
        .expect_err("outcome.ensure_matched() is an error")
        .to_string();
    assert!(err.contains("cells-differ"), "{err}");
    assert!(err.contains("tuiscotti accept"), "{err}");
}

#[test]
fn style_only_change_keeps_txt_equal() {
    if rerun_without_ci() {
        return;
    }
    let name = "flows/checkout/step2";
    let (_dir, st) = tmp_store("style").expect("tmp_store succeeds");
    st.check(
        name,
        &frame_with("same text"),
        &profile(),
        &VENDORED_FACES,
        1.0,
    )
    .expect("check");
    st.accept(name).expect("st.accept(name) succeeds");
    let mut styled = frame_with("same text");
    styled.cells[0].mods.bold = true;
    let outcome = st
        .check(name, &styled, &profile(), &VENDORED_FACES, 1.0)
        .expect("check");
    assert_eq!(outcome.status(), Status::CellsDiffer);
    assert_eq!(outcome.ansi_match, Some(false), "SGR run changed");
    assert_eq!(outcome.txt_match, Some(true), "plain text unchanged");
}

#[test]
fn png_pixel_gate_honors_threshold() {
    if rerun_without_ci() {
        return;
    }
    let name = "pages/overview";
    let (_dir, st) = tmp_store("pixels").expect("tmp_store succeeds");
    let frame = frame_with("pixel gate");
    st.check(name, &frame, &profile(), &VENDORED_FACES, 1.0)
        .expect("check");
    st.accept(name).expect("st.accept(name) succeeds");
    // Sabotage ONLY the approved PNG (render of a different screen): the
    // byte gates still pass, isolating the decoded-pixel gate.
    let other_png = {
        let profile = profile();
        let mut renderer = profile.renderer(&VENDORED_FACES).expect("renderer");
        renderer
            .render(&frame_with("pixel gate!"))
            .expect("render")
            .png
    };
    std::fs::write(st.approved_root().join(format!("{name}.png")), &other_png)
        .expect("write approved png");

    let mut renderer = profile().renderer(&VENDORED_FACES).expect("renderer");
    let strict = st
        .check_with(&mut renderer, name, &frame, 1.0)
        .expect("check_with");
    assert_eq!(strict.status(), Status::PixelsDiffer);
    assert_eq!(strict.ansi_match, Some(true));
    assert_eq!(strict.html_match, Some(true));
    let score = strict.outcome.pixel_score.expect("pixel_score");
    assert!(score < 1.0, "score {score}");
    assert!(strict.outcome.diff_png.as_ref().expect("diff_png").exists());

    // Same comparison passes under a threshold at/below the score.
    let relaxed = st
        .check_with(&mut renderer, name, &frame, 0.0)
        .expect("check_with");
    assert_eq!(relaxed.status(), Status::Matched);
    assert_eq!(relaxed.outcome.pixel_score, Some(score));
}

#[test]
fn missing_single_artifact_fails_closed() {
    if rerun_without_ci() {
        return;
    }
    let name = "a/b";
    let (_dir, st) = tmp_store("partial").expect("tmp_store succeeds");
    st.check(name, &frame_with("x"), &profile(), &VENDORED_FACES, 1.0)
        .expect("check");
    st.accept(name).expect("st.accept(name) succeeds");
    std::fs::remove_file(st.approved_root().join(format!("{name}.txt"))).expect("remove .txt");
    let outcome = st
        .check(name, &frame_with("x"), &profile(), &VENDORED_FACES, 1.0)
        .expect("check");
    assert_eq!(outcome.status(), Status::MissingApproval);
    assert!(
        outcome.outcome.note.contains(".txt"),
        "{}",
        outcome.outcome.note
    );
}

#[test]
fn corrupt_approved_png_is_an_explicit_error() {
    if rerun_without_ci() {
        return;
    }
    let name = "a/b";
    let (_dir, st) = tmp_store("corruptpng").expect("tmp_store succeeds");
    st.check(name, &frame_with("x"), &profile(), &VENDORED_FACES, 1.0)
        .expect("check");
    st.accept(name).expect("st.accept(name) succeeds");
    std::fs::write(st.approved_root().join(format!("{name}.png")), b"not a png")
        .expect("write corrupt png");
    let mut renderer = profile().renderer(&VENDORED_FACES).expect("renderer");
    let err = st
        .check_with(&mut renderer, name, &frame_with("x"), 1.0)
        .expect_err("corrupt png must error")
        .to_string();
    assert!(err.contains("cannot decode expected PNG"), "{err}");
}

#[test]
fn ansi_cursor_hidden_cursor_no_spurious_diff_and_legacy_trailer_note() {
    use tuiscotti::frame::{Cursor, CursorStyle};
    use tuiscotti::grouped::ArtifactSet;
    if rerun_without_ci() {
        return;
    }
    let (_dir, st) = tmp_store("cur_legacy").expect("tmp_store");
    let st = st.with_artifacts(ArtifactSet::AnsiCursor);
    let name = "page/cur";

    let mut base = frame_with("hello");
    base.cursor = Cursor {
        x: 9,
        y: 3,
        visible: false,
        style: CursorStyle::Bar,
        blinking: false,
    };
    let legacy_ansi = tuiscotti::render::ansi_dump(&base);
    let app_path = st.approved_root().join(format!("{name}.ansi"));
    std::fs::create_dir_all(app_path.parent().expect("parent")).expect("mkdir");
    std::fs::write(&app_path, &legacy_ansi).expect("write legacy");

    let o_legacy = st
        .check(name, &base, &profile(), &VENDORED_FACES, 1.0)
        .expect("check legacy");
    assert_eq!(o_legacy.status(), Status::CellsDiffer);
    assert!(
        o_legacy
            .outcome
            .note
            .contains("approved file predates the cursor trailer; re-accept"),
        "note: {}",
        o_legacy.outcome.note
    );
    assert_eq!(o_legacy.outcome.cell_diff_total, 0);

    st.accept(name).expect("accept canonical");
    let mut changed = base;
    changed.cells[2].symbol = "X".to_string();
    let o_diff = st
        .check(name, &changed, &profile(), &VENDORED_FACES, 1.0)
        .expect("check changed");
    assert_eq!(o_diff.status(), Status::CellsDiffer);
    assert_eq!(o_diff.outcome.cell_diff_total, 1);
    assert_eq!(
        (
            o_diff.outcome.cell_diffs[0].x,
            o_diff.outcome.cell_diffs[0].y
        ),
        (2, 0)
    );
}

#[test]
fn accept_and_accept_all_refuse_in_ci() {
    if !tuiscotti::grouped::is_ci_env() {
        let exe = std::env::current_exe().expect("current_exe");
        let out = std::process::Command::new(exe)
            .args([
                "--exact",
                "gates::accept_and_accept_all_refuse_in_ci",
                "--nocapture",
            ])
            .env("CI", "1")
            .output()
            .expect("spawn CI=1 subtest");
        assert!(
            out.status.success(),
            "CI=1 subtest failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let classic = tuiscotti::snapshot::Store::new(&dir.path().join("classic"));
    drop(
        classic
            .check("home", &frame_with("ci"), &profile(), &VENDORED_FACES, 1.0)
            .expect("classic check"),
    );
    let err_c = classic
        .accept("home")
        .expect_err("Store::accept must refuse in CI")
        .to_string();
    assert!(err_c.contains("CI is set"), "{err_c}");
    assert!(!classic.root().join("approved/home.png").exists());

    let grouped = GroupedStore::new(&dir.path().join("grouped"))
        .with_artifacts(tuiscotti::grouped::ArtifactSet::AnsiCursor);
    grouped
        .check("p/one", &frame_with("ci"), &profile(), &VENDORED_FACES, 1.0)
        .expect("grouped check");
    for err in [
        grouped.accept("p/one").expect_err("accept in CI"),
        grouped.accept_all().expect_err("accept_all in CI"),
        grouped
            .accept_matching("**")
            .expect_err("accept_matching in CI"),
    ] {
        assert!(err.to_string().contains("CI is set"), "{err}");
    }
    assert!(
        tree_files(grouped.approved_root())
            .expect("tree")
            .is_empty()
    );
}
