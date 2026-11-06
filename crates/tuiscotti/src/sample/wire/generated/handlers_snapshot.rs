// @generated required handlers for Snapshot.
pub(super) trait DecodeSnapshot<'a> {
    type Error: From<DecodeError>;
    fn schema(&mut self, value: u32) -> Result<(), Self::Error>;
    fn cols(&mut self, value: u32) -> Result<(), Self::Error>;
    fn rows(&mut self, value: u32) -> Result<(), Self::Error>;
    fn scope(&mut self, value: Scope) -> Result<(), Self::Error>;
    fn viewport_origin(&mut self, value: u64) -> Result<(), Self::Error>;
    fn cells(&mut self, value: SnapshotCellsIter<'a>) -> Result<(), Self::Error>;
    fn palette(&mut self, value: SnapshotPaletteIter<'a>) -> Result<(), Self::Error>;
    fn default_foreground(&mut self, value: ValidatedRgb<'a>) -> Result<(), Self::Error>;
    fn default_background(&mut self, value: ValidatedRgb<'a>) -> Result<(), Self::Error>;
    fn default_cursor(&mut self, value: ValidatedRgb<'a>) -> Result<(), Self::Error>;
    fn cursor(&mut self, value: ValidatedCursor<'a>) -> Result<(), Self::Error>;
    fn title(&mut self, value: Option<&'a str>) -> Result<(), Self::Error>;
    fn metadata(&mut self, value: ValidatedMetadata<'a>) -> Result<(), Self::Error>;
    fn upstream_snapshot_invariants(
        &mut self,
        value: ValidatedSnapshot<'a>,
    ) -> Result<(), Self::Error>;
}
pub(super) trait EncodeSnapshot {
    fn schema(&self) -> u32;
    fn cols(&self) -> u32;
    fn rows(&self) -> u32;
    fn scope(&self) -> Scope;
    fn viewport_origin(&self) -> u64;
    type Cells<'a>: Iterator<Item = Self::CellsItem<'a>>
    where
        Self: 'a;
    type CellsItem<'a>: EncodeCell
    where
        Self: 'a;
    fn cells(&self) -> Self::Cells<'_>;
    type Palette<'a>: Iterator<Item = Self::PaletteItem<'a>>
    where
        Self: 'a;
    type PaletteItem<'a>: EncodeRgb
    where
        Self: 'a;
    fn palette(&self) -> Self::Palette<'_>;
    type DefaultForeground<'a>: EncodeRgb
    where
        Self: 'a;
    fn default_foreground(&self) -> Self::DefaultForeground<'_>;
    type DefaultBackground<'a>: EncodeRgb
    where
        Self: 'a;
    fn default_background(&self) -> Self::DefaultBackground<'_>;
    type DefaultCursor<'a>: EncodeRgb
    where
        Self: 'a;
    fn default_cursor(&self) -> Self::DefaultCursor<'_>;
    type Cursor<'a>: EncodeCursor
    where
        Self: 'a;
    fn cursor(&self) -> Self::Cursor<'_>;
    fn title(&self) -> Option<&str>;
    type Metadata<'a>: EncodeMetadata
    where
        Self: 'a;
    fn metadata(&self) -> Self::Metadata<'_>;
}
