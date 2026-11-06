// @generated typed borrowed accessors for Observation.
impl<'a> ValidatedObservation<'a> {
    pub(super) fn note(&self) -> Result<Option<&'a str>, DecodeError> {
        let raw = find_raw(self.span, 1u32)?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        Ok(Some(match raw {
            RawField::Bytes(span) => std::str::from_utf8(&span.bytes[span.start..span.end])
                .map_err(|_| DecodeError::Invariant {
                    detail: "validated UTF-8 changed",
                })?,
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated bytes changed",
                });
            }
        }))
    }
    pub(super) fn visit<H: DecodeObservation<'a>>(&self, handler: &mut H) -> Result<(), H::Error> {
        handler.note(self.note().map_err(H::Error::from)?)?;
        Ok(())
    }
}
