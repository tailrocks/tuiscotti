# `view` documentation

# Module

Pure-view capture: Ratatui application views into the upstream snapshot model.

A pure-view test renders an application view in the test process. It starts
no shell, child process, PTY, daemon, or terminal session. The only inputs
are the caller-supplied draw closure, the caller-supplied view settings,
and the Ratatui buffer they produce.

Two entry points share one conversion:

- [`capture_view`] renders a production draw closure through a Ratatui
  `TestBackend` and converts the result.
- [`snapshot_from_buffer`] converts a caller-supplied Ratatui buffer with
  explicit cursor state.

Both pass the converted cells through the upstream validated constructor
[`TerminalSnapshot::try_new`], so one snapshot model serves live tests and
production-view tests.

# `pub type ViewResult<T> = Result<T, ViewError>;`

Result of a pure-view capture or conversion.

# `pub enum ViewError {`

Every failure a pure-view capture or conversion can report, with sources retained.

# `EmptyDimensions,`

The requested grid has a zero column or row count.

# `AreaOverflow,`

A buffer area coordinate plus its offset does not fit a 16-bit position.

# `CellOutOfBounds {`

A grid position inside the buffer area has no cell.

# `x: u32,`

Buffer-local column of the missing cell.

# `y: u32,`

Buffer-local row of the missing cell.

# `CoveredCellOccupied {`

A column covered by a wide leading cell carries its own grapheme
instead of continuing that cell.

# `x: u32,`

Buffer-local column of the occupying cell.

# `y: u32,`

Buffer-local row of the occupying cell.

# `UncoveredZeroWidthCell {`

A cell outside any wide-cell coverage has zero width, so it cannot
be a continuation: only a covering leading cell makes a
zero-width cell valid.

# `x: u32,`

Buffer-local column of the zero-width cell.

# `y: u32,`

Buffer-local row of the zero-width cell.

# `LeadingCellOverhang {`

A wide leading cell is wider than the columns left in its row, so its
continuation would fall outside the grid.

# `x: u32,`

Buffer-local column of the overhanging leading cell.

# `y: u32,`

Buffer-local row of the overhanging leading cell.

# `width: u16,`

Terminal columns the leading cell claims.

# `Snapshot(SnapshotError),`

The upstream validated constructor rejected the converted snapshot.

# `pub struct ViewCursor {`

Explicit cursor state for a pure-view capture.

Ratatui buffers carry no cursor, so buffer-only conversion takes the cursor
from these caller-supplied settings. The color doubles as the resolved
default cursor color in the snapshot. Blink intent stays unavailable:
Ratatui reports no blink state, so conversion records `None` rather than a
guessed value.

# `pub const fn new(`

Capture explicit cursor state.

The position uses buffer-local coordinates: column and row relative to
the buffer area origin, matching [`snapshot_from_buffer`].

# `pub const fn col(self) -> u32 {`

Buffer-local cursor column.

# `pub const fn row(self) -> u32 {`

Buffer-local cursor row.

# `pub const fn visible(self) -> bool {`

Whether the terminal draws the cursor.

# `pub const fn shape(self) -> SnapshotCursorShape {`

The shape the terminal draws the cursor as.

# `pub const fn color(self) -> Rgb {`

The resolved cursor color.

# `pub struct ViewSettings {`

Caller-supplied view settings for a pure-view capture.

The palette and defaults are copied into the snapshot as caller-supplied
view settings. They are never observed terminal colors: no child, PTY, or
session runs during conversion, so nothing exists to observe.

# `pub const fn new(`

Capture caller-supplied view settings with explicit cursor state.

# `pub const fn builtin() -> Self {`

The built-in view profile: the fixed view palette from the migration
API record with a hidden block cursor at the origin.

The palette copies the recorded values byte for byte. Do not
recalculate it from a later dependency version.

# `pub const fn with_palette(mut self, palette: [Rgb; 256]) -> Self {`

Replace the resolved palette.

# `pub const fn with_defaults(mut self, default_foreground: Rgb, default_background: Rgb) -> Self {`

Replace the resolved default foreground and background.

# `pub const fn with_cursor(mut self, cursor: ViewCursor) -> Self {`

Replace the explicit cursor state.

# `pub const fn palette(&self) -> &[Rgb; 256] {`

Every resolved palette entry, indices 0 through 255.

# `pub const fn default_foreground(&self) -> Rgb {`

The resolved default foreground.

# `pub const fn default_background(&self) -> Rgb {`

The resolved default background.

# `pub const fn cursor(&self) -> ViewCursor {`

The explicit cursor state.

# `const VIEW_PALETTE_HEX: [&str; 256] = [`

The fixed view palette as recorded `#rrggbb` strings.

Source: the migration API record, copied from an in-memory upstream
emulator read through `Emulator::colors()` with no child, PTY, or session.
The canonical digest over these values is
`92757b147e00dec394b10b7be2fa42f782f0d30325a5922570080f91d60f641e`;
The digest above records the palette; no test pins it.

# `const fn hex_digit(byte: u8) -> u8 {`

Parse one `#rrggbb` channel pair at compile time.

All inputs are the fixed literals in `VIEW_PALETTE_HEX`.

# `const fn parse_hex_color(hex: &str) -> Rgb {`

Parse one `#rrggbb` literal at compile time.

# `const fn build_view_palette() -> [Rgb; 256] {`

Build the fixed view palette from its recorded literals.

# `fn convert_color(color: ratatui::style::Color) -> SnapshotColor {`

Translate a Ratatui color without information loss.

`Reset` stays the terminal default, distinct from an explicit `Indexed` or
`Rgb` reference even when both resolve to the same painted color. Named
ANSI colors map to their fixed palette slots.

# `fn convert_modifier(modifier: ratatui::style::Modifier) -> (Attrs, SnapshotUnderline) {`

Translate Ratatui modifiers into upstream attributes and an underline shape.

Ratatui carries no per-cell underline shape, so an underlined cell maps to
`Single` and a plain cell maps to `None`. Blink speeds collapse into one
blink attribute, matching the single upstream blink bit.

# `fn leading_cell(`

Convert one Ratatui cell that starts a grapheme.

# `fn continuation_cell(cell: &ratatui::buffer::Cell) -> ViewResult<SnapshotCell> {`

Convert one Ratatui cell covered by a wide leading cell.

Ratatui has no hyperlink storage, so every cell records unavailable link
data. Covered cells keep their own stored colors and attributes.

# `fn resolve_cursor(`

Resolve the snapshot cursor from buffer-local coordinates.

A cursor outside the grid is not clamped: it keeps its position and reports
`in_capture` as `false`. Terminal cursor visibility stays separate from
capture clipping.

# `fn convert_cells(buffer: &Buffer) -> ViewResult<Vec<SnapshotCell>> {`

Convert a Ratatui buffer area into row-major snapshot cells.

Buffer coordinates normalize to the snapshot origin: the snapshot holds
`area.width` by `area.height` cells starting at the buffer area origin.
Continuation cells resolve from Ratatui layout rules: a wide leading cell
covers its following columns, and covered placeholder cells convert as
continuations. A covered column that carries its own grapheme, a wide
leading cell that overhangs the row end, or a zero-width cell outside
any coverage is an invalid leading and continuation combination:
conversion rejects it here with coordinates, because the upstream
validated constructor accepts individually valid cells and cannot see
the coverage mismatch. It stays as the backstop for every other
malformed combination.

Clipped edges keep the representation Ratatui itself produces: a wide
grapheme that does not fit the remaining row width renders as a blank
narrow leading cell (width one, not a continuation), and only columns a
leading cell actually covers convert as continuations. Coverage never
crosses a row boundary.

# `fn check_dimensions(cols: u32, rows: u32) -> ViewResult<()> {`

Reject grid dimensions before any allocation or conversion, mirroring
the upstream validated constructor's limits field for field: zero stays
[`ViewError::EmptyDimensions`], a dimension above [`MAX_SNAPSHOT_DIM`]
reports `DimensionTooLarge`, and a `cols * rows` product above
[`MAX_SNAPSHOT_CELLS`] reports `TooManyCells`. Both entry points call
this first so an oversize grid never reaches `TestBackend` or the cell
converter: the snapshot validator would reject the same grid only after
those allocations ran.

# `fn assemble_snapshot(`

Assemble a validated snapshot from converted cells and cursor state.

# `pub fn capture_view<F>(`

Render an application production view through a Ratatui `TestBackend`.

The draw closure calls the application production view function; it must
not be a test-only replacement view. When the draw result shows a cursor,
conversion keeps its position and visibility; otherwise the cursor stays
hidden at the explicit position from `settings`. The cursor shape and
color always come from `settings`, with a block shape in the built-in
profile. No child process, PTY, or session starts on this path: the only
inputs are the closure, the settings, and the in-memory backend.

# Errors

Returns [`ViewError`] when the grid is empty or exceeds the snapshot
dimension or cell-count limits (rejected before any allocation), a
wide-cell layout is invalid, or the converted snapshot fails upstream
validation.

# Examples

```rust
use ratatui::Frame;
use ratatui::widgets::Paragraph;
use tuiscotti::view::{ViewSettings, capture_view};

fn login_view(frame: &mut Frame) {
    frame.render_widget(Paragraph::new("hello"), frame.area());
}

let snapshot = capture_view(20, 5, &ViewSettings::builtin(), |frame| {
    login_view(frame);
});
assert!(snapshot.is_ok());
```

# `pub fn snapshot_from_buffer(`

Convert a Ratatui buffer into the shared snapshot model.

The explicit cursor state in `settings` is mandatory: Ratatui buffers
carry no cursor, so conversion cannot infer one. Buffer coordinates
normalize to the snapshot origin.

# Errors

Returns [`ViewError`] when the buffer area is empty or exceeds the
snapshot dimension or cell-count limits (rejected before conversion), an
area coordinate overflows, a wide-cell layout is invalid, or the
converted snapshot fails upstream validation.

# Examples

```rust
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tuiscotti::view::{ViewCursor, ViewSettings, snapshot_from_buffer};
use tui_test::{SnapshotCursorShape, profile::Rgb};

let buffer = Buffer::empty(Rect::new(0, 0, 5, 1));
let cursor = ViewCursor::new(0, 0, false, SnapshotCursorShape::Block, Rgb::new(192, 192, 192));
let settings = ViewSettings::builtin().with_cursor(cursor);
let snapshot = snapshot_from_buffer(&buffer, &settings);
assert!(snapshot.is_ok());
```
