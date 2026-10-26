//! D02 sample tests: format, profile, and render evidence.
//!
//! Covers the D02 subcases from `docs/migration/corpus.json`:
//!
//! - C04: the sample keeps the mandatory graphemes and continuations,
//!   through direct construction and through the `view` conversion chain.
//! - C05: the sample keeps styles, palette, defaults, underline color,
//!   reverse video, and cursor state; styled pixels differ.
//! - C06: host font availability cannot change strict profile output
//!   (temp `FONTCONFIG` manipulation, sandboxed, like upstream U04-A).
//! - C07: observation metadata changes do not move the compared state;
//!   visual changes do (comparison itself is D03).
//! - C08: RGBA digests run over decoded pixels and survive PNG re-encoding
//!   with different metadata; one changed channel fails to load.
//! - C10: candidate and baseline digests agree on construction, and frame
//!   and image data agree.
//!
//! Also: profile registry checks, profile metric verification (every
//! mandatory glyph keeps its ink inside its cell), unsupported-schema and
//! unknown-field rejection, resource-limit rejection before allocation,
//! the mandatory missing-glyph case, and unknown-profile rejection.
//!
//! All tests run offline in tempdirs. No test fetches fonts: every face
//! comes from the embedded profile assets.

use base64::Engine as _;
use ratatui::Frame;
use ratatui::widgets::Paragraph;
use sha2::{Digest, Sha256};
use tui_test::profile::Rgb;
use tui_test::terminal::cell::Attrs;
use tui_test::{
    PinnedFontError, RenderError, SnapshotCell, SnapshotColor, SnapshotCursor, SnapshotCursorShape,
    SnapshotMetadata, SnapshotParts, SnapshotScope, SnapshotSource, SnapshotUnderline,
    TerminalSnapshot,
};
use tuiscotti::render_profile::ProfileError as RegistryError;
use tuiscotti::render_profile::{PROFILE_NAME, platform_key, resolve_profile};
use tuiscotti::sample::{
    CONTRACT_VERSION, CursorPhase, Gate, ImageFormat, Observation, Sample, SampleError, SampleImage,
};
use tuiscotti::view::{ViewCursor, ViewSettings, capture_view};

/// Mandatory combining cases from the corpus.
const E_ACUTE: &str = "e\u{301}";
const A_DIAERESIS: &str = "a\u{308}";
/// Mandatory CJK case with its continuation cell.
const CJK: &str = "東";
/// Mandatory unsupported glyph: no pinned face contains it.
const CRAB: &str = "\u{1F980}";
/// A Private Use Area icon the symbol coverage serves.
const PUA_ICON: &str = "\u{E61C}";
/// Discriminating Private Use Area icon for the "symbol font first" rule:
/// present in the symbol font but absent from `JetBrains Mono` Nerd Font
/// Mono 3.5.1, so a successful render proves the symbol slot serves it.
const PUA_SYMBOL_FIRST: &str = "\u{F6C5}";

/// Black canvas the profile renders onto for these fixtures.
const CANVAS: [u8; 4] = [0, 0, 0, 255];

/// Built-in view palette: the fixed migration record, no child or session.
fn test_palette() -> [Rgb; 256] {
    *ViewSettings::builtin().palette()
}

#[expect(clippy::expect_used, reason = "fixture inputs are statically valid")]
fn plain_cell(text: &str, width: u16) -> SnapshotCell {
    SnapshotCell::new(
        text.to_string(),
        width,
        SnapshotColor::Default,
        SnapshotColor::Default,
        Attrs::empty(),
        SnapshotUnderline::None,
        SnapshotColor::Default,
        None,
        None,
    )
    .expect("fixture cell is valid")
}

#[expect(clippy::expect_used, reason = "fixture inputs are statically valid")]
fn styled_cell(
    text: &str,
    width: u16,
    attrs: Attrs,
    underline: SnapshotUnderline,
    underline_color: SnapshotColor,
    foreground: SnapshotColor,
) -> SnapshotCell {
    SnapshotCell::new(
        text.to_string(),
        width,
        foreground,
        SnapshotColor::Default,
        attrs,
        underline,
        underline_color,
        None,
        None,
    )
    .expect("fixture styled cell is valid")
}

#[expect(clippy::expect_used, reason = "fixture inputs are statically valid")]
fn continuation_cell() -> SnapshotCell {
    SnapshotCell::new(
        String::new(),
        0,
        SnapshotColor::Default,
        SnapshotColor::Default,
        Attrs::empty(),
        SnapshotUnderline::None,
        SnapshotColor::Default,
        None,
        None,
    )
    .expect("fixture continuation cell is valid")
}

fn hidden_cursor() -> SnapshotCursor {
    SnapshotCursor::new(
        0,
        0,
        true,
        false,
        SnapshotCursorShape::Block,
        Rgb::new(192, 192, 192),
        None,
    )
}

fn block_cursor_at(col: u32, row: u32) -> SnapshotCursor {
    SnapshotCursor::new(
        col,
        row,
        true,
        true,
        SnapshotCursorShape::Block,
        Rgb::new(192, 192, 192),
        None,
    )
}

fn snapshot_with(
    cols: u32,
    rows: u32,
    cells: Vec<SnapshotCell>,
    cursor: SnapshotCursor,
) -> TerminalSnapshot {
    snapshot_with_colors(
        cols,
        rows,
        cells,
        cursor,
        &test_palette(),
        Rgb::new(192, 192, 192),
        Rgb::new(0, 0, 0),
        Rgb::new(192, 192, 192),
        None,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "fixture constructor mirrors SnapshotParts"
)]
#[expect(clippy::expect_used, reason = "fixture snapshots are valid")]
fn snapshot_with_colors(
    cols: u32,
    rows: u32,
    cells: Vec<SnapshotCell>,
    cursor: SnapshotCursor,
    palette: &[Rgb; 256],
    foreground: Rgb,
    background: Rgb,
    cursor_color: Rgb,
    title: Option<String>,
) -> TerminalSnapshot {
    TerminalSnapshot::try_new(SnapshotParts {
        cols,
        rows,
        scope: SnapshotScope::Viewport,
        viewport_origin: 0,
        cells,
        palette: *palette,
        default_foreground: foreground,
        default_background: background,
        default_cursor: cursor_color,
        cursor,
        title,
        metadata: SnapshotMetadata::default(),
    })
    .expect("fixture snapshot is valid")
}

/// Test ID for a `state-and-pixels` sample on this platform.
fn pixels_test_id(logical: &str) -> String {
    format!("{logical}/pixels-{}", platform_key())
}

/// Decode the embedded image twice: raw bytes plus an independent pixel
/// oracle through the `image` crate.
#[expect(clippy::expect_used, reason = "fixture images decode")]
fn decode_image(image: &SampleImage) -> (Vec<u8>, u32, u32, Vec<u8>) {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(image.png_base64().as_bytes())
        .expect("fixture base64 decodes");
    let decoded = image::load_from_memory(&raw).expect("fixture PNG decodes");
    let rgba = decoded.to_rgba8();
    let (width, height) = (rgba.width(), rgba.height());
    (raw, width, height, rgba.into_raw())
}

/// Ink offsets (pixels different from the canvas), in image coordinates.
#[expect(clippy::expect_used, reason = "fixture images fit the grid")]
fn ink_offsets(pixels: &[u8], image_width: u32) -> Vec<(u32, u32)> {
    let (chunks, _) = pixels.as_chunks::<4>();
    chunks
        .iter()
        .enumerate()
        .filter(|(_, pixel)| **pixel != CANVAS)
        .map(|(index, _)| {
            let index = u32::try_from(index).expect("fixture image fits");
            (index % image_width, index / image_width)
        })
        .collect()
}

/// Ink inside one cell rect, relative to the rect origin.
fn cell_ink(
    pixels: &[u8],
    image_width: u32,
    left: u32,
    top: u32,
    width: u32,
    height: u32,
) -> Vec<(u32, u32)> {
    ink_offsets(pixels, image_width)
        .into_iter()
        .filter(|(x, y)| *x >= left && *x < left + width && *y >= top && *y < top + height)
        .map(|(x, y)| (x - left, y - top))
        .collect()
}

/// ASCII art of one cell rect for bbox diagnosis on failure.
fn ascii_cell(
    pixels: &[u8],
    image_width: u32,
    left: u32,
    top: u32,
    width: u32,
    height: u32,
) -> String {
    let mut art = String::new();
    for row in 0..height {
        for col in 0..width {
            let index = ((top + row) * image_width + left + col) as usize * 4;
            art.push(if pixels[index..index + 4] == CANVAS {
                '.'
            } else {
                '#'
            });
        }
        art.push('\n');
    }
    art
}

/// Render one text cell through the pinned profile for metric checks.
#[expect(clippy::expect_used, reason = "metric probes render")]
fn render_text_cell(cells: Vec<SnapshotCell>, cols: u32, name: &str) -> (u32, u32, Vec<u8>) {
    let profile = resolve_profile(PROFILE_NAME).expect("pinned profile resolves");
    let snapshot = snapshot_with(cols, 1, cells, hidden_cursor());
    let sample = Sample::with_pixels(
        &pixels_test_id(&format!("metrics/{name}")),
        &snapshot,
        Observation::empty(),
        profile,
    )
    .expect("metric probe renders");
    let image = sample.image().expect("pixels sample carries an image");
    let (_, width, height, pixels) = decode_image(image);
    (width, height, pixels)
}

/// Assert every ink pixel sits inside the glyph rect: no overflow past the
/// cell bounds, no bleed into neighbors, and no clipped edge rows.
fn assert_glyph_ink(
    name: &str,
    pixels: &[u8],
    image_width: u32,
    left: u32,
    top: u32,
    width: u32,
    height: u32,
) {
    let ink = cell_ink(pixels, image_width, left, top, width, height);
    assert!(
        !ink.is_empty(),
        "{name}: glyph cell holds no ink\n{}",
        ascii_cell(pixels, image_width, left, top, width, height)
    );
    assert_eq!(
        ink.len(),
        ink_offsets(pixels, image_width).len(),
        "{name}: ink escapes the glyph cell\n{}",
        ascii_cell(pixels, image_width, left, top, width, height)
    );
    assert!(
        ink.iter().all(|(_, y)| *y > 0 && *y < height - 1),
        "{name}: ink touches a clipped edge row\n{}",
        ascii_cell(pixels, image_width, left, top, width, height)
    );
}

#[test]
fn d02_profile_registry_resolves_pinned_v1() {
    let profile = resolve_profile(PROFILE_NAME).expect("pinned profile resolves");
    assert_eq!(profile.name(), PROFILE_NAME);
    // Four style slots: the regular slot carries three faces, every other
    // slot carries the styled face plus the regular fallback first.
    let roles: Vec<&str> = profile
        .faces()
        .iter()
        .map(tuiscotti::render_profile::ProfileFace::role)
        .collect();
    assert_eq!(
        roles,
        [
            "primary-regular",
            "symbol-regular",
            "cjk-regular",
            "primary-bold",
            "primary-regular-fallback-bold",
            "symbol-bold",
            "cjk-bold",
            "primary-italic",
            "primary-regular-fallback-italic",
            "symbol-italic",
            "cjk-italic",
            "primary-bold-italic",
            "primary-regular-fallback-bold-italic",
            "symbol-bold-italic",
            "cjk-bold-italic",
        ]
    );
    // Registry keys: SHA-256 of the face bytes plus the face index.
    for face in profile.faces() {
        assert_eq!(face.bytes_sha256().len(), 64, "{}", face.role());
        assert_eq!(face.index(), 0, "single-face assets: {}", face.role());
        assert!(face.byte_len() > 1000, "{}", face.role());
    }
    // Style slots differ; only the regular slot lacks a styled face.
    assert!(
        !profile.faces()[0].bold() && !profile.faces()[0].italic(),
        "regular slot is unstyled"
    );
    assert!(profile.faces()[3].bold() && !profile.faces()[3].italic());
    // The platform key joins the Rust target names.
    assert_eq!(
        platform_key(),
        format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
    );
    assert!(platform_key().contains('-'));
    // Unknown profiles fail without touching fonts or the network.
    assert!(matches!(
        resolve_profile("tuiscotti-portable"),
        Err(RegistryError::UnknownProfile { .. })
    ));
    assert!(matches!(
        resolve_profile(""),
        Err(RegistryError::UnknownProfile { .. })
    ));
}

/// Every byte the profile embeds — plus the license text that must ship
/// with it — lives inside the library package, so `cargo package` cannot
/// omit it. An `include_bytes!` path that escapes the crate still compiles
/// (the file exists in the checkout) but silently drops out of the
/// published crate; this test trips if that regresses.
#[test]
fn d02_profile_assets_ship_inside_the_crate() {
    const SHIPPED: &[&str] = &[
        "fonts/JetBrainsMonoNerdFontMono-Regular.ttf",
        "fonts/JetBrainsMonoNerdFontMono-Bold.ttf",
        "fonts/JetBrainsMonoNerdFontMono-Italic.ttf",
        "fonts/JetBrainsMonoNerdFontMono-BoldItalic.ttf",
        "fonts/DejaVuSansMNerdFontMono-Regular.ttf",
        "fonts/NotoSansCJKjp-subset.otf",
        "fonts/LICENSE-JetBrainsMono.txt",
        "fonts/LICENSE-DejaVuSansMono.txt",
        "fonts/LICENSE-Noto.txt",
    ];
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for name in SHIPPED {
        assert!(
            root.join(name).is_file(),
            "{name} must ship inside the tuiscotti package"
        );
    }
}

/// One render per mandatory glyph: name, cells, grid columns, and the glyph
/// rect origin and width in columns.
fn metric_cases() -> Vec<(&'static str, Vec<SnapshotCell>, u32, u32, u32)> {
    vec![
        (
            "ascii",
            vec![plain_cell("A", 1), plain_cell(" ", 1)],
            2,
            0,
            1,
        ),
        (
            "e-acute",
            vec![plain_cell(E_ACUTE, 1), plain_cell(" ", 1)],
            2,
            0,
            1,
        ),
        (
            "a-diaeresis-italic",
            vec![
                styled_cell(
                    A_DIAERESIS,
                    1,
                    Attrs::ITALIC,
                    SnapshotUnderline::None,
                    SnapshotColor::Default,
                    SnapshotColor::Default,
                ),
                plain_cell(" ", 1),
            ],
            2,
            0,
            1,
        ),
        (
            "cjk",
            vec![plain_cell(CJK, 2), continuation_cell(), plain_cell(" ", 1)],
            3,
            0,
            2,
        ),
        (
            "last-column-wide",
            vec![plain_cell(" ", 1), plain_cell(" ", 1), plain_cell(CJK, 1)],
            3,
            2,
            1,
        ),
        (
            "bold",
            vec![
                styled_cell(
                    "B",
                    1,
                    Attrs::BOLD,
                    SnapshotUnderline::None,
                    SnapshotColor::Default,
                    SnapshotColor::Default,
                ),
                plain_cell(" ", 1),
            ],
            2,
            0,
            1,
        ),
        (
            "pua-icon",
            vec![plain_cell(PUA_ICON, 1), plain_cell(" ", 1)],
            2,
            0,
            1,
        ),
        (
            "pua-icon-symbol-first",
            vec![plain_cell(PUA_SYMBOL_FIRST, 1), plain_cell(" ", 1)],
            2,
            0,
            1,
        ),
    ]
}

#[test]
fn d02_metrics_keep_mandatory_ink_in_cell() {
    assert_eq!(
        E_ACUTE.chars().count(),
        2,
        "combining sequence, not precomposed"
    );
    assert_eq!(A_DIAERESIS.chars().count(), 2);
    for (name, cells, cols, left, width) in metric_cases() {
        let (image_width, image_height, pixels) = render_text_cell(cells, cols, name);
        assert_eq!((image_width, image_height), (cols * 10, 20), "{name}");
        assert_glyph_ink(name, &pixels, image_width, left * 10, 0, width * 10, 20);
    }
}

#[test]
fn d02_metrics_cursor_over_combined_and_wide() {
    let profile = resolve_profile(PROFILE_NAME).expect("pinned profile resolves");
    for (name, cells) in [
        (
            "cursor-combined",
            vec![plain_cell(E_ACUTE, 1), plain_cell(" ", 1)],
        ),
        ("cursor-wide", vec![plain_cell(CJK, 2), continuation_cell()]),
    ] {
        let cols = u32::try_from(cells.len()).expect("fixture grid fits");
        let snapshot = snapshot_with(cols, 1, cells, block_cursor_at(0, 0));
        let sample = Sample::with_pixels(
            &pixels_test_id(&format!("metrics/{name}")),
            &snapshot,
            Observation::empty(),
            profile,
        )
        .expect("cursor over cell renders");
        let contract = sample.render_contract().expect("contract is present");
        assert_eq!(contract.cursor_phase(), CursorPhase::Shown, "{name}");
        let image = sample.image().expect("image is present");
        assert_eq!((image.width(), image.height()), (cols * 10, 20), "{name}");
    }
    // A hidden cursor records the hidden phase instead.
    let snapshot = snapshot_with(
        2,
        1,
        vec![plain_cell("A", 1), plain_cell(" ", 1)],
        hidden_cursor(),
    );
    let sample = Sample::with_pixels(
        &pixels_test_id("metrics/hidden-cursor"),
        &snapshot,
        Observation::empty(),
        profile,
    )
    .expect("hidden cursor renders");
    assert_eq!(
        sample
            .render_contract()
            .expect("contract is present")
            .cursor_phase(),
        CursorPhase::Hidden
    );
}

/// The C04 corpus grid: mandatory graphemes, continuations, styles, and a
/// cursor over a combined cell.
fn c04_snapshot(cursor: SnapshotCursor) -> TerminalSnapshot {
    snapshot_with(
        8,
        4,
        vec![
            plain_cell("A", 1),
            plain_cell("─", 1),
            plain_cell(E_ACUTE, 1),
            styled_cell(
                A_DIAERESIS,
                1,
                Attrs::ITALIC,
                SnapshotUnderline::None,
                SnapshotColor::Default,
                SnapshotColor::Default,
            ),
            styled_cell(
                "B",
                1,
                Attrs::BOLD,
                SnapshotUnderline::None,
                SnapshotColor::Default,
                SnapshotColor::Default,
            ),
            styled_cell(
                "C",
                1,
                Attrs::BOLD | Attrs::ITALIC,
                SnapshotUnderline::None,
                SnapshotColor::Default,
                SnapshotColor::Default,
            ),
            plain_cell(" ", 1),
            styled_cell(
                "R",
                1,
                Attrs::INVERSE,
                SnapshotUnderline::None,
                SnapshotColor::Default,
                SnapshotColor::Default,
            ),
            plain_cell(CJK, 2),
            continuation_cell(),
            plain_cell("x", 1),
            plain_cell("y", 1),
            plain_cell("z", 1),
            plain_cell(" ", 1),
            plain_cell(" ", 1),
            plain_cell(" ", 1),
            plain_cell(" ", 1),
            plain_cell(" ", 1),
            plain_cell(" ", 1),
            plain_cell(" ", 1),
            plain_cell(" ", 1),
            plain_cell(" ", 1),
            plain_cell(" ", 1),
            plain_cell(CJK, 1),
            plain_cell(E_ACUTE, 1),
            plain_cell(" ", 1),
            plain_cell(" ", 1),
            plain_cell(" ", 1),
            plain_cell(" ", 1),
            plain_cell(" ", 1),
            plain_cell(" ", 1),
            plain_cell(" ", 1),
        ],
        cursor,
    )
}

#[test]
fn d02_c04_sample_keeps_graphemes() {
    let snapshot = c04_snapshot(block_cursor_at(0, 3));
    let sample = Sample::state("views/login", snapshot.clone(), Observation::empty())
        .expect("state sample builds");
    assert_eq!(sample.gate(), Gate::State);
    assert!(sample.render_contract().is_none());
    assert!(sample.image().is_none());
    let bytes = sample.to_bytes().expect("sample serializes");
    // Stable field order starts every file the same way.
    assert!(
        bytes.starts_with(b"{\"schema\":\"tuiscotti.sample.v1\",\"test_id\":\"views/login\""),
        "field order is stable"
    );
    let parsed = Sample::from_bytes(&bytes).expect("sample parses");
    assert_eq!(parsed.test_id(), "views/login");
    let cells = parsed.snapshot().cells();
    assert_eq!(cells.len(), 32);
    assert_eq!(cells[2].text(), E_ACUTE);
    assert_eq!(cells[3].text(), A_DIAERESIS);
    assert_eq!(cells[8].text(), CJK);
    assert_eq!(cells[8].width(), 2);
    assert_eq!(cells[9].text(), "");
    assert!(cells[9].is_continuation());
    assert_eq!(cells[23].text(), CJK);
    assert_eq!(cells[23].width(), 1, "edge wide cell keeps one column");
    assert_eq!(cells[24].text(), E_ACUTE);
    assert_eq!(parsed.snapshot().cursor(), snapshot.cursor());
    // The candidate ID binds the file bytes: 64 hex characters.
    let candidate = Sample::candidate_id(&bytes);
    assert_eq!(candidate.len(), 64);
    assert!(candidate.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn d02_c04_pixels_sample_keeps_graphemes() {
    let profile = resolve_profile(PROFILE_NAME).expect("pinned profile resolves");
    let snapshot = c04_snapshot(block_cursor_at(0, 1));
    let sample = Sample::with_pixels(
        &pixels_test_id("views/login"),
        &snapshot,
        Observation::empty(),
        profile,
    )
    .expect("pixels sample builds");
    assert_eq!(sample.gate(), Gate::StateAndPixels);
    let image = sample.image().expect("image is present");
    assert_eq!((image.width(), image.height()), (80, 80));
    assert_eq!(image.format(), ImageFormat::Png);
    let bytes = sample.to_bytes().expect("sample serializes");
    let parsed = Sample::from_bytes(&bytes).expect("sample parses");
    assert_eq!(parsed.snapshot().cells()[8].text(), CJK);
    assert_eq!(
        parsed.image().expect("image").rgba_sha256(),
        image.rgba_sha256()
    );
}

#[test]
fn d02_c04_view_chain_keeps_graphemes() {
    let text = format!("{E_ACUTE}{A_DIAERESIS}{CJK}──");
    let snapshot = capture_view(10, 3, &ViewSettings::builtin(), |frame: &mut Frame<'_>| {
        frame.render_widget(Paragraph::new(text.clone()), frame.area());
    })
    .expect("view captures without a child or PTY");
    let sample =
        Sample::state("views/login", snapshot, Observation::empty()).expect("state sample builds");
    let bytes = sample.to_bytes().expect("sample serializes");
    let parsed = Sample::from_bytes(&bytes).expect("sample parses");
    let row: String = parsed.snapshot().cells()[0..10]
        .iter()
        .map(SnapshotCell::text)
        .collect();
    assert!(
        row.starts_with(&format!("{E_ACUTE}{A_DIAERESIS}{CJK}──")),
        "row keeps graphemes in order: {row:?}"
    );
    // Buffer conversion with an explicit cursor keeps a combined cell too.
    let mut buffer = ratatui::buffer::Buffer::empty(ratatui::layout::Rect::new(0, 0, 2, 1));
    buffer.set_string(0, 0, E_ACUTE, ratatui::style::Style::default());
    let converted = tuiscotti::view::snapshot_from_buffer(
        &buffer,
        &ViewSettings::builtin().with_cursor(ViewCursor::new(
            0,
            0,
            true,
            SnapshotCursorShape::Block,
            Rgb::new(192, 192, 192),
        )),
    )
    .expect("buffer converts");
    assert_eq!(converted.cells()[0].text(), E_ACUTE);
    // A combining mark takes one column: the next cell stays a plain space.
    assert_eq!(converted.cells()[1].text(), " ");
}

/// The C05 corpus fixture: styles, an explicit underline color (U11),
/// reverse video, a restyled palette and defaults, and a visible cursor.
fn c05_snapshot() -> TerminalSnapshot {
    let mut palette = test_palette();
    palette[1] = Rgb::new(10, 20, 30);
    snapshot_with_colors(
        6,
        1,
        vec![
            styled_cell(
                "B",
                1,
                Attrs::BOLD,
                SnapshotUnderline::None,
                SnapshotColor::Default,
                SnapshotColor::Indexed(1),
            ),
            styled_cell(
                "I",
                1,
                Attrs::ITALIC,
                SnapshotUnderline::None,
                SnapshotColor::Default,
                SnapshotColor::Default,
            ),
            styled_cell(
                "U",
                1,
                Attrs::empty(),
                SnapshotUnderline::Single,
                SnapshotColor::Rgb(255, 0, 0),
                SnapshotColor::Default,
            ),
            styled_cell(
                "R",
                1,
                Attrs::INVERSE,
                SnapshotUnderline::None,
                SnapshotColor::Default,
                SnapshotColor::Default,
            ),
            plain_cell(E_ACUTE, 1),
            plain_cell(" ", 1),
        ],
        block_cursor_at(4, 0),
        &palette,
        Rgb::new(200, 201, 202),
        Rgb::new(3, 4, 5),
        Rgb::new(10, 11, 12),
        Some("login".to_string()),
    )
}

#[test]
fn d02_c05_sample_keeps_styles() {
    let snapshot = c05_snapshot();
    let sample = Sample::state("views/style", snapshot.clone(), Observation::empty())
        .expect("state sample builds");
    let bytes = sample.to_bytes().expect("sample serializes");
    let parsed = Sample::from_bytes(&bytes).expect("sample parses");
    let styled = parsed.snapshot();
    assert_eq!(styled.cells()[0].fg(), SnapshotColor::Indexed(1));
    assert!(styled.cells()[0].has(Attrs::BOLD));
    assert!(styled.cells()[1].has(Attrs::ITALIC));
    assert_eq!(styled.cells()[2].underline(), SnapshotUnderline::Single);
    assert_eq!(
        styled.cells()[2].underline_color(),
        SnapshotColor::Rgb(255, 0, 0)
    );
    assert!(styled.cells()[3].has(Attrs::INVERSE));
    assert_eq!(styled.cells()[4].text(), E_ACUTE);
    assert_eq!(styled.palette()[1], Rgb::new(10, 20, 30));
    assert_eq!(styled.default_foreground(), Rgb::new(200, 201, 202));
    assert_eq!(styled.default_background(), Rgb::new(3, 4, 5));
    assert_eq!(styled.cursor(), snapshot.cursor());
    assert!(styled.cursor().visible());
    assert_eq!(styled.title(), Some("login"));
    assert_eq!(
        parsed.state_digest().expect("digest"),
        sample.state_digest().expect("digest")
    );
}

#[test]
fn d02_c05_styles_reach_pixels() {
    let profile = resolve_profile(PROFILE_NAME).expect("pinned profile resolves");
    let styled = c05_snapshot();
    let pixels = |snapshot: &TerminalSnapshot, logical: &str| {
        Sample::with_pixels(
            &pixels_test_id(logical),
            snapshot,
            Observation::empty(),
            profile,
        )
        .expect("styled snapshot renders")
        .state_digest()
        .expect("digest")
    };
    let styled_digest = pixels(&styled, "c05/styled");
    // Plain text, no underline, and no reverse all paint differently.
    let mut plain_cells = styled.cells().to_vec();
    for cell in &mut plain_cells {
        *cell = plain_cell(cell.text(), cell.width());
    }
    let plain = snapshot_with(6, 1, plain_cells, hidden_cursor());
    assert_ne!(pixels(&plain, "c05/plain"), styled_digest, "styles paint");
    // A restyled palette entry moves the contract digest and the pixels.
    let repainted = styled.cells().to_vec();
    let mut palette = test_palette();
    palette[1] = Rgb::new(30, 20, 10);
    let repainted_snapshot = snapshot_with_colors(
        6,
        1,
        repainted,
        block_cursor_at(4, 0),
        &palette,
        Rgb::new(200, 201, 202),
        Rgb::new(3, 4, 5),
        Rgb::new(10, 11, 12),
        Some("login".to_string()),
    );
    assert_ne!(
        pixels(&repainted_snapshot, "c05/repainted"),
        styled_digest,
        "palette paints"
    );
}

/// The C06 fixture: plain ASCII the pinned faces cover.
fn c06_snapshot() -> TerminalSnapshot {
    snapshot_with(
        4,
        1,
        vec![
            plain_cell("h", 1),
            plain_cell("o", 1),
            plain_cell("s", 1),
            plain_cell("t", 1),
        ],
        hidden_cursor(),
    )
}

/// C06: strict profile output ignores host font availability. The rogue
/// environment runs in a child process (its own `FONTCONFIG_PATH` and
/// family selection), so no test mutates process-wide variables.
#[test]
fn d02_c06_pinned_output_survives_changed_host_fonts() {
    if std::env::var_os("TUISCOTTI_C06_ROGUE").is_some() {
        let profile = resolve_profile(PROFILE_NAME).expect("pinned profile resolves");
        let sample = Sample::with_pixels(
            &pixels_test_id("c06/host"),
            &c06_snapshot(),
            Observation::empty(),
            profile,
        )
        .expect("pinned render under rogue host fonts");
        println!("RGBA:{}", sample.image().expect("image").rgba_sha256());
        println!("STATE:{}", sample.state_digest().expect("state digest"));
        return;
    }
    let profile = resolve_profile(PROFILE_NAME).expect("pinned profile resolves");
    let sample = Sample::with_pixels(
        &pixels_test_id("c06/host"),
        &c06_snapshot(),
        Observation::empty(),
        profile,
    )
    .expect("pinned baseline renders");
    let expected_rgba = sample.image().expect("image").rgba_sha256().to_string();
    let expected_state = sample.state_digest().expect("state digest");
    let empty_fonts = tempfile::tempdir().expect("fontconfig sandbox builds");
    let child =
        std::process::Command::new(std::env::current_exe().expect("current test binary resolves"))
            .arg("--exact")
            .arg("d02_c06_pinned_output_survives_changed_host_fonts")
            .arg("--nocapture")
            .env("TUISCOTTI_C06_ROGUE", "1")
            .env("FONTCONFIG_PATH", empty_fonts.path())
            .env("TUI_TEST_RECORDING_FONT_FAMILIES", "NoSuchFamily-XYZ")
            .output()
            .expect("rogue child runs");
    assert!(
        child.status.success(),
        "rogue child passes: {}",
        String::from_utf8_lossy(&child.stderr)
    );
    let stdout = String::from_utf8(child.stdout).expect("child output is UTF-8");
    assert!(
        stdout.contains(&format!("RGBA:{expected_rgba}")),
        "host fonts cannot change strict pixels"
    );
    assert!(
        stdout.contains(&format!("STATE:{expected_state}")),
        "host fonts cannot change the compared state"
    );
}

#[test]
fn d02_c07_observation_metadata_ignored_in_compared_state() {
    let snapshot = snapshot_with(
        2,
        1,
        vec![plain_cell("A", 1), plain_cell(" ", 1)],
        hidden_cursor(),
    );
    let first = Sample::state("views/meta", snapshot.clone(), Observation::noted("first"))
        .expect("first sample builds");
    let second = Sample::state("views/meta", snapshot.clone(), Observation::noted("second"))
        .expect("second sample builds");
    // Equal visual state: the observation note never joins the digest.
    assert_eq!(
        first.state_digest().expect("digest"),
        second.state_digest().expect("digest")
    );
    // Different file bytes: the note still moves the candidate ID.
    assert_ne!(
        Sample::candidate_id(&first.to_bytes().expect("bytes")),
        Sample::candidate_id(&second.to_bytes().expect("bytes"))
    );
    // Upstream snapshot metadata is observation too: a live-stamped
    // snapshot compares equal to the offline default.
    let stamped = TerminalSnapshot::try_new(SnapshotParts {
        cols: snapshot.cols(),
        rows: snapshot.rows(),
        scope: SnapshotScope::Viewport,
        viewport_origin: 0,
        cells: snapshot.cells().to_vec(),
        palette: test_palette(),
        default_foreground: Rgb::new(192, 192, 192),
        default_background: Rgb::new(0, 0, 0),
        default_cursor: Rgb::new(192, 192, 192),
        cursor: hidden_cursor(),
        title: None,
        metadata: SnapshotMetadata {
            captured_at_ms: 1_792_000_000_000,
            revision: 7,
            backend: "ghostty".to_string(),
            session: "s".to_string(),
            source: SnapshotSource::Live,
        },
    })
    .expect("stamped snapshot is valid");
    let stamped_sample =
        Sample::state("views/meta", stamped, Observation::empty()).expect("stamped sample builds");
    assert_eq!(
        first.state_digest().expect("digest"),
        stamped_sample.state_digest().expect("digest"),
        "capture metadata never joins the compared state"
    );
    // One changed cell moves the compared state.
    let changed = snapshot_with(
        2,
        1,
        vec![plain_cell("B", 1), plain_cell(" ", 1)],
        hidden_cursor(),
    );
    let changed_sample = Sample::state("views/meta", changed, Observation::noted("first"))
        .expect("changed sample builds");
    assert_ne!(
        first.state_digest().expect("digest"),
        changed_sample.state_digest().expect("digest"),
        "visual changes move the compared state"
    );
}

/// SHA-256 hex over raw bytes, independently of the sample module.
fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 15) as usize] as char);
    }
    out
}

/// Re-encode decoded pixels with different PNG settings: different bytes,
/// identical pixels.
#[expect(clippy::expect_used, reason = "fixture re-encode succeeds")]
fn reencode_alt(pixels: &[u8], width: u32, height: u32) -> Vec<u8> {
    use image::ImageEncoder as _;
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    let mut bytes = Vec::new();
    PngEncoder::new_with_quality(&mut bytes, CompressionType::Best, FilterType::NoFilter)
        .write_image(pixels, width, height, image::ExtendedColorType::Rgba8)
        .expect("re-encode succeeds");
    bytes
}

#[test]
fn d02_c08_rgba_digests_survive_png_remetadata() {
    let profile = resolve_profile(PROFILE_NAME).expect("pinned profile resolves");
    let sample = Sample::with_pixels(
        &pixels_test_id("views/remeta"),
        &c06_snapshot(),
        Observation::empty(),
        profile,
    )
    .expect("pixels sample builds");
    let image = sample.image().expect("image is present").clone();
    let (raw, width, height, pixels) = decode_image(&image);
    assert_eq!((width, height), (image.width(), image.height()));
    // Same pixels, different encoding: the PNG digest moves, the RGBA
    // digest and the compared state stay.
    let alt_raw = reencode_alt(&pixels, width, height);
    assert_ne!(alt_raw, raw, "different settings encode different bytes");
    let bytes = sample.to_bytes().expect("sample serializes");
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("sample is JSON");
    value["image"]["png_base64"] = base64::engine::general_purpose::STANDARD
        .encode(&alt_raw)
        .into();
    value["image"]["png_sha256"] = hex_digest(&alt_raw).into();
    let alt_bytes = serde_json::to_vec(&value).expect("surgery serializes");
    let alt = Sample::from_bytes(&alt_bytes).expect("re-encoded sample loads");
    assert_eq!(
        alt.image().expect("image").rgba_sha256(),
        image.rgba_sha256()
    );
    assert_ne!(alt.image().expect("image").png_sha256(), image.png_sha256());
    assert_eq!(
        alt.state_digest().expect("digest"),
        sample.state_digest().expect("digest"),
        "PNG metadata never joins the compared state"
    );
    // One changed channel fails to load: the RGBA digest binds pixels.
    let mut flipped = pixels.clone();
    flipped[0] ^= 1;
    let flipped_raw = reencode_alt(&flipped, width, height);
    let mut bad = value;
    bad["image"]["png_base64"] = base64::engine::general_purpose::STANDARD
        .encode(&flipped_raw)
        .into();
    bad["image"]["png_sha256"] = hex_digest(&flipped_raw).into();
    let bad_bytes = serde_json::to_vec(&bad).expect("surgery serializes");
    assert!(
        matches!(
            Sample::from_bytes(&bad_bytes),
            Err(SampleError::DigestMismatch {
                field: "rgba_sha256",
                ..
            })
        ),
        "one changed channel fails the RGBA digest"
    );
}

/// The recorded contract matches the profile that rendered the sample.
#[expect(clippy::expect_used, reason = "fixtures carry contracts")]
fn assert_contract_agrees(sample: &Sample) {
    use tuiscotti::sample::RenderContract;
    let contract: &RenderContract = sample.render_contract().expect("contract is present");
    assert_eq!(contract.profile(), PROFILE_NAME);
    assert_eq!(contract.contract_version(), CONTRACT_VERSION);
    assert_eq!(contract.raster_policy(), "pinned-v1");
    let version_parts: Vec<&str> = contract.renderer_version().split('.').collect();
    assert_eq!(version_parts.len(), 3, "renderer version is release-shaped");
    assert!(
        version_parts.iter().all(|part| part.parse::<u64>().is_ok()),
        "renderer version is numeric"
    );
    assert_eq!(
        (contract.cell_width(), contract.cell_height()),
        (10.0, 20.0)
    );
    assert!(!contract.title_bar());
    assert_eq!(contract.platform_key(), platform_key());
    assert_eq!(contract.palette_digest().len(), 64);
    assert_eq!(contract.fonts().len(), 15, "four style slots");
    for (position, font) in contract.fonts().iter().enumerate() {
        assert_eq!(font.fallback_position(), position);
        assert_eq!(font.digest().len(), 64);
    }
    // Geometry is bit-exact: the renderer stores float bits, so compare
    // bits rather than floats.
    assert_eq!(contract.cell_width().to_bits(), 10.0f32.to_bits());
    assert_eq!(contract.cell_height().to_bits(), 20.0f32.to_bits());
    assert_eq!(contract.font_size().to_bits(), 16.0f32.to_bits());
    assert_eq!(contract.baseline().to_bits(), 15.0f32.to_bits());
    assert_eq!(contract.scale().to_bits(), 1.0f64.to_bits());
}

#[test]
fn d02_c10_construction_digests_agree() {
    let profile = resolve_profile(PROFILE_NAME).expect("pinned profile resolves");
    // State sample: candidate ID binds the file bytes; re-parse agrees.
    let state = Sample::state("views/agree", c06_snapshot(), Observation::noted("v1"))
        .expect("state sample builds");
    let state_bytes = state.to_bytes().expect("state serializes");
    assert_eq!(Sample::candidate_id(&state_bytes), hex_digest(&state_bytes));
    let state_reparsed = Sample::from_bytes(&state_bytes).expect("state parses");
    assert_eq!(
        Sample::candidate_id(&state_bytes),
        Sample::candidate_id(&state_reparsed.to_bytes().expect("bytes"))
    );
    assert_eq!(
        state.state_digest().expect("digest"),
        state_reparsed.state_digest().expect("digest")
    );
    // Pixels sample: frame and image data agree with each other.
    let pixels = Sample::with_pixels(
        &pixels_test_id("views/agree"),
        &c06_snapshot(),
        Observation::empty(),
        profile,
    )
    .expect("pixels sample builds");
    assert_contract_agrees(&pixels);
    let image = pixels.image().expect("image is present");
    let (_, width, height, decoded) = decode_image(image);
    assert_eq!((width, height), (40, 20), "zero chrome: grid times cell");
    assert_eq!(decoded.len(), 40 * 20 * 4);
    assert_eq!(image.rgba_sha256(), hex_digest(&decoded));
    assert_eq!(
        image.png_sha256(),
        hex_digest(
            &base64::engine::general_purpose::STANDARD
                .decode(image.png_base64().as_bytes())
                .expect("base64 decodes")
        )
    );
    // File round trip through a tempdir keeps every agreement.
    let dir = tempfile::tempdir().expect("tempdir builds");
    let path = dir.path().join("agree.tuiscotti.json");
    pixels.save(&path).expect("sample saves");
    let loaded = Sample::from_file(&path).expect("sample loads");
    assert_eq!(
        loaded.state_digest().expect("digest"),
        pixels.state_digest().expect("digest")
    );
    assert_eq!(
        Sample::candidate_id(&std::fs::read(&path).expect("file reads")),
        Sample::candidate_id(&pixels.to_bytes().expect("bytes"))
    );
    assert!(matches!(
        Sample::from_file(&dir.path().join("missing.json")),
        Err(SampleError::Io { .. })
    ));
}

#[test]
fn d02_rejects_unsupported_schema_and_unknown_fields() {
    let sample = Sample::state("views/reject", c06_snapshot(), Observation::empty())
        .expect("state sample builds");
    let bytes = sample.to_bytes().expect("sample serializes");
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("sample is JSON");
    // A future schema version is not accepted.
    value["schema"] = "tuiscotti.sample.v2".into();
    assert!(
        matches!(
            Sample::from_bytes(&serde_json::to_vec(&value).expect("bytes")),
            Err(SampleError::UnsupportedSchema { .. })
        ),
        "unsupported schema fails"
    );
    // Unknown fields fail at the envelope and inside nested objects.
    let mut extra = serde_json::from_slice::<serde_json::Value>(&bytes).expect("JSON");
    extra["tomorrow"] = 1.into();
    assert!(
        matches!(
            Sample::from_bytes(&serde_json::to_vec(&extra).expect("bytes")),
            Err(SampleError::UnknownField { .. })
        ),
        "unknown envelope field fails"
    );
    let mut nested = serde_json::from_slice::<serde_json::Value>(&bytes).expect("JSON");
    nested["observation"]["session"] = "s".into();
    assert!(
        matches!(
            Sample::from_bytes(&serde_json::to_vec(&nested).expect("bytes")),
            Err(SampleError::UnknownField { .. })
        ),
        "unknown nested field fails"
    );
    // An unknown gate value is a format failure, not a silent default.
    let mut gate = serde_json::from_slice::<serde_json::Value>(&bytes).expect("JSON");
    gate["gate"] = "pixels-only".into();
    assert!(
        matches!(
            Sample::from_bytes(&serde_json::to_vec(&gate).expect("bytes")),
            Err(SampleError::InvalidFormat { .. })
        ),
        "unknown gate fails"
    );
}

#[test]
fn d02_rejects_gate_evidence_mismatch() {
    let state = Sample::state("views/mismatch", c06_snapshot(), Observation::empty())
        .expect("state sample builds");
    let state_bytes = state.to_bytes().expect("state serializes");
    let mut pixels_claim = serde_json::from_slice::<serde_json::Value>(&state_bytes).expect("JSON");
    pixels_claim["gate"] = "state-and-pixels".into();
    assert!(
        matches!(
            Sample::from_bytes(&serde_json::to_vec(&pixels_claim).expect("bytes")),
            Err(SampleError::GateMismatch { .. })
        ),
        "pixels gate without evidence fails"
    );
    let profile = resolve_profile(PROFILE_NAME).expect("pinned profile resolves");
    let pixels = Sample::with_pixels(
        &pixels_test_id("views/mismatch"),
        &c06_snapshot(),
        Observation::empty(),
        profile,
    )
    .expect("pixels sample builds");
    let pixels_bytes = pixels.to_bytes().expect("pixels serializes");
    let mut state_claim = serde_json::from_slice::<serde_json::Value>(&pixels_bytes).expect("JSON");
    state_claim["gate"] = "state".into();
    assert!(
        matches!(
            Sample::from_bytes(&serde_json::to_vec(&state_claim).expect("bytes")),
            Err(SampleError::GateMismatch { .. })
        ),
        "state gate with evidence fails"
    );
}

#[test]
fn d02_rejects_oversize_dims_before_allocation() {
    let profile = resolve_profile(PROFILE_NAME).expect("pinned profile resolves");
    let sample = Sample::with_pixels(
        &pixels_test_id("views/huge"),
        &c06_snapshot(),
        Observation::empty(),
        profile,
    )
    .expect("pixels sample builds");
    let bytes = sample.to_bytes().expect("sample serializes");
    // One dimension past the per-dimension limit fails before decoding.
    let mut wide = serde_json::from_slice::<serde_json::Value>(&bytes).expect("JSON");
    wide["image"]["width"] = 40_000.into();
    assert!(
        matches!(
            Sample::from_bytes(&serde_json::to_vec(&wide).expect("bytes")),
            Err(SampleError::DimensionTooLarge { .. })
        ),
        "oversize dimension fails before allocation"
    );
    // In-limit dimensions with an over-limit product fail just as early:
    // 30000 by 30000 RGBA needs 3.6 GiB, far past the 256 MiB budget.
    let mut huge = serde_json::from_slice::<serde_json::Value>(&bytes).expect("JSON");
    huge["image"]["width"] = 30_000.into();
    huge["image"]["height"] = 30_000.into();
    assert!(
        matches!(
            Sample::from_bytes(&serde_json::to_vec(&huge).expect("bytes")),
            Err(SampleError::RgbaTooLarge { .. })
        ),
        "over-limit RGBA fails before allocation"
    );
    // Zero dimensions fail without decoding.
    let mut zero = serde_json::from_slice::<serde_json::Value>(&bytes).expect("JSON");
    zero["image"]["width"] = 0.into();
    assert!(
        matches!(
            Sample::from_bytes(&serde_json::to_vec(&zero).expect("bytes")),
            Err(SampleError::DimensionMismatch { .. })
        ),
        "zero dimension fails"
    );
}

#[test]
fn d02_rejects_bad_test_ids() {
    let snapshot = c06_snapshot();
    for bad in [
        "",
        "views//login",
        "/views/login",
        "views/pixels-linux-x86_64",
    ] {
        assert!(
            Sample::state(bad, snapshot.clone(), Observation::empty()).is_err(),
            "{bad:?} fails for the state gate"
        );
    }
    assert!(
        Sample::state(&"v/".repeat(200), snapshot.clone(), Observation::empty()).is_err(),
        "overlong test ID fails"
    );
    assert!(
        Sample::state("views/lögin", snapshot.clone(), Observation::empty()).is_err(),
        "non-ASCII test ID fails"
    );
    assert!(
        Sample::state(
            &format!("views/{}", "s".repeat(65)),
            snapshot.clone(),
            Observation::empty()
        )
        .is_err(),
        "overlong segment fails"
    );
    // The pixels gate needs this platform's suffix, not another platform.
    assert!(
        Sample::with_pixels(
            "views/login/pixels-far-far-away",
            &snapshot,
            Observation::empty(),
            resolve_profile(PROFILE_NAME).expect("profile resolves"),
        )
        .is_err(),
        "foreign platform suffix fails"
    );
    assert!(
        Sample::with_pixels(
            "views/login",
            &snapshot,
            Observation::empty(),
            resolve_profile(PROFILE_NAME).expect("profile resolves"),
        )
        .is_err(),
        "logical test ID fails for the pixels gate"
    );
}

#[test]
fn d02_rejects_missing_glyph() {
    let profile = resolve_profile(PROFILE_NAME).expect("pinned profile resolves");
    let snapshot = snapshot_with(1, 1, vec![plain_cell(CRAB, 1)], hidden_cursor());
    assert_eq!(CRAB.chars().count(), 1, "crab is one scalar");
    let error = Sample::with_pixels(
        &pixels_test_id("views/crab"),
        &snapshot,
        Observation::empty(),
        profile,
    )
    .expect_err("the crab has no pinned face");
    assert!(
        matches!(
            error,
            SampleError::Render {
                source: RenderError::PinnedFont(PinnedFontError::MissingGlyph {
                    character,
                    ..
                }),
            } if character == '🦀'
        ),
        "missing glyph names the crab: {error}"
    );
}

#[test]
fn d02_rejects_unknown_profile() {
    assert!(
        matches!(
            resolve_profile("tuiscotti-pinned-v2"),
            Err(RegistryError::UnknownProfile { .. })
        ),
        "a future profile name fails"
    );
}

#[test]
fn d02_rejects_oversize_file() {
    let bytes = vec![0u8; 64 * 1024 * 1024 + 1];
    assert!(
        matches!(
            Sample::from_bytes(&bytes),
            Err(SampleError::TooLarge {
                what: "sample file",
                ..
            })
        ),
        "past-limit files fail before parsing"
    );
}

/// No `.tmp-*` file may survive beside a saved sample.
#[expect(clippy::expect_used, reason = "test helper reads saved samples")]
fn assert_no_temp_residue(dir: &std::path::Path) {
    for entry in std::fs::read_dir(dir).expect("sample dir lists") {
        let name = entry
            .expect("entry reads")
            .file_name()
            .to_string_lossy()
            .into_owned();
        assert!(
            !name.starts_with(".tmp-"),
            "temporary file {name:?} left behind in {}",
            dir.display()
        );
    }
}

/// The sample file saves atomically: complete bytes, stable across a
/// re-save, with no temporary file left behind; a failing save publishes
/// no truncated sample either.
#[test]
fn d03_sample_saves_atomically_without_temp_residue() {
    let sample = Sample::state("views/atomic", c06_snapshot(), Observation::empty())
        .expect("state sample builds");
    let expected = sample.to_bytes().expect("sample serializes");
    let dir = tempfile::tempdir().expect("tempdir builds");
    let path = dir.path().join("atomic.tuiscotti.json");
    sample.save(&path).expect("sample saves");
    assert_eq!(std::fs::read(&path).expect("sample reads"), expected);
    assert_no_temp_residue(dir.path());
    let loaded = Sample::from_file(&path).expect("sample loads");
    assert_eq!(
        loaded.to_bytes().expect("loaded serializes"),
        expected,
        "saved bytes round-trip"
    );
    sample.save(&path).expect("sample resaves");
    assert_eq!(std::fs::read(&path).expect("sample rereads"), expected);
    assert_no_temp_residue(dir.path());
    // A failing save publishes nothing: no truncated file, no residue.
    let missing = dir.path().join("no-such-dir").join("atomic.tuiscotti.json");
    assert!(
        sample.save(&missing).is_err(),
        "save into a missing directory fails"
    );
    assert!(!missing.exists(), "failed save publishes nothing");
    assert_no_temp_residue(dir.path());
}

/// Re-encode valid sample bytes with one JSON field replaced.
#[expect(clippy::expect_used, reason = "rework validation fixture")]
fn sample_bytes_with_field(sample_bytes: &[u8], field: &str, value: serde_json::Value) -> Vec<u8> {
    let mut object: serde_json::Value =
        serde_json::from_slice(sample_bytes).expect("sample is JSON");
    object[field] = value;
    serde_json::to_vec(&object).expect("surgery serializes")
}

/// A minimal pixels sample on this platform for serde bypass probes.
#[expect(clippy::expect_used, reason = "rework validation fixture")]
fn pixels_sample_bytes() -> Vec<u8> {
    let profile = resolve_profile(PROFILE_NAME).expect("pinned profile resolves");
    Sample::with_pixels(
        &pixels_test_id("r03/pixels"),
        &c06_snapshot(),
        Observation::empty(),
        profile,
    )
    .expect("pixels fixture renders")
    .to_bytes()
    .expect("pixels fixture serializes")
}

/// R3: `serde_json::from_slice::<Sample>` must run the same validation as
/// `from_bytes`. A gate/image mismatch the derived `Deserialize` used to
/// accept must fail through the serde path too.
#[test]
fn r3_serde_path_rejects_gate_image_mismatch() {
    let state_bytes = Sample::state("views/r03", c06_snapshot(), Observation::empty())
        .expect("state fixture builds")
        .to_bytes()
        .expect("state fixture serializes");
    Sample::from_bytes(&state_bytes).expect("control sample parses");
    serde_json::from_slice::<Sample>(&state_bytes).expect("control serde parses");
    let claimed = sample_bytes_with_field(
        &state_bytes,
        "gate",
        serde_json::Value::String("state-and-pixels".to_string()),
    );
    assert!(
        Sample::from_bytes(&claimed).is_err(),
        "from_bytes rejects the gate/image mismatch"
    );
    assert!(
        serde_json::from_slice::<Sample>(&claimed).is_err(),
        "serde_json::from_slice::<Sample> rejects the gate/image mismatch"
    );
}

/// R3: tampered digests must fail through the serde path, not just
/// `from_bytes`.
#[test]
fn r3_serde_path_rejects_tampered_digest() {
    let bytes = pixels_sample_bytes();
    serde_json::from_slice::<Sample>(&bytes).expect("control serde parses");
    let mut object: serde_json::Value = serde_json::from_slice(&bytes).expect("sample is JSON");
    let image = object
        .get_mut("image")
        .expect("pixels sample carries an image");
    image["png_sha256"] = serde_json::Value::String("0".repeat(64));
    let tampered = serde_json::to_vec(&object).expect("surgery serializes");
    assert!(
        Sample::from_bytes(&tampered).is_err(),
        "from_bytes rejects the tampered digest"
    );
    assert!(
        serde_json::from_slice::<Sample>(&tampered).is_err(),
        "serde_json::from_slice::<Sample> rejects the tampered digest"
    );
}

/// R21: saved-sample parsing validates the pixels suffix against the
/// stored contract identity, not the current platform. A sample recorded
/// on another platform parses here when its ID matches its contract.
#[test]
fn r21_saved_sample_parses_against_stored_contract_identity() {
    let bytes = pixels_sample_bytes();
    let mut object: serde_json::Value = serde_json::from_slice(&bytes).expect("sample is JSON");
    let platform = object
        .get("render_contract")
        .and_then(|contract| contract.get("platform_key"))
        .and_then(|platform| platform.as_str())
        .expect("pixels sample carries a platform key")
        .to_string();
    assert!(Sample::from_bytes(&bytes).is_ok(), "control sample parses");
    object["render_contract"]["platform_key"] =
        serde_json::Value::String("far-far-away".to_string());
    let old_id = object
        .get("test_id")
        .and_then(|id| id.as_str())
        .expect("sample carries a test ID")
        .to_string();
    let suffix = format!("/pixels-{platform}");
    let logical = old_id
        .strip_suffix(&suffix)
        .expect("control ID carries the current suffix");
    object["test_id"] = serde_json::Value::String(format!("{logical}/pixels-far-far-away"));
    let foreign = serde_json::to_vec(&object).expect("surgery serializes");
    assert!(
        Sample::from_bytes(&foreign).is_ok(),
        "stored-contract identity validates the foreign-platform sample"
    );
}
