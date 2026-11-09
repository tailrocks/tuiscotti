// @generated required handlers for Cursor.
pub(super) trait DecodeCursor<'a> {
    type Error: From<DecodeError>;
    fn col(&mut self, value: u32) -> Result<(), Self::Error>;
    fn row(&mut self, value: u32) -> Result<(), Self::Error>;
    fn in_capture(&mut self, value: bool) -> Result<(), Self::Error>;
    fn visible(&mut self, value: bool) -> Result<(), Self::Error>;
    fn shape(&mut self, value: CursorShape) -> Result<(), Self::Error>;
    fn color(&mut self, value: ValidatedRgb<'a>) -> Result<(), Self::Error>;
    fn blink(&mut self, value: Option<bool>) -> Result<(), Self::Error>;
}
pub(super) trait EncodeCursor {
    fn col(&self) -> u32;
    fn row(&self) -> u32;
    fn in_capture(&self) -> bool;
    fn visible(&self) -> bool;
    fn shape(&self) -> CursorShape;
    type Color<'a>: EncodeRgb
    where
        Self: 'a;
    fn color(&self) -> Self::Color<'_>;
    fn blink(&self) -> Option<bool>;
}
