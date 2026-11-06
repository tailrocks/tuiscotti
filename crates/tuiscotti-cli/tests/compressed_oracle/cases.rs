use super::support::{
    accept, expect_failure, pixel_sample, rich_snapshot, sha256, snapshot_with_capture_time,
    state_sample, verify_approval_summary_subprocess,
};
use tui_test::profile::Rgb;
use tui_test::terminal::cell::Attrs;
use tui_test::{
    SnapshotColor, SnapshotCursorShape, SnapshotScope, SnapshotSource, SnapshotUnderline,
};
use tuiscotti::check::{CheckOutcome, FrozenStore, MismatchReason};
use tuiscotti::render_profile::PROFILE_NAME;
use tuiscotti::sample::{Gate, Observation, Sample};

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one fixed workflow captures ordered comparison semantics"
)]
fn pre_format_comparison_and_state_digest_oracle() {
    let temp = tempfile::tempdir().expect("temporary root builds");
    let baselines = temp.path().join("baselines");
    let artifacts = temp.path().join("artifacts");
    let candidates = temp.path().join("candidates");
    std::fs::create_dir_all(&baselines).expect("baseline root builds");
    std::fs::create_dir_all(&candidates).expect("candidate root builds");
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");

    let baseline = state_sample('A', None);
    let baseline_bytes = baseline.to_bytes().expect("baseline encodes");
    let changed_observation = state_sample('A', Some("review note"));
    let changed_observation_bytes = changed_observation
        .to_bytes()
        .expect("observation sample encodes");
    let changed_state = state_sample('B', None);
    let changed_state_bytes = changed_state.to_bytes().expect("changed sample encodes");
    let metadata_sample = Sample::state(
        "oracle/state",
        snapshot_with_capture_time(1234),
        Observation::empty(),
    )
    .expect("metadata-only state sample builds");
    let metadata_bytes = metadata_sample.to_bytes().expect("metadata sample encodes");

    let digest_baseline = baseline.state_digest().expect("baseline digest computes");
    let digest_observation = changed_observation
        .state_digest()
        .expect("observation digest computes");
    let digest_changed_state = changed_state
        .state_digest()
        .expect("changed state digest computes");
    let digest_metadata = metadata_sample
        .state_digest()
        .expect("metadata state digest computes");
    assert_eq!(
        digest_baseline,
        "b523516c06bb3743b0490d1421d88357584b918ba64d1823fc090aa3eb7ec345"
    );
    assert_eq!(digest_observation, digest_baseline);
    assert_eq!(digest_metadata, digest_baseline);
    assert_eq!(
        digest_changed_state,
        "411aa92d579ef237a317b631b2a83525aab3368c97049b2a12e0e7ec7d01385f"
    );
    assert_ne!(digest_baseline, digest_changed_state);
    assert_ne!(baseline_bytes, changed_observation_bytes);
    assert_ne!(baseline_bytes, metadata_bytes);

    let missing = expect_failure(
        store.check(&baseline_bytes).expect("missing check runs"),
        MismatchReason::Missing,
    );
    assert_eq!(missing.changed_fields(), ["baseline"]);

    let baseline_path = candidates.join("baseline.sample");
    std::fs::write(&baseline_path, &baseline_bytes).expect("candidate sample saves");
    accept(&baseline_path, &baselines, &baseline_bytes);

    assert!(matches!(
        store.check(&baseline_bytes).expect("equal check runs"),
        CheckOutcome::Pass(_)
    ));
    assert!(matches!(
        store
            .check(&changed_observation_bytes)
            .expect("observation-only check runs"),
        CheckOutcome::Pass(_)
    ));
    assert!(matches!(
        store
            .check(&metadata_bytes)
            .expect("metadata-only check runs"),
        CheckOutcome::Pass(_)
    ));

    let changed = expect_failure(
        store
            .check(&changed_state_bytes)
            .expect("state-difference check runs"),
        MismatchReason::State,
    );
    assert_eq!(changed.changed_fields(), &Vec::<String>::new());
    assert_eq!(changed.changed_cell_total(), 1);
    let cell = changed
        .changed_cells()
        .first()
        .expect("changed cell is reported");
    assert_eq!((cell.col(), cell.row()), (0, 0));
    assert_eq!(cell.fields(), ["text"]);
    assert_eq!((cell.actual_text(), cell.baseline_text()), ("B", "A"));
    assert_eq!((changed.pixel_changed(), changed.pixel_compared()), (0, 0));
    assert_eq!(changed.pixel_bounds(), None);

    let rich_baseline = Sample::state(
        "oracle/rich",
        rich_snapshot(false),
        Observation::noted("rich observation"),
    )
    .expect("rich baseline builds");
    let rich_value = rich_baseline.snapshot();
    assert_eq!(rich_value.palette()[17], Rgb::new(17, 51, 238));
    assert_eq!(rich_value.default_foreground(), Rgb::new(61, 62, 63));
    assert_eq!(rich_value.default_background(), Rgb::new(81, 82, 83));
    assert_eq!(rich_value.default_cursor(), Rgb::new(101, 102, 103));
    assert_eq!(rich_value.scope(), SnapshotScope::Viewport);
    assert_eq!(rich_value.title(), Some("oracle title"));
    assert_eq!(rich_value.metadata().captured_at_ms, 1234);
    assert_eq!(rich_value.metadata().revision, 7);
    assert_eq!(rich_value.metadata().backend, "oracle-backend");
    assert_eq!(rich_value.metadata().session, "oracle-session");
    assert_eq!(rich_value.metadata().source, SnapshotSource::Live);
    let rich_cell = rich_value.cell_at(0, 0).expect("rich cell exists");
    assert_eq!(rich_cell.text(), "A");
    assert_eq!(rich_cell.fg(), SnapshotColor::Indexed(17));
    assert_eq!(rich_cell.bg(), SnapshotColor::Rgb(21, 22, 23));
    assert!(rich_cell.has(Attrs::BOLD | Attrs::DIM | Attrs::ITALIC | Attrs::INVERSE));
    assert!(rich_cell.has(Attrs::INVISIBLE | Attrs::STRIKE | Attrs::BLINK));
    assert_eq!(rich_cell.underline(), SnapshotUnderline::Double);
    assert_eq!(rich_cell.underline_color(), SnapshotColor::Indexed(18));
    assert_eq!(rich_cell.link_uri(), Some("https://example.test/oracle"));
    assert_eq!(rich_cell.link_id(), Some("link-a"));
    assert_eq!(rich_value.cursor().col(), 0);
    assert_eq!(rich_value.cursor().row(), 0);
    assert!(rich_value.cursor().in_capture());
    assert!(!rich_value.cursor().visible());
    assert_eq!(rich_value.cursor().shape(), SnapshotCursorShape::Bar);
    assert_eq!(rich_value.cursor().color(), Rgb::new(41, 42, 43));
    assert_eq!(rich_value.cursor().blink(), Some(false));
    let rich_bytes = rich_baseline.to_bytes().expect("rich baseline encodes");
    let rich_digest = rich_baseline
        .state_digest()
        .expect("rich state digest computes");
    let mut projection =
        serde_json::to_value(rich_baseline.snapshot()).expect("rich snapshot projects to JSON");
    projection
        .as_object_mut()
        .expect("snapshot projection is an object")
        .remove("metadata");
    let projection_bytes = serde_json::to_vec(&projection).expect("projection bytes encode");
    assert_eq!(
        projection_bytes,
        include_bytes!("../fixtures/rich_state_projection.canonical.json")
    );
    assert_eq!(projection_bytes.len(), 3348);
    assert_eq!(
        sha256(&projection_bytes),
        "a67e3a9a46d29e392d1dcde4f588f80fbd29c6e6ae0cf0964aeb91fe18809601"
    );
    assert_eq!(
        rich_digest,
        "ccee7231fa2f9bbf45ba74205cd6b4c3b30cba20c3f88e90c480b73e580b2ce8"
    );
    let rich_with_other_metadata = Sample::state(
        "oracle/rich",
        rich_snapshot(false),
        Observation::noted("different observation"),
    )
    .expect("rich observation variant builds");
    assert_eq!(
        rich_with_other_metadata
            .state_digest()
            .expect("rich observation digest computes"),
        rich_digest
    );

    let rich_baselines = temp.path().join("rich-baselines");
    let rich_artifacts = temp.path().join("rich-artifacts");
    std::fs::create_dir_all(&rich_baselines).expect("rich baseline root builds");
    let rich_store =
        FrozenStore::new(&rich_baselines, &rich_artifacts).expect("rich frozen store opens");
    let rich_path = candidates.join("rich.sample");
    std::fs::write(&rich_path, &rich_bytes).expect("rich candidate saves");
    accept(&rich_path, &rich_baselines, &rich_bytes);
    let rich_changed = Sample::state("oracle/rich", rich_snapshot(true), Observation::empty())
        .expect("changed rich sample builds");
    let rich_failure = expect_failure(
        rich_store
            .check(
                &rich_changed
                    .to_bytes()
                    .expect("changed rich sample encodes"),
            )
            .expect("rich comparison runs"),
        MismatchReason::State,
    );
    assert_eq!(
        rich_failure.changed_fields(),
        [
            "scope",
            "palette",
            "default_foreground",
            "default_background",
            "default_cursor",
            "cursor.col",
            "cursor.visible",
            "cursor.shape",
            "cursor.color",
            "cursor.blink",
            "title",
        ]
    );
    assert_eq!(rich_failure.changed_cell_total(), 1);
    let rich_cell = &rich_failure.changed_cells()[0];
    assert_eq!((rich_cell.col(), rich_cell.row()), (0, 0));
    assert_eq!(
        rich_cell.fields(),
        [
            "text",
            "fg",
            "bg",
            "dim",
            "inverse",
            "underline",
            "underline_color",
            "link_uri",
            "link_id",
        ]
    );
    assert_eq!(
        (rich_cell.actual_text(), rich_cell.baseline_text()),
        ("Ω", "A")
    );

    let visual = pixel_sample('A');
    let visual_bytes = visual.to_bytes().expect("pixel sample encodes");
    let image = visual.image().expect("pixel image exists");
    let visual_digest = visual.state_digest().expect("pixel state digest computes");
    assert_eq!(visual_digest.len(), 64);
    assert_eq!((image.width(), image.height()), (20, 20));
    assert_eq!(
        image.png_sha256(),
        "bbfe50eda184a97f3a4e8e0db529fd54f94fdc218766f3c5d6b9334c7dc85770"
    );
    assert_eq!(
        image.rgba_sha256(),
        "2b49b902a2b0d7af09af14f66212a7142e20d8592920813fe9864aff124222bf"
    );
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    assert_eq!(
        visual_digest,
        "cc393c1c7a07632a3ec141561c373d13a708a1cb10b41f4a0c69834386cec943"
    );
    assert_eq!(visual.gate(), Gate::StateAndPixels);
    assert_eq!(
        visual.render_contract().expect("contract exists").profile(),
        PROFILE_NAME
    );
    let contract = visual.render_contract().expect("contract exists");
    assert_eq!(
        (
            contract.cell_width(),
            contract.cell_height(),
            contract.font_size(),
            contract.baseline(),
            contract.scale(),
        ),
        (10.0, 20.0, 16.0, 15.0, 1.0)
    );
    assert_eq!(contract.blink(), None);
    assert_eq!(
        visual_bytes,
        visual.to_bytes().expect("encoding is deterministic")
    );

    let pixel_baselines = temp.path().join("pixel-baselines");
    let pixel_artifacts = temp.path().join("pixel-artifacts");
    std::fs::create_dir_all(&pixel_baselines).expect("pixel baseline root builds");
    let pixel_store =
        FrozenStore::new(&pixel_baselines, &pixel_artifacts).expect("pixel frozen store opens");
    let pixel_path = candidates.join("visual.sample");
    std::fs::write(&pixel_path, &visual_bytes).expect("pixel candidate saves");
    accept(&pixel_path, &pixel_baselines, &visual_bytes);
    assert!(matches!(
        pixel_store
            .check(&visual_bytes)
            .expect("pixel pass check runs"),
        CheckOutcome::Pass(_)
    ));

    verify_approval_summary_subprocess();
}
