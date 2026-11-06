use ratatui::buffer::Buffer;
use ratatui::{Frame, Terminal, backend::TestBackend};
use tui_test::TerminalSnapshot;

use super::convert::{assemble_snapshot, check_dimensions, convert_cells, resolve_cursor};
use super::types::{ViewResult, ViewSettings};

#[doc = "Render an application production view through a `Ratatui` `TestBackend`.\n\n# Errors\n\nReturns [`crate::view::ViewError`] when the grid is empty or exceeds the snapshot dimension or cell-count limits (rejected before any allocation), a wide-cell layout is invalid, or the converted snapshot fails upstream validation."]
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

#[doc = "Convert a `Ratatui` buffer into the shared snapshot model.\n\n# Errors\n\nReturns [`crate::view::ViewError`] when the buffer area is empty or exceeds the snapshot dimension or cell-count limits (rejected before conversion), an area coordinate overflows, a wide-cell layout is invalid, or the converted snapshot fails upstream validation."]
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
