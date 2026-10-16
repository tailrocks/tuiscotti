use super::*;
use tuiscotti::VENDORED_FACES;
use tuiscotti::grouped::GroupedStore;
use tuiscotti::snapshot::Status;

#[test]
fn accept_all_walks_nested_names_recursively() {
    let (_dir, st) = tmp_store("acceptall").expect("tmp_store succeeds");
    let names = [
        "showcase/pages/overview_120x40_truecolor",
        "showcase/pages/detail_120x40_truecolor",
        "showcase/modals/confirm",
        "home",
    ];
    for (i, name) in names.iter().enumerate() {
        st.check(
            name,
            &frame_with(&format!("screen {i}")),
            &profile(),
            &VENDORED_FACES,
            1.0,
        )
        .expect("st.check( name, &frame_with(&format!(\"screen {i}\")), &profile(), &VENDORED_FACES, 1.0, ) succeeds");
    }
    let mut listed = st.actual_names().expect("st.actual_names() succeeds");
    assert_eq!(listed.len(), 4, "{listed:?}");
    let accepted = st.accept_all().expect("st.accept_all() succeeds");
    assert_eq!(accepted, listed);
    assert_eq!(
        st.approved_names().expect("st.approved_names() succeeds"),
        listed
    );
    for name in &names {
        let outcome = st
            .check(
                name,
                &frame_with("placeholder"),
                &profile(),
                &VENDORED_FACES,
                1.0,
            )
            .expect("st .check( name, &frame_with(\"placeholder\"), &profile(), &VENDORED_FACES, 1.0, ) succeeds");
        assert_eq!(outcome.status(), Status::CellsDiffer, "{name} was approved");
    }
    listed.sort();
    let mut sorted = names.to_vec();
    sorted.sort_unstable();
    assert_eq!(listed, sorted);
}

#[test]
fn report_reverifies_nested_actuals_outside_approved_tree() {
    let (_dir, st) = tmp_store("report").expect("tmp_store succeeds");
    st.check(
        "suite/one",
        &frame_with("alpha"),
        &profile(),
        &VENDORED_FACES,
        1.0,
    )
    .expect("st.check( \"suite/one\", &frame_with(\"alpha\"), &profile(), &VENDORED_FACES, 1.0, ) succeeds");
    st.accept("suite/one")
        .expect("st.accept(\"suite/one\") succeeds");
    st.check(
        "suite/nested/two",
        &frame_with("beta"),
        &profile(),
        &VENDORED_FACES,
        1.0,
    )
    .expect("st.check( \"suite/nested/two\", &frame_with(\"beta\"), &profile(), &VENDORED_FACES, 1.0, ) succeeds");
    let report = st
        .report(&profile(), &VENDORED_FACES, 1.0, "grouped suite")
        .expect("st .report(&profile(), &VENDORED_FACES, 1.0, \"grouped suite\") succeeds");
    assert_eq!(report.outcomes.len(), 2);
    assert_eq!(report.failed(), 1, "`suite/nested/two` has no approval");
    assert!(report.path.exists());
    // Default report path: under the actual scratch root, NEVER approved/.
    assert!(report.path.starts_with(st.actual_root()));
    assert!(!report.path.starts_with(st.approved_root()));
    let html = std::fs::read_to_string(&report.path)
        .expect("std::fs::read_to_string(&report.path) succeeds");
    assert!(html.contains("suite/one — matched"), "{html}");
    assert!(
        html.contains("suite/nested/two — missing-approval"),
        "{html}"
    );
    assert!(
        !html.contains("data:image/png;base64,"),
        "suite report must not embed PNGs"
    );
    // The approved tree is still exactly the four artifacts of `suite/one`.
    assert_eq!(
        tree_files(st.approved_root()).expect("tree_files succeeds"),
        vec![
            "suite/one.ansi".to_string(),
            "suite/one.html".to_string(),
            "suite/one.png".to_string(),
            "suite/one.txt".to_string(),
        ]
    );
}

#[test]
fn report_path_is_configurable() {
    let (dir, st) = tmp_store("reportcfg").expect("tmp_store succeeds");
    let custom = dir.path().join("target").join("grouped-report.html");
    let st = st.with_report_path(&custom);
    st.check("a/b", &frame_with("x"), &profile(), &VENDORED_FACES, 1.0)
        .expect("st.check(\"a/b\", &frame_with(\"x\"), &profile(), &VENDORED_FACES, 1.0) succeeds");
    let report = st
        .report(&profile(), &VENDORED_FACES, 1.0, "custom path")
        .expect("st .report(&profile(), &VENDORED_FACES, 1.0, \"custom path\") succeeds");
    assert_eq!(report.path, custom);
    assert!(custom.exists());
}

#[test]
fn custom_actual_and_diff_roots_are_honored() {
    let dir = tempfile::tempdir().expect("tempfile::tempdir() succeeds");
    let approved = dir.path().join("approved-tree");
    let st = GroupedStore::new(&approved)
        .with_actual_root(&dir.path().join("scratch/actual"))
        .with_diff_root(&dir.path().join("scratch/diff"));
    let name = "deep/nested/name";
    let outcome = st
        .check(name, &frame_with("roots"), &profile(), &VENDORED_FACES, 1.0)
        .expect(
            "st .check(name, &frame_with(\"roots\"), &profile(), &VENDORED_FACES, 1.0) succeeds",
        );
    assert!(
        outcome
            .actual
            .ansi
            .starts_with(dir.path().join("scratch/actual"))
    );
    assert_eq!(outcome.status(), Status::MissingApproval);
    st.accept(name).expect("st.accept(name) succeeds");
    assert!(approved.join(format!("{name}.ansi")).exists());
    let outcome = st
        .check(
            name,
            &frame_with("changed"),
            &profile(),
            &VENDORED_FACES,
            1.0,
        )
        .expect("st .check( name, &frame_with(\"changed\"), &profile(), &VENDORED_FACES, 1.0, ) succeeds");
    let diff = outcome
        .outcome
        .diff_png
        .expect("outcome.outcome.diff_png is some");
    assert!(
        diff.starts_with(dir.path().join("scratch/diff")),
        "{}",
        diff.display()
    );
}

#[test]
fn default_roots_are_siblings_of_approved() {
    let dir = tempfile::tempdir().expect("tempfile::tempdir() succeeds");
    let st = GroupedStore::new(&dir.path().join("snapshots"));
    assert_eq!(st.actual_root(), &dir.path().join("snapshots.actual"));
    assert_eq!(st.diff_root(), &dir.path().join("snapshots.diff"));
    assert_eq!(
        st.report_path(),
        dir.path().join("snapshots.actual").join("report.html")
    );
}

#[test]
fn accept_without_actuals_is_an_error() {
    let (_dir, st) = tmp_store("noactual").expect("tmp_store succeeds");
    let err = st
        .accept("never/checked")
        .expect_err("st.accept(\"never/checked\") is an error")
        .to_string();
    assert!(err.contains("nothing to accept"), "{err}");
}

#[test]
fn check_with_reuses_one_renderer_across_nested_checks() {
    let (_dir, st) = tmp_store("checkwith").expect("tmp_store succeeds");
    let profile = profile();
    let mut renderer = profile
        .renderer(&VENDORED_FACES)
        .expect("profile.renderer(&VENDORED_FACES) succeeds");
    let frame = frame_with("cached grouped");
    let o1 = st
        .check_with(&mut renderer, "g/s", &frame, 1.0)
        .expect("st.check_with(&mut renderer, \"g/s\", &frame, 1.0) succeeds");
    assert_eq!(o1.status(), Status::MissingApproval);
    st.accept("g/s").expect("st.accept(\"g/s\") succeeds");
    let o2 = st
        .check_with(&mut renderer, "g/s", &frame, 1.0)
        .expect("st.check_with(&mut renderer, \"g/s\", &frame, 1.0) succeeds");
    assert_eq!(o2.status(), Status::Matched);
    assert_eq!(o2.outcome.pixel_score, Some(1.0));
    o2.ensure_matched().expect("o2.ensure_matched() succeeds");
}

#[test]
fn html_artifact_is_a_standalone_colored_render() {
    let name = "docs/preview";
    let (_dir, st) = tmp_store("htmlview").expect("tmp_store succeeds");
    let outcome = st
        .check(
            name,
            &frame_with("standalone"),
            &profile(),
            &VENDORED_FACES,
            1.0,
        )
        .expect("st .check( name, &frame_with(\"standalone\"), &profile(), &VENDORED_FACES, 1.0, ) succeeds");
    let html = std::fs::read_to_string(&outcome.actual.html)
        .expect("std::fs::read_to_string(&outcome.actual.html) succeeds");
    assert!(html.contains("<svg"), "{html}");
    assert!(html.contains("data:image/png;base64,"), "{html}");
    assert!(
        html.contains("<script type=\"application/json\">"),
        "{html}"
    );
    assert!(html.contains("<title>docs/preview</title>"), "{html}");
    let body = html
        .split("<body>")
        .nth(1)
        .expect("html.split(\"<body>\").nth(1) is some");
    let img = body.find("<img ").expect("png img");
    assert!(
        !body[..img].contains("<details"),
        "PNG must be the primary visual, not nested under details: {html}"
    );
}

#[test]
fn accept_rejects_ghost_tampered_and_stale_run_candidates() {
    if tuiscotti::grouped::is_ci_env() {
        let exe = std::env::current_exe().expect("current_exe");
        let out = std::process::Command::new(exe)
            .args([
                "--exact",
                "accept_report::accept_rejects_ghost_tampered_and_stale_run_candidates",
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

    let (_dir, st) = tmp_store("freshguard").expect("tmp_store");
    let st = st.with_artifacts(tuiscotti::grouped::ArtifactSet::AnsiCursor);
    let f1 = frame_with("run one");
    let f2 = frame_with("run two");

    // 1. Seed a candidate under run-1, then another under run-2.
    let st_r1 = st.clone().with_run_id("run-1");
    st_r1
        .check("showcase/pages/old", &f1, &profile(), &VENDORED_FACES, 1.0)
        .expect("check old");
    std::thread::sleep(std::time::Duration::from_millis(2));
    let st_r2 = st.clone().with_run_id("run-2");
    st_r2
        .check("showcase/pages/new", &f2, &profile(), &VENDORED_FACES, 1.0)
        .expect("check new");

    // 2. Plant an unsealed ghost .ansi file with no manifest/verdict.
    let ghost = st.actual_root().join("showcase/pages/ghost.ansi");
    std::fs::create_dir_all(ghost.parent().expect("parent")).expect("mkdir");
    std::fs::write(&ghost, tuiscotti::render::ansi_dump_with_cursor(&f1)).expect("write ghost");

    // Direct accept and accept_matching both reject the ghost candidate all-or-nothing.
    assert!(st.accept("showcase/pages/ghost").is_err());
    let ghost_err = st
        .accept_matching("showcase/pages/*")
        .expect_err("ghost must fail accept_matching")
        .to_string();
    assert!(ghost_err.contains("incomplete"), "{ghost_err}");
    std::fs::write(
        st.actual_root().join("showcase/pages/ghost.frame.json"),
        f1.to_json(),
    )
    .expect("write ghost frame");
    let ghost_no_manifest = st
        .accept_matching("showcase/pages/*")
        .expect_err("ghost without manifest must fail")
        .to_string();
    assert!(
        ghost_no_manifest.contains("manifest.json missing"),
        "{ghost_no_manifest}"
    );
    assert!(tree_files(st.approved_root()).expect("tree").is_empty());
    std::fs::remove_file(&ghost).expect("rm ghost");
    std::fs::remove_file(st.actual_root().join("showcase/pages/ghost.frame.json"))
        .expect("rm ghost frame");

    // 3. Multi-candidate accept_matching rejects mixed run_ids (stale `showcase/pages/old`).
    let stale_err = st
        .accept_matching("showcase/pages/*")
        .expect_err("stale run_id must fail")
        .to_string();
    assert!(
        stale_err.contains("older run") && stale_err.contains("showcase/pages/old"),
        "{stale_err}"
    );
    assert!(tree_files(st.approved_root()).expect("tree").is_empty());

    // 4. Explicit single-scenario name is allowed when store.run_id() is None,
    //    but refused when store.with_run_id("run-2") is pinned.
    assert!(st_r2.accept_matching("showcase/pages/old").is_err());
    let single = st
        .accept_matching("showcase/pages/old")
        .expect("explicit single name allowed");
    assert_eq!(single, vec!["showcase/pages/old".to_string()]);

    // 5. Tampering with actual .ansi after check invalidates the manifest hash and refuses accept.
    std::fs::write(
        st.actual_root().join("showcase/pages/new.ansi"),
        tuiscotti::render::ansi_dump_with_cursor(&f1),
    )
    .expect("tamper actual");
    let tamper_err = st
        .accept("showcase/pages/new")
        .expect_err("tampered actual must fail")
        .to_string();
    assert!(tamper_err.contains("hash disagrees"), "{tamper_err}");
}

#[test]
fn ansi_cursor_hidden_cursor_no_spurious_diff_and_legacy_trailer_note() {
    use tuiscotti::frame::{Cursor, CursorStyle};
    use tuiscotti::grouped::ArtifactSet;
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
