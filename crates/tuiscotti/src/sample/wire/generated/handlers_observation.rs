// @generated required handlers for Observation.
pub(super) trait DecodeObservation<'a> {
    type Error: From<DecodeError>;
    fn note(&mut self, value: Option<&'a str>) -> Result<(), Self::Error>;
}
pub(super) trait EncodeObservation {
    fn note(&self) -> Option<&str>;
}
