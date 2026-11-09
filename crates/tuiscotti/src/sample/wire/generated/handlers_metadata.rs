// @generated required handlers for Metadata.
pub(super) trait DecodeMetadata<'a> {
    type Error: From<DecodeError>;
    fn captured_at_ms(&mut self, value: u64) -> Result<(), Self::Error>;
    fn revision(&mut self, value: u64) -> Result<(), Self::Error>;
    fn backend(&mut self, value: &'a str) -> Result<(), Self::Error>;
    fn session(&mut self, value: &'a str) -> Result<(), Self::Error>;
    fn source(&mut self, value: Source) -> Result<(), Self::Error>;
}
pub(super) trait EncodeMetadata {
    fn captured_at_ms(&self) -> u64;
    fn revision(&self) -> u64;
    fn backend(&self) -> &str;
    fn session(&self) -> &str;
    fn source(&self) -> Source;
}
