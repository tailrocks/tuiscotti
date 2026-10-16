use super::*;
use tuiscotti::VENDORED_FACES;
use tuiscotti::grouped::GroupedStore;
use tuiscotti::snapshot::Status;

#[test]
fn accept_all_walks_nested_names_recursively() {
    if rerun_without_ci() {
        return;
    }
    let (_dir, st) = tmp_store("acceptall").expect("tmp_store");
    assert!(st.accept_all().expect("empty accept_all").is_empty());
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
        .expect("check");
    }
    let mut listed = st.actual_names().expect("actual_names");
    assert_eq!(listed.len(), 4, "{listed:?}");
    let accepted = st.accept_all().expect("accept_all");
    assert_eq!(accepted, listed);
    assert_eq!(st.approved_names().expect("approved_names"), listed);

    // Re-checking identical frames produces Status::Matched; accept_all skips them.
    for (i, name) in names.iter().enumerate() {
        let o = st
            .check(
                name,
                &frame_with(&format!("screen {i}")),
                &profile(),
                &VENDORED_FACES,
                1.0,
            )
            .expect("recheck matched");
        assert_eq!(o.status(), Status::Matched);
    }
    assert!(
        st.accept_all()
            .expect("accept_all skips matched")
            .is_empty()
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
            .expect("check");
        assert_eq!(outcome.status(), Status::CellsDiffer, "{name} was approved");
    }
    listed.sort();
    let mut sorted = names.to_vec();
    sorted.sort_unstable();
    assert_eq!(listed, sorted);
}

#[test]
fn report_reverifies_nested_actuals_outside_approved_tree() {
    if rerun_without_ci() {
        return;
    }
    let (_dir, st) = tmp_store("report").expect("tmp_store");
    st.check(
        "suite/one",
        &frame_with("alpha"),
        &profile(),
        &VENDORED_FACES,
        1.0,
    )
    .expect("check");
    st.accept("suite/one").expect("accept");
    st.check(
        "suite/nested/two",
        &frame_with("beta"),
        &profile(),
        &VENDORED_FACES,
        1.0,
    )
    .expect("check");
    let report = st
        .report(&profile(), &VENDORED_FACES, 1.0, "grouped suite")
        .expect("report");
    assert_eq!(report.outcomes.len(), 2);
    assert_eq!(report.failed(), 1, "`suite/nested/two` has no approval");
    assert!(report.path.exists());
    assert!(report.path.starts_with(st.actual_root()));
    assert!(!report.path.starts_with(st.approved_root()));
    let html = std::fs::read_to_string(&report.path).expect("read report");
    assert!(html.contains("suite/one — matched"), "{html}");
    assert!(
        html.contains("suite/nested/two — missing-approval"),
        "{html}"
    );
    assert!(
        !html.contains("data:image/png;base64,"),
        "suite report must not embed PNGs"
    );
    assert_eq!(
        tree_files(st.approved_root()).expect("tree_files"),
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
    let (dir, st) = tmp_store("reportcfg").expect("tmp_store");
    let custom = dir.path().join("target").join("grouped-report.html");
    let st = st.with_report_path(&custom);
    st.check("a/b", &frame_with("x"), &profile(), &VENDORED_FACES, 1.0)
        .expect("check");
    let report = st
        .report(&profile(), &VENDORED_FACES, 1.0, "custom path")
        .expect("report");
    assert_eq!(report.path, custom);
    assert!(custom.exists());
}

#[test]
fn custom_actual_and_diff_roots_are_honored() {
    if rerun_without_ci() {
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let approved = dir.path().join("approved-tree");
    let st = GroupedStore::new(&approved)
        .with_actual_root(&dir.path().join("scratch/actual"))
        .with_diff_root(&dir.path().join("scratch/diff"));
    let name = "deep/nested/name";
    let outcome = st
        .check(name, &frame_with("roots"), &profile(), &VENDORED_FACES, 1.0)
        .expect("check");
    assert!(
        outcome
            .actual
            .ansi
            .starts_with(dir.path().join("scratch/actual"))
    );
    assert_eq!(outcome.status(), Status::MissingApproval);
    st.accept(name).expect("accept");
    assert!(approved.join(format!("{name}.ansi")).exists());
    let outcome = st
        .check(
            name,
            &frame_with("changed"),
            &profile(),
            &VENDORED_FACES,
            1.0,
        )
        .expect("check");
    let diff = outcome.outcome.diff_png.expect("diff_png");
    assert!(
        diff.starts_with(dir.path().join("scratch/diff")),
        "{}",
        diff.display()
    );
}

#[test]
fn default_roots_are_siblings_of_approved() {
    let dir = tempfile::tempdir().expect("tempdir");
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
    if rerun_without_ci() {
        return;
    }
    let (_dir, st) = tmp_store("noactual").expect("tmp_store");
    let err = st
        .accept("never/checked")
        .expect_err("accept without actuals must error")
        .to_string();
    assert!(err.contains("nothing to accept"), "{err}");
}

#[test]
fn check_with_reuses_one_renderer_across_nested_checks() {
    if rerun_without_ci() {
        return;
    }
    let (_dir, st) = tmp_store("checkwith").expect("tmp_store");
    let profile = profile();
    let mut renderer = profile.renderer(&VENDORED_FACES).expect("renderer");
    let frame = frame_with("cached grouped");
    let o1 = st
        .check_with(&mut renderer, "g/s", &frame, 1.0)
        .expect("check_with");
    assert_eq!(o1.status(), Status::MissingApproval);
    st.accept("g/s").expect("accept");
    let o2 = st
        .check_with(&mut renderer, "g/s", &frame, 1.0)
        .expect("check_with");
    assert_eq!(o2.status(), Status::Matched);
    assert_eq!(o2.outcome.pixel_score, Some(1.0));
    o2.ensure_matched().expect("ensure_matched");
}

#[test]
fn html_artifact_is_a_standalone_colored_render() {
    let name = "docs/preview";
    let (_dir, st) = tmp_store("htmlview").expect("tmp_store");
    let outcome = st
        .check(
            name,
            &frame_with("standalone"),
            &profile(),
            &VENDORED_FACES,
            1.0,
        )
        .expect("check");
    let html = std::fs::read_to_string(&outcome.actual.html).expect("read html");
    assert!(html.contains("<svg"), "{html}");
    assert!(html.contains("data:image/png;base64,"), "{html}");
    assert!(
        html.contains("<script type=\"application/json\">"),
        "{html}"
    );
    assert!(html.contains("<title>docs/preview</title>"), "{html}");
    let body = html.split("<body>").nth(1).expect("body");
    let img = body.find("<img ").expect("png img");
    assert!(
        !body[..img].contains("<details"),
        "PNG must be the primary visual, not nested under details: {html}"
    );
}

#[test]
fn accept_rejects_ghost_tampered_and_stale_run_candidates() {
    if rerun_without_ci() {
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

    // Direct accept, accept_all, and accept_matching all reject the ghost candidate.
    assert!(st.accept("showcase/pages/ghost").is_err());
    assert!(st.accept_all().is_err());
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

    // 3. Multi-candidate accept_matching and accept_all reject mixed run_ids.
    for stale_err in [
        st.accept_matching("showcase/pages/*")
            .expect_err("stale run_id must fail accept_matching")
            .to_string(),
        st.accept_all()
            .expect_err("stale run_id must fail accept_all")
            .to_string(),
    ] {
        assert!(
            stale_err.contains("older run") && stale_err.contains("showcase/pages/old"),
            "{stale_err}"
        );
    }
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
