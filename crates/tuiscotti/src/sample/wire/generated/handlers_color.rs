// @generated required handlers for Color.
pub(super) trait DecodeColor<'a> {
    type Error: From<DecodeError>;
    fn default_color(&mut self, value: ValidatedDefaultColor) -> Result<(), Self::Error>;
    fn indexed(&mut self, value: u32) -> Result<(), Self::Error>;
    fn rgb(&mut self, value: ValidatedRgb<'a>) -> Result<(), Self::Error>;
}
pub(super) trait EncodeColor {
    type DefaultColor<'a>: EncodeDefaultColor
    where
        Self: 'a;
    fn default_color(&self) -> Option<Self::DefaultColor<'_>>;
    fn indexed(&self) -> Option<u32>;
    type Rgb<'a>: EncodeRgb
    where
        Self: 'a;
    fn rgb(&self) -> Option<Self::Rgb<'_>>;
}
