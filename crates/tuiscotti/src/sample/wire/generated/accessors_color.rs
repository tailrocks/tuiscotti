// @generated typed borrowed accessors for Color.
impl<'a> ValidatedColor<'a> {
    pub(super) fn default_color(&self) -> Result<Option<ValidatedDefaultColor>, DecodeError> {
        let raw = find_raw(self.span, 1u32)?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        Ok(Some(match raw {
            RawField::Bytes(_) => ValidatedDefaultColor { _private: () },
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated bytes changed",
                });
            }
        }))
    }
    pub(super) fn indexed(&self) -> Result<Option<u32>, DecodeError> {
        let raw = find_raw(self.span, 2u32)?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        Ok(Some(match raw {
            RawField::Varint(value) => {
                u32::try_from(value).map_err(|_| DecodeError::Invariant {
                    detail: "validated uint32 changed",
                })?
            }
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated scalar changed",
                });
            }
        }))
    }
    pub(super) fn rgb(&self) -> Result<Option<ValidatedRgb<'a>>, DecodeError> {
        let raw = find_raw(self.span, 3u32)?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        Ok(Some(match raw {
            RawField::Bytes(span) => ValidatedRgb { span },
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated bytes changed",
                });
            }
        }))
    }
    pub(super) fn visit<H: DecodeColor<'a>>(&self, handler: &mut H) -> Result<(), H::Error> {
        match self.value().map_err(H::Error::from)? {
            ValidatedColorValue::DefaultColor(value) => handler.default_color(value)?,
            ValidatedColorValue::Indexed(value) => handler.indexed(value)?,
            ValidatedColorValue::Rgb(value) => handler.rgb(value)?,
        }
        Ok(())
    }
}
