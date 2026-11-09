// @generated closed oneof view for Color.
pub(super) enum ValidatedColorValue<'a> {
    DefaultColor(ValidatedDefaultColor),
    Indexed(u32),
    Rgb(ValidatedRgb<'a>),
}
impl<'a> ValidatedColor<'a> {
    pub(super) fn value(&self) -> Result<ValidatedColorValue<'a>, DecodeError> {
        if let Some(value) = self.default_color()? {
            return Ok(ValidatedColorValue::DefaultColor(value));
        }
        if let Some(value) = self.indexed()? {
            return Ok(ValidatedColorValue::Indexed(value));
        }
        if let Some(value) = self.rgb()? {
            return Ok(ValidatedColorValue::Rgb(value));
        }
        Err(DecodeError::Invariant {
            detail: "validated color has no selected branch",
        })
    }
}
