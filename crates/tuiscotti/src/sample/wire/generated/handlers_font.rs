// @generated required handlers for Font.
pub(super) trait DecodeFont<'a> {
    type Error: From<DecodeError>;
    fn digest(&mut self, value: &'a str) -> Result<(), Self::Error>;
    fn face_index(&mut self, value: u32) -> Result<(), Self::Error>;
    fn bold(&mut self, value: bool) -> Result<(), Self::Error>;
    fn italic(&mut self, value: bool) -> Result<(), Self::Error>;
    fn fallback_position(&mut self, value: u64) -> Result<(), Self::Error>;
}
pub(super) trait EncodeFont {
    fn digest(&self) -> &str;
    fn face_index(&self) -> u32;
    fn bold(&self) -> bool;
    fn italic(&self) -> bool;
    fn fallback_position(&self) -> u64;
}
