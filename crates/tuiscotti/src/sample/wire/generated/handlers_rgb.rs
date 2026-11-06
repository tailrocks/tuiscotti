// @generated required handlers for Rgb.
pub(super) trait DecodeRgb<'a> {
    type Error: From<DecodeError>;
    fn red(&mut self, value: u32) -> Result<(), Self::Error>;
    fn green(&mut self, value: u32) -> Result<(), Self::Error>;
    fn blue(&mut self, value: u32) -> Result<(), Self::Error>;
}
pub(super) trait EncodeRgb {
    fn red(&self) -> u32;
    fn green(&self) -> u32;
    fn blue(&self) -> u32;
}
