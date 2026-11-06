use ratatui::buffer::{Buffer, CellWidth};
use ratatui::layout::Position;
use tui_test::terminal::cell::Attrs;
use tui_test::{MAX_SNAPSHOT_CELLS, MAX_SNAPSHOT_DIM, SnapshotError};
use tui_test::{
    SnapshotCell, SnapshotColor, SnapshotCursor, SnapshotMetadata, SnapshotParts, SnapshotScope,
    SnapshotUnderline, TerminalSnapshot,
};

use super::types::{ViewError, ViewResult, ViewSettings};

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

pub(super) fn resolve_cursor(
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

pub(super) fn convert_cells(buffer: &Buffer) -> ViewResult<Vec<SnapshotCell>> {
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
                    x: u32::from(col),
                    y: u32::from(row),
                })?;
            let symbol = cell.symbol();
            if covered > 0 {
                if symbol == " " || symbol.is_empty() {
                    covered -= 1;
                    cells.push(continuation_cell(cell)?);
                } else {
                    return Err(ViewError::CoveredCellOccupied {
                        x: u32::from(col),
                        y: u32::from(row),
                    });
                }
            } else {
                let width = cell.cell_width();
                if width == 0 {
                    return Err(ViewError::UncoveredZeroWidthCell {
                        x: u32::from(col),
                        y: u32::from(row),
                    });
                }
                if width > area.width - col {
                    return Err(ViewError::LeadingCellOverhang {
                        x: u32::from(col),
                        y: u32::from(row),
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

pub(super) fn check_dimensions(cols: u32, rows: u32) -> ViewResult<()> {
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

pub(super) fn assemble_snapshot(
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
