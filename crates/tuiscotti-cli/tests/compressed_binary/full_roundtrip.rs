//! Public API round trips for independent values and pixel evidence.

#![expect(
    clippy::expect_used,
    reason = "round-trip fixture failures need actionable assertion context"
)]

use super::*;

use tui_test::{SnapshotColor, SnapshotScope, SnapshotSource, SnapshotUnderline};
use tuiscotti::sample::Sample;

#[test]
fn independent_fixture_round_trips_every_snapshot_field() {
    let original = Sample::from_bytes(&fixtures::valid_container()).expect("fixture decodes");
    assert_independent_snapshot(&original);
    let encoded = original.to_bytes().expect("public writer encodes fixture");
    assert!(encoded.starts_with(b"TUISCPB\0"));
    let decoded = Sample::from_bytes(&encoded).expect("public reader decodes writer output");
    assert_independent_snapshot(&decoded);
}

fn assert_independent_snapshot(sample: &Sample) {
    assert_eq!(sample.schema(), tuiscotti::sample::SampleSchema);
    assert_eq!(sample.test_id(), fixtures::TEST_ID);
    assert_eq!(sample.gate().as_str(), "state");
    assert_eq!(sample.observation().note(), Some(""));
    assert!(sample.render_contract().is_none());
    assert!(sample.image().is_none());

    let snapshot = sample.snapshot();
    assert_eq!((snapshot.cols(), snapshot.rows()), (2, 1));
    assert_eq!(snapshot.scope(), SnapshotScope::FullHistory);
    assert_eq!(snapshot.viewport_origin(), 0);
    assert_eq!(snapshot.title(), Some(""));
    assert_eq!(snapshot.cells().len(), 2);
    let styled = &snapshot.cells()[0];
    assert_eq!(styled.text(), "e\u{301}");
    assert_eq!(styled.width(), 1);
    assert_eq!(styled.fg(), SnapshotColor::Rgb(0, 128, 255));
    assert_eq!(styled.bg(), SnapshotColor::Indexed(255));
    assert!(styled.has(tui_test::terminal::cell::Attrs::BOLD));
    assert!(!styled.has(tui_test::terminal::cell::Attrs::DIM));
    assert!(styled.has(tui_test::terminal::cell::Attrs::ITALIC));
    assert!(!styled.has(tui_test::terminal::cell::Attrs::INVERSE));
    assert!(styled.has(tui_test::terminal::cell::Attrs::INVISIBLE));
    assert!(!styled.has(tui_test::terminal::cell::Attrs::STRIKE));
    assert!(styled.has(tui_test::terminal::cell::Attrs::BLINK));
    assert_eq!(styled.underline(), SnapshotUnderline::Dashed);
    assert_eq!(styled.underline_color(), SnapshotColor::Indexed(0));
    assert_eq!(styled.link_uri(), Some(""));
    assert_eq!(styled.link_id(), Some("link-id"));

    let wide = &snapshot.cells()[1];
    assert_eq!(wide.text(), "🦀");
    assert_eq!(wide.width(), 2);
    assert_eq!(wide.fg(), SnapshotColor::Default);
    assert_eq!(wide.bg(), SnapshotColor::Rgb(16, 32, 48));
    assert_eq!(wide.underline(), SnapshotUnderline::None);
    assert_eq!(wide.underline_color(), SnapshotColor::Default);
    assert_eq!(wide.link_uri(), None);
    assert_eq!(wide.link_id(), None);
    for index in 0..256 {
        let index = u8::try_from(index).expect("palette index fits u8");
        assert_eq!(snapshot.palette()[usize::from(index)], tuiscotti_rgb(index),);
    }
    assert_eq!(
        snapshot.default_foreground(),
        tui_test::profile::Rgb::new(1, 2, 3)
    );
    assert_eq!(
        snapshot.default_background(),
        tui_test::profile::Rgb::new(4, 5, 6)
    );
    assert_eq!(
        snapshot.default_cursor(),
        tui_test::profile::Rgb::new(7, 8, 9)
    );

    let cursor = snapshot.cursor();
    assert_eq!((cursor.col(), cursor.row()), (123, 456));
    assert!(!cursor.in_capture());
    assert!(!cursor.visible());
    assert_eq!(cursor.shape().name(), "bar");
    assert_eq!(cursor.color(), tui_test::profile::Rgb::new(17, 34, 51));
    assert_eq!(cursor.blink(), Some(false));

    let metadata = snapshot.metadata();
    assert_eq!(metadata.captured_at_ms, u64::MAX);
    assert_eq!(metadata.revision, 0);
    assert_eq!(metadata.backend, "fixture-backend");
    assert_eq!(metadata.session, "");
    assert_eq!(metadata.source, SnapshotSource::Live);
}

fn tuiscotti_rgb(index: u8) -> tui_test::profile::Rgb {
    tui_test::profile::Rgb::new(index, 255 - index, index / 2)
}

#[test]
fn public_pixel_round_trip_preserves_contract_png_and_decoded_rgba() {
    let settings = tuiscotti::view::ViewSettings::builtin();
    let snapshot = tuiscotti::view::capture_view(12, 3, &settings, |frame| {
        frame.buffer_mut()[(0, 0)].set_char('X');
    })
    .expect("view fixture captures");
    let profile =
        tuiscotti::render_profile::resolve_profile(tuiscotti::render_profile::PROFILE_NAME)
            .expect("pinned profile resolves");
    let id = format!(
        "binary/full-roundtrip/pixels-{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let original = Sample::with_pixels(
        &id,
        &snapshot,
        tuiscotti::sample::Observation::noted(""),
        profile,
    )
    .expect("pixel sample builds");
    let decoded =
        Sample::from_bytes(&original.to_bytes().expect("sample encodes")).expect("sample decodes");

    assert_eq!(decoded.test_id(), original.test_id());
    assert_eq!(decoded.gate(), original.gate());
    assert_eq!(decoded.observation(), original.observation());
    assert_eq!(decoded.snapshot(), original.snapshot());
    assert_eq!(
        decoded.snapshot().metadata(),
        original.snapshot().metadata()
    );
    let before = original.render_contract().expect("contract exists");
    let after = decoded.render_contract().expect("decoded contract exists");
    assert_eq!(after.profile(), before.profile());
    assert_eq!(after.contract_version(), before.contract_version());
    assert_eq!(after.renderer_version(), before.renderer_version());
    assert_eq!(after.raster_policy(), before.raster_policy());
    assert_eq!(after.fonts(), before.fonts());
    assert_eq!(after.cell_width().to_bits(), before.cell_width().to_bits());
    assert_eq!(
        after.cell_height().to_bits(),
        before.cell_height().to_bits()
    );
    assert_eq!(after.font_size().to_bits(), before.font_size().to_bits());
    assert_eq!(after.baseline().to_bits(), before.baseline().to_bits());
    assert_eq!(after.scale().to_bits(), before.scale().to_bits());
    assert_eq!(after.title_bar(), before.title_bar());
    assert_eq!(after.palette_digest(), before.palette_digest());
    assert_eq!(after.cursor_phase(), before.cursor_phase());
    assert_eq!(after.blink(), before.blink());
    assert_eq!(after.platform_key(), before.platform_key());

    let before = original.image().expect("image exists");
    let after = decoded.image().expect("decoded image exists");
    assert_eq!(after.format(), before.format());
    assert_eq!(
        (after.width(), after.height()),
        (before.width(), before.height())
    );
    assert_eq!(after.png_sha256(), before.png_sha256());
    assert_eq!(after.rgba_sha256(), before.rgba_sha256());
    let before_png = base64::engine::general_purpose::STANDARD
        .decode(before.png_base64())
        .expect("original PNG base64 decodes");
    let after_png = base64::engine::general_purpose::STANDARD
        .decode(after.png_base64())
        .expect("decoded PNG base64 decodes");
    assert_eq!(after_png, before_png);
    let before_rgba = image::load_from_memory_with_format(&before_png, image::ImageFormat::Png)
        .expect("original PNG decodes")
        .to_rgba8()
        .into_raw();
    let after_rgba = image::load_from_memory_with_format(&after_png, image::ImageFormat::Png)
        .expect("round-trip PNG decodes")
        .to_rgba8()
        .into_raw();
    assert_eq!(after_rgba, before_rgba);
}
