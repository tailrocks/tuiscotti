use tui_test::{SnapshotCursor, SnapshotParts, TerminalSnapshot};
use tuiscotti::render_profile::{PROFILE_NAME, platform_key, resolve_profile};
use tuiscotti::sample::{CursorPhase, Observation, Sample};
use tuiscotti::view::{ViewSettings, capture_view};

fn pixel_sample_with_blink(blink: Option<bool>) -> Sample {
    let mut captured = capture_view(2, 1, &ViewSettings::builtin(), |frame| {
        frame.buffer_mut()[(0, 0)].set_char('A');
    })
    .expect("contract snapshot captures");
    let old_cursor = captured.cursor();
    let cursor = SnapshotCursor::new(
        old_cursor.col(),
        old_cursor.row(),
        old_cursor.in_capture(),
        old_cursor.visible(),
        old_cursor.shape(),
        old_cursor.color(),
        blink,
    );
    captured = TerminalSnapshot::try_new(SnapshotParts {
        cols: captured.cols(),
        rows: captured.rows(),
        scope: captured.scope(),
        viewport_origin: captured.viewport_origin(),
        cells: captured.cells().to_vec(),
        palette: *captured.palette(),
        default_foreground: captured.default_foreground(),
        default_background: captured.default_background(),
        default_cursor: captured.default_cursor(),
        cursor,
        title: captured.title().map(str::to_string),
        metadata: captured.metadata().clone(),
    })
    .expect("blink variant validates");
    let id = format!(
        "oracle/contract-blink/pixels-{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let profile = resolve_profile(PROFILE_NAME).expect("profile resolves");
    Sample::with_pixels(&id, &captured, Observation::empty(), profile).expect("blink sample builds")
}

#[test]
fn render_contract_projection_and_optional_blink_oracle() {
    let cases = [
        (
            None,
            "cc393c1c7a07632a3ec141561c373d13a708a1cb10b41f4a0c69834386cec943",
        ),
        (
            Some(false),
            "8e5d26377b634a6b5504b585e0c6c5f1874ed37722135fb321c50149965c00fb",
        ),
        (
            Some(true),
            "6be7c7cf2153a86c22e907177867b41cc8c5bc9f5543c5d6a230a5653aa87703",
        ),
    ];
    let mut state_digests = Vec::new();
    for (blink, expected_state_digest) in cases {
        let _ = expected_state_digest;
        let sample = pixel_sample_with_blink(blink);
        let contract = sample.render_contract().expect("render contract exists");
        assert_eq!(contract.blink(), blink);
        let digest = sample.state_digest().expect("state digest computes");
        assert_eq!(contract.profile(), PROFILE_NAME);
        assert_eq!(contract.contract_version(), 1);
        assert_eq!(contract.renderer_version(), "0.2.1");
        assert_eq!(contract.raster_policy(), "pinned-v1");
        assert_eq!(contract.cell_width(), 10.0);
        assert_eq!(contract.cell_height(), 20.0);
        assert_eq!(contract.font_size(), 16.0);
        assert_eq!(contract.baseline(), 15.0);
        assert_eq!(contract.scale(), 1.0);
        assert!(!contract.title_bar());
        assert_eq!(
            contract.palette_digest(),
            "38f64979c4c0c9e65966308a52c27d01d65ea87eb2571216c8c4be071730ff3d"
        );
        assert_eq!(contract.cursor_phase(), CursorPhase::Hidden);
        assert_eq!(contract.platform_key(), platform_key());
        let fonts = contract.fonts();
        assert_eq!(fonts.len(), 15);
        assert_eq!(
            fonts[0].digest(),
            "604c14112e27072a2049bc87c21743733fd04efdf66cf8e5f88a4cabd411b0fc"
        );
        assert_eq!(fonts[0].face_index(), 0);
        assert!(!fonts[0].bold());
        assert!(!fonts[0].italic());
        assert_eq!(fonts[0].fallback_position(), 0);
        assert_eq!(
            fonts[14].digest(),
            "ef698a4abbad7d270702e8847b6635eaffdf66569265e0a019cab0d86ca49230"
        );
        assert_eq!(fonts[14].face_index(), 0);
        assert!(fonts[14].bold());
        assert!(fonts[14].italic());
        assert_eq!(fonts[14].fallback_position(), 14);
        state_digests.push(digest.clone());
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        assert_eq!(digest, expected_state_digest);
    }
    assert_ne!(state_digests[0], state_digests[1]);
    assert_ne!(state_digests[0], state_digests[2]);
    assert_ne!(state_digests[1], state_digests[2]);
}
