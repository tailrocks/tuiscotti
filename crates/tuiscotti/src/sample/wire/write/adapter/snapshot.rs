use tui_test::{
    SnapshotCell, SnapshotColor, SnapshotCursor, SnapshotCursorShape, SnapshotScope,
    SnapshotUnderline, TerminalSnapshot,
};

use super::super::super::generated::{
    CursorShape as WireCursorShape, EncodeCell, EncodeColor, EncodeCursor, EncodeDefaultColor,
    EncodeMetadata, EncodeRgb, EncodeSnapshot, Scope as WireScope, Source as WireSource,
    Underline as WireUnderline,
};

pub(in crate::sample::wire) struct SnapshotAdapter<'a>(pub(super) &'a TerminalSnapshot);

impl EncodeSnapshot for SnapshotAdapter<'_> {
    fn schema(&self) -> u32 {
        tui_test::SNAPSHOT_SCHEMA_VERSION
    }

    fn cols(&self) -> u32 {
        self.0.cols()
    }

    fn rows(&self) -> u32 {
        self.0.rows()
    }

    fn scope(&self) -> WireScope {
        match self.0.scope() {
            SnapshotScope::Viewport => WireScope::Viewport,
            SnapshotScope::ActiveScreen => WireScope::ActiveScreen,
            SnapshotScope::FullHistory => WireScope::FullHistory,
        }
    }

    fn viewport_origin(&self) -> u64 {
        self.0.viewport_origin()
    }

    type Cells<'a>
        =
        std::iter::Map<std::slice::Iter<'a, SnapshotCell>, fn(&'a SnapshotCell) -> CellAdapter<'a>>
    where
        Self: 'a;
    type CellsItem<'a>
        = CellAdapter<'a>
    where
        Self: 'a;
    fn cells(&self) -> Self::Cells<'_> {
        fn adapt(cell: &SnapshotCell) -> CellAdapter<'_> {
            CellAdapter(cell)
        }
        self.0.cells().iter().map(adapt)
    }

    type Palette<'a>
        = std::iter::Map<
        std::iter::Copied<std::slice::Iter<'a, tui_test::profile::Rgb>>,
        fn(tui_test::profile::Rgb) -> RgbAdapter,
    >
    where
        Self: 'a;
    type PaletteItem<'a>
        = RgbAdapter
    where
        Self: 'a;
    fn palette(&self) -> Self::Palette<'_> {
        fn adapt(rgb: tui_test::profile::Rgb) -> RgbAdapter {
            RgbAdapter(rgb)
        }
        self.0.palette().iter().copied().map(adapt)
    }

    type DefaultForeground<'a>
        = RgbAdapter
    where
        Self: 'a;
    fn default_foreground(&self) -> Self::DefaultForeground<'_> {
        RgbAdapter(self.0.default_foreground())
    }

    type DefaultBackground<'a>
        = RgbAdapter
    where
        Self: 'a;
    fn default_background(&self) -> Self::DefaultBackground<'_> {
        RgbAdapter(self.0.default_background())
    }

    type DefaultCursor<'a>
        = RgbAdapter
    where
        Self: 'a;
    fn default_cursor(&self) -> Self::DefaultCursor<'_> {
        RgbAdapter(self.0.default_cursor())
    }

    type Cursor<'a>
        = CursorAdapter<'a>
    where
        Self: 'a;
    fn cursor(&self) -> Self::Cursor<'_> {
        CursorAdapter(self.0.cursor())
    }

    fn title(&self) -> Option<&str> {
        self.0.title()
    }

    type Metadata<'a>
        = MetadataAdapter<'a>
    where
        Self: 'a;
    fn metadata(&self) -> Self::Metadata<'_> {
        MetadataAdapter(self.0.metadata())
    }
}

pub(in crate::sample::wire) struct CellAdapter<'a>(&'a SnapshotCell);

impl EncodeCell for CellAdapter<'_> {
    fn text(&self) -> &str {
        self.0.text()
    }
    fn width(&self) -> u32 {
        u32::from(self.0.width())
    }
    type Fg<'a>
        = ColorAdapter
    where
        Self: 'a;
    fn fg(&self) -> Self::Fg<'_> {
        ColorAdapter(self.0.fg())
    }
    type Bg<'a>
        = ColorAdapter
    where
        Self: 'a;
    fn bg(&self) -> Self::Bg<'_> {
        ColorAdapter(self.0.bg())
    }
    fn bold(&self) -> bool {
        self.0.has(tui_test::terminal::cell::Attrs::BOLD)
    }
    fn dim(&self) -> bool {
        self.0.has(tui_test::terminal::cell::Attrs::DIM)
    }
    fn italic(&self) -> bool {
        self.0.has(tui_test::terminal::cell::Attrs::ITALIC)
    }
    fn inverse(&self) -> bool {
        self.0.has(tui_test::terminal::cell::Attrs::INVERSE)
    }
    fn invisible(&self) -> bool {
        self.0.has(tui_test::terminal::cell::Attrs::INVISIBLE)
    }
    fn strike(&self) -> bool {
        self.0.has(tui_test::terminal::cell::Attrs::STRIKE)
    }
    fn blink(&self) -> bool {
        self.0.has(tui_test::terminal::cell::Attrs::BLINK)
    }
    fn underline(&self) -> WireUnderline {
        match self.0.underline() {
            SnapshotUnderline::None => WireUnderline::None,
            SnapshotUnderline::Single => WireUnderline::Single,
            SnapshotUnderline::Double => WireUnderline::Double,
            SnapshotUnderline::Curly => WireUnderline::Curly,
            SnapshotUnderline::Dotted => WireUnderline::Dotted,
            SnapshotUnderline::Dashed => WireUnderline::Dashed,
        }
    }
    type UnderlineColor<'a>
        = ColorAdapter
    where
        Self: 'a;
    fn underline_color(&self) -> Self::UnderlineColor<'_> {
        ColorAdapter(self.0.underline_color())
    }
    fn link_uri(&self) -> Option<&str> {
        self.0.link_uri()
    }
    fn link_id(&self) -> Option<&str> {
        self.0.link_id()
    }
}

#[derive(Clone, Copy)]
pub(in crate::sample::wire) struct ColorAdapter(SnapshotColor);
impl EncodeColor for ColorAdapter {
    type DefaultColor<'a>
        = DefaultColorAdapter
    where
        Self: 'a;
    fn default_color(&self) -> Option<Self::DefaultColor<'_>> {
        matches!(self.0, SnapshotColor::Default).then_some(DefaultColorAdapter)
    }
    fn indexed(&self) -> Option<u32> {
        match self.0 {
            SnapshotColor::Indexed(value) => Some(u32::from(value)),
            _ => None,
        }
    }
    type Rgb<'a>
        = RgbAdapter
    where
        Self: 'a;
    fn rgb(&self) -> Option<Self::Rgb<'_>> {
        match self.0 {
            SnapshotColor::Rgb(r, g, b) => Some(RgbAdapter(tui_test::profile::Rgb::new(r, g, b))),
            _ => None,
        }
    }
}

pub(in crate::sample::wire) struct DefaultColorAdapter;
impl EncodeDefaultColor for DefaultColorAdapter {}

#[derive(Clone, Copy)]
pub(in crate::sample::wire) struct RgbAdapter(tui_test::profile::Rgb);
impl EncodeRgb for RgbAdapter {
    fn red(&self) -> u32 {
        u32::from(self.0.r)
    }
    fn green(&self) -> u32 {
        u32::from(self.0.g)
    }
    fn blue(&self) -> u32 {
        u32::from(self.0.b)
    }
}

pub(in crate::sample::wire) struct CursorAdapter<'a>(&'a SnapshotCursor);
impl EncodeCursor for CursorAdapter<'_> {
    fn col(&self) -> u32 {
        self.0.col()
    }
    fn row(&self) -> u32 {
        self.0.row()
    }
    fn in_capture(&self) -> bool {
        self.0.in_capture()
    }
    fn visible(&self) -> bool {
        self.0.visible()
    }
    fn shape(&self) -> WireCursorShape {
        match self.0.shape() {
            SnapshotCursorShape::Block => WireCursorShape::Block,
            SnapshotCursorShape::Underline => WireCursorShape::Underline,
            SnapshotCursorShape::Bar => WireCursorShape::Bar,
        }
    }
    type Color<'a>
        = RgbAdapter
    where
        Self: 'a;
    fn color(&self) -> Self::Color<'_> {
        RgbAdapter(self.0.color())
    }
    fn blink(&self) -> Option<bool> {
        self.0.blink()
    }
}

pub(in crate::sample::wire) struct MetadataAdapter<'a>(&'a tui_test::SnapshotMetadata);
impl EncodeMetadata for MetadataAdapter<'_> {
    fn captured_at_ms(&self) -> u64 {
        self.0.captured_at_ms
    }
    fn revision(&self) -> u64 {
        self.0.revision
    }
    fn backend(&self) -> &str {
        &self.0.backend
    }
    fn session(&self) -> &str {
        &self.0.session
    }
    fn source(&self) -> WireSource {
        match self.0.source {
            tui_test::SnapshotSource::Live => WireSource::Live,
            tui_test::SnapshotSource::Offline => WireSource::Offline,
        }
    }
}
