// @generated required handlers for Cell.
pub(super) trait DecodeCell<'a> {
    type Error: From<DecodeError>;
    fn text(&mut self, value: &'a str) -> Result<(), Self::Error>;
    fn width(&mut self, value: u32) -> Result<(), Self::Error>;
    fn fg(&mut self, value: ValidatedColor<'a>) -> Result<(), Self::Error>;
    fn bg(&mut self, value: ValidatedColor<'a>) -> Result<(), Self::Error>;
    fn bold(&mut self, value: bool) -> Result<(), Self::Error>;
    fn dim(&mut self, value: bool) -> Result<(), Self::Error>;
    fn italic(&mut self, value: bool) -> Result<(), Self::Error>;
    fn inverse(&mut self, value: bool) -> Result<(), Self::Error>;
    fn invisible(&mut self, value: bool) -> Result<(), Self::Error>;
    fn strike(&mut self, value: bool) -> Result<(), Self::Error>;
    fn blink(&mut self, value: bool) -> Result<(), Self::Error>;
    fn underline(&mut self, value: Underline) -> Result<(), Self::Error>;
    fn underline_color(&mut self, value: ValidatedColor<'a>) -> Result<(), Self::Error>;
    fn link_uri(&mut self, value: Option<&'a str>) -> Result<(), Self::Error>;
    fn link_id(&mut self, value: Option<&'a str>) -> Result<(), Self::Error>;
}
pub(super) trait EncodeCell {
    fn text(&self) -> &str;
    fn width(&self) -> u32;
    type Fg<'a>: EncodeColor
    where
        Self: 'a;
    fn fg(&self) -> Self::Fg<'_>;
    type Bg<'a>: EncodeColor
    where
        Self: 'a;
    fn bg(&self) -> Self::Bg<'_>;
    fn bold(&self) -> bool;
    fn dim(&self) -> bool;
    fn italic(&self) -> bool;
    fn inverse(&self) -> bool;
    fn invisible(&self) -> bool;
    fn strike(&self) -> bool;
    fn blink(&self) -> bool;
    fn underline(&self) -> Underline;
    type UnderlineColor<'a>: EncodeColor
    where
        Self: 'a;
    fn underline_color(&self) -> Self::UnderlineColor<'_>;
    fn link_uri(&self) -> Option<&str>;
    fn link_id(&self) -> Option<&str>;
}
