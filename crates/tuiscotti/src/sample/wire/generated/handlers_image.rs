// @generated required handlers for Image.
pub(super) trait DecodeImage<'a> {
    type Error: From<DecodeError>;
    fn format(&mut self, value: ImageFormat) -> Result<(), Self::Error>;
    fn width(&mut self, value: u32) -> Result<(), Self::Error>;
    fn height(&mut self, value: u32) -> Result<(), Self::Error>;
    fn png_sha256(&mut self, value: &'a str) -> Result<(), Self::Error>;
    fn rgba_sha256(&mut self, value: &'a str) -> Result<(), Self::Error>;
    fn png(&mut self, value: &'a [u8]) -> Result<(), Self::Error>;
}
pub(super) trait EncodeImage {
    fn format(&self) -> ImageFormat;
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn png_sha256(&self) -> &str;
    fn rgba_sha256(&self) -> &str;
    fn png(&self) -> &[u8];
}
