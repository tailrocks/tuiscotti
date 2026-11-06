// @generated required handlers for Sample.
pub(super) trait DecodeSample<'a> {
    type Error: From<DecodeError>;
    fn schema(&mut self, value: &'a str) -> Result<(), Self::Error>;
    fn test_id(&mut self, value: &'a str) -> Result<(), Self::Error>;
    fn gate(&mut self, value: Gate) -> Result<(), Self::Error>;
    fn snapshot(&mut self, value: ValidatedSnapshot<'a>) -> Result<(), Self::Error>;
    fn render_contract(
        &mut self,
        value: Option<ValidatedRenderContract<'a>>,
    ) -> Result<(), Self::Error>;
    fn image(&mut self, value: Option<ValidatedImage<'a>>) -> Result<(), Self::Error>;
    fn observation(&mut self, value: ValidatedObservation<'a>) -> Result<(), Self::Error>;
}
pub(super) trait EncodeSample {
    fn schema(&self) -> &str;
    fn test_id(&self) -> &str;
    fn gate(&self) -> Gate;
    type Snapshot<'a>: EncodeSnapshot
    where
        Self: 'a;
    fn snapshot(&self) -> Self::Snapshot<'_>;
    type RenderContract<'a>: EncodeRenderContract
    where
        Self: 'a;
    fn render_contract(&self) -> Option<Self::RenderContract<'_>>;
    type Image<'a>: EncodeImage
    where
        Self: 'a;
    fn image(&self) -> Option<Self::Image<'_>>;
    type Observation<'a>: EncodeObservation
    where
        Self: 'a;
    fn observation(&self) -> Self::Observation<'_>;
}
