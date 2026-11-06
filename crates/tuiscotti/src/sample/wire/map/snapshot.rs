use tui_test::profile::Rgb;
use tui_test::{SnapshotCursor, SnapshotMetadata, SnapshotParts, SnapshotScope, TerminalSnapshot};

use super::super::super::decode_error::DecodeError;
use super::super::generated::{
    DecodeSnapshot, Scope, ValidatedCursor, ValidatedMetadata, ValidatedRgb, ValidatedSnapshot,
};
use super::allocation::{copied_string, copied_vec, domain_error};
use super::cell::map_cell;
use super::snapshot_components::{map_cursor, map_metadata, map_rgb, scope};

struct SnapshotFields {
    schema: Option<u32>,
    cols: Option<u32>,
    rows: Option<u32>,
    scope: Option<SnapshotScope>,
    viewport_origin: Option<u64>,
    cells: Vec<tui_test::SnapshotCell>,
    palette: [Rgb; 256],
    palette_len: usize,
    default_foreground: Option<Rgb>,
    default_background: Option<Rgb>,
    default_cursor: Option<Rgb>,
    cursor: Option<SnapshotCursor>,
    title: super::FieldPresence<Option<String>>,
    metadata: Option<SnapshotMetadata>,
}

impl Default for SnapshotFields {
    fn default() -> Self {
        Self {
            schema: None,
            cols: None,
            rows: None,
            scope: None,
            viewport_origin: None,
            cells: Vec::new(),
            palette: [Rgb::new(0, 0, 0); 256],
            palette_len: 0,
            default_foreground: None,
            default_background: None,
            default_cursor: None,
            cursor: None,
            title: super::FieldPresence::Unset,
            metadata: None,
        }
    }
}

impl SnapshotFields {
    fn new(cell_capacity: usize) -> Result<Self, DecodeError> {
        Ok(Self {
            cells: copied_vec(cell_capacity, "snapshot cells")?,
            ..Self::default()
        })
    }

    fn finish(self) -> Result<TerminalSnapshot, DecodeError> {
        if self.schema != Some(1) {
            return Err(DecodeError::Invariant {
                detail: "validated snapshot schema changed",
            });
        }
        if self.palette_len != self.palette.len() {
            return Err(DecodeError::Invariant {
                detail: "validated palette count changed",
            });
        }
        TerminalSnapshot::try_new(SnapshotParts {
            cols: self.cols.ok_or(DecodeError::Invariant {
                detail: "validated snapshot omitted columns",
            })?,
            rows: self.rows.ok_or(DecodeError::Invariant {
                detail: "validated snapshot omitted rows",
            })?,
            scope: self.scope.ok_or(DecodeError::Invariant {
                detail: "validated snapshot omitted scope",
            })?,
            viewport_origin: self.viewport_origin.ok_or(DecodeError::Invariant {
                detail: "validated snapshot omitted viewport origin",
            })?,
            cells: self.cells,
            palette: self.palette,
            default_foreground: self.default_foreground.ok_or(DecodeError::Invariant {
                detail: "validated snapshot omitted default foreground",
            })?,
            default_background: self.default_background.ok_or(DecodeError::Invariant {
                detail: "validated snapshot omitted default background",
            })?,
            default_cursor: self.default_cursor.ok_or(DecodeError::Invariant {
                detail: "validated snapshot omitted default cursor",
            })?,
            cursor: self.cursor.ok_or(DecodeError::Invariant {
                detail: "validated snapshot omitted cursor",
            })?,
            title: self
                .title
                .take("validated snapshot omitted title presence")?,
            metadata: self.metadata.ok_or(DecodeError::Invariant {
                detail: "validated snapshot omitted metadata",
            })?,
        })
        .map_err(domain_error)
    }
}

impl<'a> DecodeSnapshot<'a> for SnapshotFields {
    type Error = DecodeError;
    fn schema(&mut self, value: u32) -> Result<(), Self::Error> {
        self.schema = Some(value);
        Ok(())
    }
    fn cols(&mut self, value: u32) -> Result<(), Self::Error> {
        self.cols = Some(value);
        Ok(())
    }
    fn rows(&mut self, value: u32) -> Result<(), Self::Error> {
        self.rows = Some(value);
        Ok(())
    }
    fn scope(&mut self, value: Scope) -> Result<(), Self::Error> {
        self.scope = Some(scope(value));
        Ok(())
    }
    fn viewport_origin(&mut self, value: u64) -> Result<(), Self::Error> {
        self.viewport_origin = Some(value);
        Ok(())
    }
    fn cells(
        &mut self,
        values: super::super::generated::SnapshotCellsIter<'a>,
    ) -> Result<(), Self::Error> {
        for value in values {
            self.cells.push(map_cell(value?)?);
        }
        Ok(())
    }
    fn palette(
        &mut self,
        values: super::super::generated::SnapshotPaletteIter<'a>,
    ) -> Result<(), Self::Error> {
        for value in values {
            let slot = self
                .palette
                .get_mut(self.palette_len)
                .ok_or(DecodeError::Invariant {
                    detail: "validated palette exceeds 256 entries",
                })?;
            *slot = map_rgb(value?)?;
            self.palette_len += 1;
        }
        Ok(())
    }
    fn default_foreground(&mut self, value: ValidatedRgb<'a>) -> Result<(), Self::Error> {
        self.default_foreground = Some(map_rgb(value)?);
        Ok(())
    }
    fn default_background(&mut self, value: ValidatedRgb<'a>) -> Result<(), Self::Error> {
        self.default_background = Some(map_rgb(value)?);
        Ok(())
    }
    fn default_cursor(&mut self, value: ValidatedRgb<'a>) -> Result<(), Self::Error> {
        self.default_cursor = Some(map_rgb(value)?);
        Ok(())
    }
    fn cursor(&mut self, value: ValidatedCursor<'a>) -> Result<(), Self::Error> {
        self.cursor = Some(map_cursor(value)?);
        Ok(())
    }
    fn title(&mut self, value: Option<&'a str>) -> Result<(), Self::Error> {
        self.title = super::FieldPresence::Set(
            value
                .map(|text| copied_string(text, "snapshot title"))
                .transpose()?,
        );
        Ok(())
    }
    fn metadata(&mut self, value: ValidatedMetadata<'a>) -> Result<(), Self::Error> {
        self.metadata = Some(map_metadata(value)?);
        Ok(())
    }
    fn upstream_snapshot_invariants(
        &mut self,
        _value: ValidatedSnapshot<'a>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

pub(super) fn map_snapshot(
    value: ValidatedSnapshot<'_>,
    cell_capacity: usize,
) -> Result<TerminalSnapshot, DecodeError> {
    let mut fields = SnapshotFields::new(cell_capacity)?;
    value.visit(&mut fields)?;
    fields.finish()
}
