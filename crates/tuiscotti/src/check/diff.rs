use tui_test::terminal::cell::Attrs;
use tui_test::{SnapshotCell, TerminalSnapshot};

use crate::report::ReportedCell;

use super::MAX_REPORTED_CELLS;

const MAX_DISPLAY_CHARS: usize = 200;

const CELL_FLAGS: [(&str, Attrs); 7] = [
    ("bold", Attrs::BOLD),
    ("dim", Attrs::DIM),
    ("italic", Attrs::ITALIC),
    ("inverse", Attrs::INVERSE),
    ("invisible", Attrs::INVISIBLE),
    ("strike", Attrs::STRIKE),
    ("blink", Attrs::BLINK),
];

pub(super) struct StateDiff {
    pub(super) changed_fields: Vec<String>,
    pub(super) changed_cells: Vec<ReportedCell>,
    pub(super) changed_cell_total: usize,
    pub(super) title_values: Option<(String, String)>,
    pub(super) pixel_changed: u64,
    pub(super) pixel_compared: u64,
    pub(super) pixel_bounds: Option<[u32; 4]>,
}

impl StateDiff {
    pub(super) fn named(field: &str) -> Self {
        Self {
            changed_fields: vec![field.to_string()],
            changed_cells: Vec::new(),
            changed_cell_total: 0,
            title_values: None,
            pixel_changed: 0,
            pixel_compared: 0,
            pixel_bounds: None,
        }
    }

    pub(super) fn pixels(changed: u64, compared: u64, bounds: Option<[u32; 4]>) -> Self {
        Self {
            changed_fields: Vec::new(),
            changed_cells: Vec::new(),
            changed_cell_total: 0,
            title_values: None,
            pixel_changed: changed,
            pixel_compared: compared,
            pixel_bounds: bounds,
        }
    }

    pub(super) fn is_equal(&self) -> bool {
        self.changed_fields.is_empty() && self.changed_cell_total == 0
    }
}

pub(super) fn diff_snapshots(actual: &TerminalSnapshot, baseline: &TerminalSnapshot) -> StateDiff {
    let mut changed_fields = Vec::new();
    if actual.cols() != baseline.cols() {
        changed_fields.push("cols".to_string());
    }
    if actual.rows() != baseline.rows() {
        changed_fields.push("rows".to_string());
    }
    if actual.scope() != baseline.scope() {
        changed_fields.push("scope".to_string());
    }
    if actual.viewport_origin() != baseline.viewport_origin() {
        changed_fields.push("viewport_origin".to_string());
    }
    if actual.palette() != baseline.palette() {
        changed_fields.push("palette".to_string());
    }
    if actual.default_foreground() != baseline.default_foreground() {
        changed_fields.push("default_foreground".to_string());
    }
    if actual.default_background() != baseline.default_background() {
        changed_fields.push("default_background".to_string());
    }
    if actual.default_cursor() != baseline.default_cursor() {
        changed_fields.push("default_cursor".to_string());
    }
    changed_fields.extend(diff_cursors(actual, baseline));
    let title_values = if actual.title() == baseline.title() {
        None
    } else {
        changed_fields.push("title".to_string());
        Some((
            display_value(actual.title().unwrap_or("")),
            display_value(baseline.title().unwrap_or("")),
        ))
    };
    let (changed_cells, changed_cell_total) = diff_cells(actual, baseline);
    StateDiff {
        changed_fields,
        changed_cells,
        changed_cell_total,
        title_values,
        pixel_changed: 0,
        pixel_compared: 0,
        pixel_bounds: None,
    }
}

fn display_value(value: &str) -> String {
    if value.chars().count() <= MAX_DISPLAY_CHARS {
        return value.to_string();
    }
    let kept: String = value.chars().take(MAX_DISPLAY_CHARS).collect();
    format!("{kept}…")
}

fn diff_cursors(actual: &TerminalSnapshot, baseline: &TerminalSnapshot) -> Vec<String> {
    let mut fields = Vec::new();
    let (left, right) = (actual.cursor(), baseline.cursor());
    if left.col() != right.col() {
        fields.push("cursor.col".to_string());
    }
    if left.row() != right.row() {
        fields.push("cursor.row".to_string());
    }
    if left.in_capture() != right.in_capture() {
        fields.push("cursor.in_capture".to_string());
    }
    if left.visible() != right.visible() {
        fields.push("cursor.visible".to_string());
    }
    if left.shape() != right.shape() {
        fields.push("cursor.shape".to_string());
    }
    if left.color() != right.color() {
        fields.push("cursor.color".to_string());
    }
    if left.blink() != right.blink() {
        fields.push("cursor.blink".to_string());
    }
    fields
}

fn diff_cells(
    actual: &TerminalSnapshot,
    baseline: &TerminalSnapshot,
) -> (Vec<ReportedCell>, usize) {
    let rows = actual.rows().max(baseline.rows());
    let mut changed = Vec::new();
    let mut total = 0usize;
    for row in 0..rows {
        let row_cols = if row < actual.rows() && row < baseline.rows() {
            actual.cols().max(baseline.cols())
        } else if row < actual.rows() {
            actual.cols()
        } else {
            baseline.cols()
        };
        for col in 0..row_cols {
            let pair = (actual.cell_at(col, row), baseline.cell_at(col, row));
            let Some((fields, actual_text, baseline_text)) = changed_pair(pair) else {
                continue;
            };
            total += 1;
            if changed.len() < MAX_REPORTED_CELLS {
                changed.push(ReportedCell::new(
                    col,
                    row,
                    fields,
                    actual_text,
                    baseline_text,
                ));
            }
        }
    }
    (changed, total)
}

fn changed_pair(
    pair: (Option<&SnapshotCell>, Option<&SnapshotCell>),
) -> Option<(Vec<String>, String, String)> {
    let (fields, actual_text, baseline_text) = match pair {
        (Some(left), Some(right)) => {
            let fields = diff_cell(left, right);
            if fields.is_empty() {
                return None;
            }
            (
                fields,
                display_value(left.text()),
                display_value(right.text()),
            )
        }
        (Some(left), None) => (
            vec!["cell".to_string()],
            display_value(left.text()),
            String::new(),
        ),
        (None, Some(right)) => (
            vec!["cell".to_string()],
            String::new(),
            display_value(right.text()),
        ),
        (None, None) => return None,
    };
    Some((fields, actual_text, baseline_text))
}

fn diff_cell(actual: &SnapshotCell, baseline: &SnapshotCell) -> Vec<String> {
    let mut fields = Vec::new();
    if actual.text() != baseline.text() {
        fields.push("text".to_string());
    }
    if actual.width() != baseline.width() {
        fields.push("width".to_string());
    }
    if actual.fg() != baseline.fg() {
        fields.push("fg".to_string());
    }
    if actual.bg() != baseline.bg() {
        fields.push("bg".to_string());
    }
    for (name, flag) in CELL_FLAGS {
        if actual.has(flag) != baseline.has(flag) {
            fields.push(name.to_string());
        }
    }
    if actual.underline() != baseline.underline() {
        fields.push("underline".to_string());
    }
    if actual.underline_color() != baseline.underline_color() {
        fields.push("underline_color".to_string());
    }
    if actual.link_uri() != baseline.link_uri() {
        fields.push("link_uri".to_string());
    }
    if actual.link_id() != baseline.link_id() {
        fields.push("link_id".to_string());
    }
    fields
}
