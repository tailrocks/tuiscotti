//! Pure-view capture: Ratatui application views into the upstream snapshot model.
//!
//! A pure-view test renders an application view in the test process. It starts
//! no shell, child process, PTY, daemon, or terminal session. The only inputs
//! are the caller-supplied draw closure, the caller-supplied view settings,
//! and the Ratatui buffer they produce.
//!
//! Two entry points share one conversion:
//!
//! - [`capture_view`] renders a production draw closure through a Ratatui
//!   `TestBackend` and converts the result.
//! - [`snapshot_from_buffer`] converts a caller-supplied Ratatui buffer with
//!   explicit cursor state.
//!
//! Both pass the converted cells through the upstream validated constructor
//! [`TerminalSnapshot::try_new`], so one snapshot model serves live tests and
//! production-view tests.

use std::fmt::{Display, Formatter};

use ratatui::buffer::{Buffer, CellWidth};
use ratatui::layout::Position;
use ratatui::{Frame, Terminal, backend::TestBackend};
use tui_test::profile::Rgb;
use tui_test::terminal::cell::Attrs;
use tui_test::{
    MAX_SNAPSHOT_CELLS, MAX_SNAPSHOT_DIM, SnapshotCell, SnapshotColor, SnapshotCursor,
    SnapshotCursorShape, SnapshotError, SnapshotMetadata, SnapshotParts, SnapshotScope,
    SnapshotUnderline, TerminalSnapshot,
};

/// Result of a pure-view capture or conversion.
pub type ViewResult<T> = Result<T, ViewError>;

/// Every failure a pure-view capture or conversion can report, with sources retained.
#[derive(Debug)]
#[non_exhaustive]
pub enum ViewError {
    /// The requested grid has a zero column or row count.
    EmptyDimensions,
    /// A buffer area coordinate plus its offset does not fit a 16-bit position.
    AreaOverflow,
    /// A grid position inside the buffer area has no cell.
    CellOutOfBounds {
        /// Buffer-local column of the missing cell.
        x: u32,
        /// Buffer-local row of the missing cell.
        y: u32,
    },
    /// A column covered by a wide leading cell carries its own grapheme
    /// instead of continuing that cell.
    CoveredCellOccupied {
        /// Buffer-local column of the occupying cell.
        x: u32,
        /// Buffer-local row of the occupying cell.
        y: u32,
    },
    /// A cell outside any wide-cell coverage has zero width, so it cannot
    /// be a continuation: only a covering leading cell makes a
    /// zero-width cell valid.
    UncoveredZeroWidthCell {
        /// Buffer-local column of the zero-width cell.
        x: u32,
        /// Buffer-local row of the zero-width cell.
        y: u32,
    },
    /// A wide leading cell is wider than the columns left in its row, so its
    /// continuation would fall outside the grid.
    LeadingCellOverhang {
        /// Buffer-local column of the overhanging leading cell.
        x: u32,
        /// Buffer-local row of the overhanging leading cell.
        y: u32,
        /// Terminal columns the leading cell claims.
        width: u16,
    },
    /// The upstream validated constructor rejected the converted snapshot.
    Snapshot(SnapshotError),
}

impl Display for ViewError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyDimensions => f.write_str("view grid dimensions must be nonzero"),
            Self::AreaOverflow => {
                f.write_str("buffer area coordinate does not fit a 16-bit position")
            }
            Self::CellOutOfBounds { x, y } => {
                write!(f, "buffer has no cell at ({x}, {y})")
            }
            Self::CoveredCellOccupied { x, y } => {
                write!(
                    f,
                    "covered cell at ({x}, {y}) carries its own grapheme instead of continuing the wide cell"
                )
            }
            Self::UncoveredZeroWidthCell { x, y } => {
                write!(
                    f,
                    "cell at ({x}, {y}) has zero width without a covering wide cell"
                )
            }
            Self::LeadingCellOverhang { x, y, width } => {
                write!(
                    f,
                    "wide leading cell at ({x}, {y}) claims {width} columns past the row end"
                )
            }
            Self::Snapshot(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ViewError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Snapshot(error) => Some(error),
            _ => None,
        }
    }
}

impl From<SnapshotError> for ViewError {
    fn from(error: SnapshotError) -> Self {
        Self::Snapshot(error)
    }
}

/// Explicit cursor state for a pure-view capture.
///
/// Ratatui buffers carry no cursor, so buffer-only conversion takes the cursor
/// from these caller-supplied settings. The color doubles as the resolved
/// default cursor color in the snapshot. Blink intent stays unavailable:
/// Ratatui reports no blink state, so conversion records `None` rather than a
/// guessed value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewCursor {
    col: u32,
    row: u32,
    visible: bool,
    shape: SnapshotCursorShape,
    color: Rgb,
}

impl ViewCursor {
    /// Capture explicit cursor state.
    ///
    /// The position uses buffer-local coordinates: column and row relative to
    /// the buffer area origin, matching [`snapshot_from_buffer`].
    #[must_use]
    pub const fn new(
        col: u32,
        row: u32,
        visible: bool,
        shape: SnapshotCursorShape,
        color: Rgb,
    ) -> Self {
        Self {
            col,
            row,
            visible,
            shape,
            color,
        }
    }

    /// Buffer-local cursor column.
    #[must_use]
    pub const fn col(self) -> u32 {
        self.col
    }

    /// Buffer-local cursor row.
    #[must_use]
    pub const fn row(self) -> u32 {
        self.row
    }

    /// Whether the terminal draws the cursor.
    #[must_use]
    pub const fn visible(self) -> bool {
        self.visible
    }

    /// The shape the terminal draws the cursor as.
    #[must_use]
    pub const fn shape(self) -> SnapshotCursorShape {
        self.shape
    }

    /// The resolved cursor color.
    #[must_use]
    pub const fn color(self) -> Rgb {
        self.color
    }
}

/// Caller-supplied view settings for a pure-view capture.
///
/// The palette and defaults are copied into the snapshot as caller-supplied
/// view settings. They are never observed terminal colors: no child, PTY, or
/// session runs during conversion, so nothing exists to observe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewSettings {
    palette: [Rgb; 256],
    default_foreground: Rgb,
    default_background: Rgb,
    cursor: ViewCursor,
}

impl ViewSettings {
    /// Capture caller-supplied view settings with explicit cursor state.
    #[must_use]
    pub const fn new(
        palette: [Rgb; 256],
        default_foreground: Rgb,
        default_background: Rgb,
        cursor: ViewCursor,
    ) -> Self {
        Self {
            palette,
            default_foreground,
            default_background,
            cursor,
        }
    }

    /// The built-in view profile: the fixed view palette from the migration
    /// API record with a hidden block cursor at the origin.
    ///
    /// The palette copies the recorded values byte for byte. Do not
    /// recalculate it from a later dependency version.
    #[must_use]
    pub const fn builtin() -> Self {
        Self {
            palette: build_view_palette(),
            default_foreground: Rgb::new(192, 192, 192),
            default_background: Rgb::new(0, 0, 0),
            cursor: ViewCursor::new(
                0,
                0,
                false,
                SnapshotCursorShape::Block,
                Rgb::new(192, 192, 192),
            ),
        }
    }

    /// Replace the resolved palette.
    #[must_use]
    pub const fn with_palette(mut self, palette: [Rgb; 256]) -> Self {
        self.palette = palette;
        self
    }

    /// Replace the resolved default foreground and background.
    #[must_use]
    pub const fn with_defaults(mut self, default_foreground: Rgb, default_background: Rgb) -> Self {
        self.default_foreground = default_foreground;
        self.default_background = default_background;
        self
    }

    /// Replace the explicit cursor state.
    #[must_use]
    pub const fn with_cursor(mut self, cursor: ViewCursor) -> Self {
        self.cursor = cursor;
        self
    }

    /// Every resolved palette entry, indices 0 through 255.
    #[must_use]
    pub const fn palette(&self) -> &[Rgb; 256] {
        &self.palette
    }

    /// The resolved default foreground.
    #[must_use]
    pub const fn default_foreground(&self) -> Rgb {
        self.default_foreground
    }

    /// The resolved default background.
    #[must_use]
    pub const fn default_background(&self) -> Rgb {
        self.default_background
    }

    /// The explicit cursor state.
    #[must_use]
    pub const fn cursor(&self) -> ViewCursor {
        self.cursor
    }
}

/// The fixed view palette as recorded `#rrggbb` strings.
///
/// Source: the migration API record, copied from an in-memory upstream
/// emulator read through `Emulator::colors()` with no child, PTY, or session.
/// The canonical digest over these values is
/// `92757b147e00dec394b10b7be2fa42f782f0d30325a5922570080f91d60f641e`;
/// `view_palette_digest` in the test suite pins it.
const VIEW_PALETTE_HEX: [&str; 256] = [
    "#000000", "#800000", "#008000", "#808000", "#000080", "#800080", "#008080", "#c0c0c0",
    "#808080", "#ff0000", "#00ff00", "#ffff00", "#0000ff", "#ff00ff", "#00ffff", "#ffffff",
    "#000000", "#00005f", "#000087", "#0000af", "#0000d7", "#0000ff", "#005f00", "#005f5f",
    "#005f87", "#005faf", "#005fd7", "#005fff", "#008700", "#00875f", "#008787", "#0087af",
    "#0087d7", "#0087ff", "#00af00", "#00af5f", "#00af87", "#00afaf", "#00afd7", "#00afff",
    "#00d700", "#00d75f", "#00d787", "#00d7af", "#00d7d7", "#00d7ff", "#00ff00", "#00ff5f",
    "#00ff87", "#00ffaf", "#00ffd7", "#00ffff", "#5f0000", "#5f005f", "#5f0087", "#5f00af",
    "#5f00d7", "#5f00ff", "#5f5f00", "#5f5f5f", "#5f5f87", "#5f5faf", "#5f5fd7", "#5f5fff",
    "#5f8700", "#5f875f", "#5f8787", "#5f87af", "#5f87d7", "#5f87ff", "#5faf00", "#5faf5f",
    "#5faf87", "#5fafaf", "#5fafd7", "#5fafff", "#5fd700", "#5fd75f", "#5fd787", "#5fd7af",
    "#5fd7d7", "#5fd7ff", "#5fff00", "#5fff5f", "#5fff87", "#5fffaf", "#5fffd7", "#5fffff",
    "#870000", "#87005f", "#870087", "#8700af", "#8700d7", "#8700ff", "#875f00", "#875f5f",
    "#875f87", "#875faf", "#875fd7", "#875fff", "#878700", "#87875f", "#878787", "#8787af",
    "#8787d7", "#8787ff", "#87af00", "#87af5f", "#87af87", "#87afaf", "#87afd7", "#87afff",
    "#87d700", "#87d75f", "#87d787", "#87d7af", "#87d7d7", "#87d7ff", "#87ff00", "#87ff5f",
    "#87ff87", "#87ffaf", "#87ffd7", "#87ffff", "#af0000", "#af005f", "#af0087", "#af00af",
    "#af00d7", "#af00ff", "#af5f00", "#af5f5f", "#af5f87", "#af5faf", "#af5fd7", "#af5fff",
    "#af8700", "#af875f", "#af8787", "#af87af", "#af87d7", "#af87ff", "#afaf00", "#afaf5f",
    "#afaf87", "#afafaf", "#afafd7", "#afafff", "#afd700", "#afd75f", "#afd787", "#afd7af",
    "#afd7d7", "#afd7ff", "#afff00", "#afff5f", "#afff87", "#afffaf", "#afffd7", "#afffff",
    "#d70000", "#d7005f", "#d70087", "#d700af", "#d700d7", "#d700ff", "#d75f00", "#d75f5f",
    "#d75f87", "#d75faf", "#d75fd7", "#d75fff", "#d78700", "#d7875f", "#d78787", "#d787af",
    "#d787d7", "#d787ff", "#d7af00", "#d7af5f", "#d7af87", "#d7afaf", "#d7afd7", "#d7afff",
    "#d7d700", "#d7d75f", "#d7d787", "#d7d7af", "#d7d7d7", "#d7d7ff", "#d7ff00", "#d7ff5f",
    "#d7ff87", "#d7ffaf", "#d7ffd7", "#d7ffff", "#ff0000", "#ff005f", "#ff0087", "#ff00af",
    "#ff00d7", "#ff00ff", "#ff5f00", "#ff5f5f", "#ff5f87", "#ff5faf", "#ff5fd7", "#ff5fff",
    "#ff8700", "#ff875f", "#ff8787", "#ff87af", "#ff87d7", "#ff87ff", "#ffaf00", "#ffaf5f",
    "#ffaf87", "#ffafaf", "#ffafd7", "#ffafff", "#ffd700", "#ffd75f", "#ffd787", "#ffd7af",
    "#ffd7d7", "#ffd7ff", "#ffff00", "#ffff5f", "#ffff87", "#ffffaf", "#ffffd7", "#ffffff",
    "#080808", "#121212", "#1c1c1c", "#262626", "#303030", "#3a3a3a", "#444444", "#4e4e4e",
    "#585858", "#626262", "#6c6c6c", "#767676", "#808080", "#8a8a8a", "#949494", "#9e9e9e",
    "#a8a8a8", "#b2b2b2", "#bcbcbc", "#c6c6c6", "#d0d0d0", "#dadada", "#e4e4e4", "#eeeeee",
];

/// Parse one `#rrggbb` channel pair at compile time.
///
/// All inputs are the known-good literals in [`VIEW_PALETTE_HEX`]; the
/// `view_palette_digest` test pins the parsed palette against the canonical
/// digest from the migration API record.
const fn hex_digit(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => 0,
    }
}

/// Parse one `#rrggbb` literal at compile time.
const fn parse_hex_color(hex: &str) -> Rgb {
    let bytes = hex.as_bytes();
    Rgb::new(
        hex_digit(bytes[1]) * 16 + hex_digit(bytes[2]),
        hex_digit(bytes[3]) * 16 + hex_digit(bytes[4]),
        hex_digit(bytes[5]) * 16 + hex_digit(bytes[6]),
    )
}

/// Build the fixed view palette from its recorded literals.
const fn build_view_palette() -> [Rgb; 256] {
    let mut palette = [Rgb::new(0, 0, 0); 256];
    let mut index = 0;
    while index < 256 {
        palette[index] = parse_hex_color(VIEW_PALETTE_HEX[index]);
        index += 1;
    }
    palette
}

/// Translate a Ratatui color without information loss.
///
/// `Reset` stays the terminal default, distinct from an explicit `Indexed` or
/// `Rgb` reference even when both resolve to the same painted color. Named
/// ANSI colors map to their fixed palette slots.
fn convert_color(color: ratatui::style::Color) -> SnapshotColor {
    use ratatui::style::Color as RatatuiColor;
    match color {
        RatatuiColor::Reset => SnapshotColor::Default,
        RatatuiColor::Black => SnapshotColor::Indexed(0),
        RatatuiColor::Red => SnapshotColor::Indexed(1),
        RatatuiColor::Green => SnapshotColor::Indexed(2),
        RatatuiColor::Yellow => SnapshotColor::Indexed(3),
        RatatuiColor::Blue => SnapshotColor::Indexed(4),
        RatatuiColor::Magenta => SnapshotColor::Indexed(5),
        RatatuiColor::Cyan => SnapshotColor::Indexed(6),
        RatatuiColor::Gray => SnapshotColor::Indexed(7),
        RatatuiColor::DarkGray => SnapshotColor::Indexed(8),
        RatatuiColor::LightRed => SnapshotColor::Indexed(9),
        RatatuiColor::LightGreen => SnapshotColor::Indexed(10),
        RatatuiColor::LightYellow => SnapshotColor::Indexed(11),
        RatatuiColor::LightBlue => SnapshotColor::Indexed(12),
        RatatuiColor::LightMagenta => SnapshotColor::Indexed(13),
        RatatuiColor::LightCyan => SnapshotColor::Indexed(14),
        RatatuiColor::White => SnapshotColor::Indexed(15),
        RatatuiColor::Indexed(index) => SnapshotColor::Indexed(index),
        RatatuiColor::Rgb(red, green, blue) => SnapshotColor::Rgb(red, green, blue),
    }
}

/// Translate Ratatui modifiers into upstream attributes and an underline shape.
///
/// Ratatui carries no per-cell underline shape, so an underlined cell maps to
/// `Single` and a plain cell maps to `None`. Blink speeds collapse into one
/// blink attribute, matching the single upstream blink bit.
fn convert_modifier(modifier: ratatui::style::Modifier) -> (Attrs, SnapshotUnderline) {
    use ratatui::style::Modifier as RatatuiModifier;
    let mut attrs = Attrs::empty();
    if modifier.contains(RatatuiModifier::BOLD) {
        attrs |= Attrs::BOLD;
    }
    if modifier.contains(RatatuiModifier::DIM) {
        attrs |= Attrs::DIM;
    }
    if modifier.contains(RatatuiModifier::ITALIC) {
        attrs |= Attrs::ITALIC;
    }
    if modifier.contains(RatatuiModifier::REVERSED) {
        attrs |= Attrs::INVERSE;
    }
    if modifier.contains(RatatuiModifier::HIDDEN) {
        attrs |= Attrs::INVISIBLE;
    }
    if modifier.contains(RatatuiModifier::CROSSED_OUT) {
        attrs |= Attrs::STRIKE;
    }
    if modifier.contains(RatatuiModifier::SLOW_BLINK)
        || modifier.contains(RatatuiModifier::RAPID_BLINK)
    {
        attrs |= Attrs::BLINK;
    }
    let underline = if modifier.contains(RatatuiModifier::UNDERLINED) {
        SnapshotUnderline::Single
    } else {
        SnapshotUnderline::None
    };
    (attrs, underline)
}

/// Convert one Ratatui cell that starts a grapheme.
fn leading_cell(
    cell: &ratatui::buffer::Cell,
    symbol: &str,
    width: u16,
) -> ViewResult<SnapshotCell> {
    let (attrs, underline) = convert_modifier(cell.modifier);
    SnapshotCell::new(
        symbol.to_string(),
        width,
        convert_color(cell.fg),
        convert_color(cell.bg),
        attrs,
        underline,
        convert_color(cell.underline_color),
        None,
        None,
    )
    .map_err(ViewError::Snapshot)
}

/// Convert one Ratatui cell covered by a wide leading cell.
///
/// Ratatui has no hyperlink storage, so every cell records unavailable link
/// data. Covered cells keep their own stored colors and attributes.
fn continuation_cell(cell: &ratatui::buffer::Cell) -> ViewResult<SnapshotCell> {
    let (attrs, underline) = convert_modifier(cell.modifier);
    SnapshotCell::new(
        String::new(),
        0,
        convert_color(cell.fg),
        convert_color(cell.bg),
        attrs,
        underline,
        convert_color(cell.underline_color),
        None,
        None,
    )
    .map_err(ViewError::Snapshot)
}

/// Resolve the snapshot cursor from buffer-local coordinates.
///
/// A cursor outside the grid is not clamped: it keeps its position and reports
/// `in_capture` as `false`. Terminal cursor visibility stays separate from
/// capture clipping.
fn resolve_cursor(
    col: u32,
    row: u32,
    visible: bool,
    settings: &ViewSettings,
    cols: u32,
    rows: u32,
) -> SnapshotCursor {
    let cursor = settings.cursor();
    SnapshotCursor::new(
        col,
        row,
        col < cols && row < rows,
        visible,
        cursor.shape(),
        cursor.color(),
        None,
    )
}

/// Convert a Ratatui buffer area into row-major snapshot cells.
///
/// Buffer coordinates normalize to the snapshot origin: the snapshot holds
/// `area.width` by `area.height` cells starting at the buffer area origin.
/// Continuation cells resolve from Ratatui layout rules: a wide leading cell
/// covers its following columns, and covered placeholder cells convert as
/// continuations. A covered column that carries its own grapheme, a wide
/// leading cell that overhangs the row end, or a zero-width cell outside
/// any coverage is an invalid leading and continuation combination:
/// conversion rejects it here with coordinates, because the upstream
/// validated constructor accepts individually valid cells and cannot see
/// the coverage mismatch. It stays as the backstop for every other
/// malformed combination.
///
/// Clipped edges keep the representation Ratatui itself produces: a wide
/// grapheme that does not fit the remaining row width renders as a blank
/// narrow leading cell (width one, not a continuation), and only columns a
/// leading cell actually covers convert as continuations. Coverage never
/// crosses a row boundary.
fn convert_cells(buffer: &Buffer) -> ViewResult<Vec<SnapshotCell>> {
    let area = buffer.area;
    let mut cells = Vec::new();
    for row in 0..area.height {
        let mut covered: u16 = 0;
        for col in 0..area.width {
            let x = area.x.checked_add(col).ok_or(ViewError::AreaOverflow)?;
            let y = area.y.checked_add(row).ok_or(ViewError::AreaOverflow)?;
            let cell = buffer
                .cell(Position::new(x, y))
                .ok_or(ViewError::CellOutOfBounds {
                    x: u32::from(x),
                    y: u32::from(y),
                })?;
            let symbol = cell.symbol();
            if covered > 0 {
                if symbol == " " || symbol.is_empty() {
                    covered -= 1;
                    cells.push(continuation_cell(cell)?);
                } else {
                    return Err(ViewError::CoveredCellOccupied {
                        x: u32::from(x),
                        y: u32::from(y),
                    });
                }
            } else {
                let width = cell.cell_width();
                if width == 0 {
                    return Err(ViewError::UncoveredZeroWidthCell {
                        x: u32::from(x),
                        y: u32::from(y),
                    });
                }
                if width > area.width - col {
                    return Err(ViewError::LeadingCellOverhang {
                        x: u32::from(x),
                        y: u32::from(y),
                        width,
                    });
                }
                cells.push(leading_cell(cell, symbol, width)?);
                covered = width.saturating_sub(1);
            }
        }
    }
    Ok(cells)
}

/// Reject grid dimensions before any allocation or conversion, mirroring
/// the upstream validated constructor's limits field for field: zero stays
/// [`ViewError::EmptyDimensions`], a dimension above [`MAX_SNAPSHOT_DIM`]
/// reports `DimensionTooLarge`, and a `cols * rows` product above
/// [`MAX_SNAPSHOT_CELLS`] reports `TooManyCells`. Both entry points call
/// this first so an oversize grid never reaches `TestBackend` or the cell
/// converter: the snapshot validator would reject the same grid only after
/// those allocations ran.
fn check_dimensions(cols: u32, rows: u32) -> ViewResult<()> {
    if cols == 0 || rows == 0 {
        return Err(ViewError::EmptyDimensions);
    }
    if cols > MAX_SNAPSHOT_DIM || rows > MAX_SNAPSHOT_DIM {
        return Err(ViewError::Snapshot(SnapshotError::DimensionTooLarge {
            cols: u64::from(cols),
            rows: u64::from(rows),
            limit: MAX_SNAPSHOT_DIM,
        }));
    }
    let cells =
        u64::from(cols)
            .checked_mul(u64::from(rows))
            .ok_or(SnapshotError::CountOverflow {
                field: "cols * rows",
            })?;
    if cells > MAX_SNAPSHOT_CELLS {
        return Err(ViewError::Snapshot(SnapshotError::TooManyCells {
            cells,
            limit: MAX_SNAPSHOT_CELLS,
        }));
    }
    Ok(())
}

/// Assemble a validated snapshot from converted cells and cursor state.
fn assemble_snapshot(
    cols: u32,
    rows: u32,
    cells: Vec<SnapshotCell>,
    cursor: SnapshotCursor,
    settings: &ViewSettings,
) -> ViewResult<TerminalSnapshot> {
    TerminalSnapshot::try_new(SnapshotParts {
        cols,
        rows,
        scope: SnapshotScope::Viewport,
        viewport_origin: 0,
        cells,
        palette: settings.palette,
        default_foreground: settings.default_foreground,
        default_background: settings.default_background,
        default_cursor: settings.cursor().color(),
        cursor,
        title: None,
        metadata: SnapshotMetadata::default(),
    })
    .map_err(ViewError::Snapshot)
}

/// Render an application production view through a Ratatui `TestBackend`.
///
/// The draw closure calls the application production view function; it must
/// not be a test-only replacement view. When the draw result shows a cursor,
/// conversion keeps its position and visibility; otherwise the cursor stays
/// hidden at the explicit position from `settings`. The cursor shape and
/// color always come from `settings`, with a block shape in the built-in
/// profile. No child process, PTY, or session starts on this path: the only
/// inputs are the closure, the settings, and the in-memory backend.
///
/// # Errors
///
/// Returns [`ViewError`] when the grid is empty or exceeds the snapshot
/// dimension or cell-count limits (rejected before any allocation), a
/// wide-cell layout is invalid, or the converted snapshot fails upstream
/// validation.
///
/// # Examples
///
/// ```rust
/// use ratatui::Frame;
/// use ratatui::widgets::Paragraph;
/// use tuiscotti::view::{ViewSettings, capture_view};
///
/// fn login_view(frame: &mut Frame) {
///     frame.render_widget(Paragraph::new("hello"), frame.area());
/// }
///
/// let snapshot = capture_view(20, 5, &ViewSettings::builtin(), |frame| {
///     login_view(frame);
/// });
/// assert!(snapshot.is_ok());
/// ```
#[must_use = "a captured snapshot has no effect unless inspected"]
pub fn capture_view<F>(
    width: u16,
    height: u16,
    settings: &ViewSettings,
    draw: F,
) -> ViewResult<TerminalSnapshot>
where
    F: FnOnce(&mut Frame<'_>),
{
    check_dimensions(u32::from(width), u32::from(height))?;
    let backend = TestBackend::new(width, height);
    let mut terminal = match Terminal::new(backend) {
        Ok(terminal) => terminal,
        Err(error) => match error {},
    };
    let frame = match terminal.draw(draw) {
        Ok(frame) => frame,
        Err(error) => match error {},
    };
    let area = frame.buffer.area;
    let cols = u32::from(area.width);
    let rows = u32::from(area.height);
    let cells = convert_cells(frame.buffer)?;
    let backend = terminal.backend();
    let (col, row, visible) = if backend.cursor_visible() {
        let position = backend.cursor_position();
        (u32::from(position.x), u32::from(position.y), true)
    } else {
        let cursor = settings.cursor();
        (cursor.col(), cursor.row(), false)
    };
    let cursor = resolve_cursor(col, row, visible, settings, cols, rows);
    assemble_snapshot(cols, rows, cells, cursor, settings)
}

/// Convert a Ratatui buffer into the shared snapshot model.
///
/// The explicit cursor state in `settings` is mandatory: Ratatui buffers
/// carry no cursor, so conversion cannot infer one. Buffer coordinates
/// normalize to the snapshot origin.
///
/// # Errors
///
/// Returns [`ViewError`] when the buffer area is empty or exceeds the
/// snapshot dimension or cell-count limits (rejected before conversion), an
/// area coordinate overflows, a wide-cell layout is invalid, or the
/// converted snapshot fails upstream validation.
///
/// # Examples
///
/// ```rust
/// use ratatui::buffer::Buffer;
/// use ratatui::layout::Rect;
/// use tuiscotti::view::{ViewCursor, ViewSettings, snapshot_from_buffer};
/// use tui_test::{SnapshotCursorShape, profile::Rgb};
///
/// let buffer = Buffer::empty(Rect::new(0, 0, 5, 1));
/// let cursor = ViewCursor::new(0, 0, false, SnapshotCursorShape::Block, Rgb::new(192, 192, 192));
/// let settings = ViewSettings::builtin().with_cursor(cursor);
/// let snapshot = snapshot_from_buffer(&buffer, &settings);
/// assert!(snapshot.is_ok());
/// ```
#[must_use = "a converted snapshot has no effect unless inspected"]
pub fn snapshot_from_buffer(
    buffer: &Buffer,
    settings: &ViewSettings,
) -> ViewResult<TerminalSnapshot> {
    let area = buffer.area;
    let cols = u32::from(area.width);
    let rows = u32::from(area.height);
    check_dimensions(cols, rows)?;
    let cursor_state = settings.cursor();
    let cursor = resolve_cursor(
        cursor_state.col(),
        cursor_state.row(),
        cursor_state.visible(),
        settings,
        cols,
        rows,
    );
    let cells = convert_cells(buffer)?;
    assemble_snapshot(cols, rows, cells, cursor, settings)
}
