// @generated required handlers for RenderContract.
pub(super) trait DecodeRenderContract<'a> {
    type Error: From<DecodeError>;
    fn profile(&mut self, value: &'a str) -> Result<(), Self::Error>;
    fn contract_version(&mut self, value: u32) -> Result<(), Self::Error>;
    fn renderer_version(&mut self, value: &'a str) -> Result<(), Self::Error>;
    fn raster_policy(&mut self, value: &'a str) -> Result<(), Self::Error>;
    fn fonts(&mut self, value: RenderContractFontsIter<'a>) -> Result<(), Self::Error>;
    fn cell_width(&mut self, value: f32) -> Result<(), Self::Error>;
    fn cell_height(&mut self, value: f32) -> Result<(), Self::Error>;
    fn font_size(&mut self, value: f32) -> Result<(), Self::Error>;
    fn baseline(&mut self, value: f32) -> Result<(), Self::Error>;
    fn scale(&mut self, value: f64) -> Result<(), Self::Error>;
    fn title_bar(&mut self, value: bool) -> Result<(), Self::Error>;
    fn palette_digest(&mut self, value: &'a str) -> Result<(), Self::Error>;
    fn cursor_phase(&mut self, value: CursorPhase) -> Result<(), Self::Error>;
    fn blink(&mut self, value: Option<bool>) -> Result<(), Self::Error>;
    fn platform_key(&mut self, value: &'a str) -> Result<(), Self::Error>;
}
pub(super) trait EncodeRenderContract {
    fn profile(&self) -> &str;
    fn contract_version(&self) -> u32;
    fn renderer_version(&self) -> &str;
    fn raster_policy(&self) -> &str;
    type Fonts<'a>: Iterator<Item = Self::FontsItem<'a>>
    where
        Self: 'a;
    type FontsItem<'a>: EncodeFont
    where
        Self: 'a;
    fn fonts(&self) -> Self::Fonts<'_>;
    fn cell_width(&self) -> f32;
    fn cell_height(&self) -> f32;
    fn font_size(&self) -> f32;
    fn baseline(&self) -> f32;
    fn scale(&self) -> f64;
    fn title_bar(&self) -> bool;
    fn palette_digest(&self) -> &str;
    fn cursor_phase(&self) -> CursorPhase;
    fn blink(&self) -> Option<bool>;
    fn platform_key(&self) -> &str;
}
