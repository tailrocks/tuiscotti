// @generated typed borrowed accessors for Rgb.
impl<'a> ValidatedRgb<'a> {
    pub(super) fn red(&self) -> Result<u32, DecodeError> {
        let raw = find_raw(self.span, 1u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
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
        })
    }
    pub(super) fn green(&self) -> Result<u32, DecodeError> {
        let raw = find_raw(self.span, 2u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
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
        })
    }
    pub(super) fn blue(&self) -> Result<u32, DecodeError> {
        let raw = find_raw(self.span, 3u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
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
        })
    }
    pub(super) fn visit<H: DecodeRgb<'a>>(&self, handler: &mut H) -> Result<(), H::Error> {
        handler.red(self.red().map_err(H::Error::from)?)?;
        handler.green(self.green().map_err(H::Error::from)?)?;
        handler.blue(self.blue().map_err(H::Error::from)?)?;
        Ok(())
    }
}
