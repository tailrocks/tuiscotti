// @generated typed borrowed accessors for Metadata.
impl<'a> ValidatedMetadata<'a> {
    pub(super) fn captured_at_ms(&self) -> Result<u64, DecodeError> {
        let raw = find_raw(self.span, 1u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Varint(value) => value,
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated scalar changed",
                });
            }
        })
    }
    pub(super) fn revision(&self) -> Result<u64, DecodeError> {
        let raw = find_raw(self.span, 2u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Varint(value) => value,
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated scalar changed",
                });
            }
        })
    }
    pub(super) fn backend(&self) -> Result<&'a str, DecodeError> {
        let raw = find_raw(self.span, 3u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Bytes(span) => std::str::from_utf8(&span.bytes[span.start..span.end])
                .map_err(|_| DecodeError::Invariant {
                    detail: "validated UTF-8 changed",
                })?,
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated bytes changed",
                });
            }
        })
    }
    pub(super) fn session(&self) -> Result<&'a str, DecodeError> {
        let raw = find_raw(self.span, 4u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Bytes(span) => std::str::from_utf8(&span.bytes[span.start..span.end])
                .map_err(|_| DecodeError::Invariant {
                    detail: "validated UTF-8 changed",
                })?,
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated bytes changed",
                });
            }
        })
    }
    pub(super) fn source(&self) -> Result<Source, DecodeError> {
        let raw = find_raw(self.span, 5u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Varint(value) => {
                Source::from_wire(i32::try_from(value).map_err(|_| DecodeError::Invariant {
                    detail: "validated enum changed",
                })?)
                .ok_or(DecodeError::Invariant {
                    detail: "validated enum changed",
                })?
            }
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated scalar changed",
                });
            }
        })
    }
    pub(super) fn visit<H: DecodeMetadata<'a>>(&self, handler: &mut H) -> Result<(), H::Error> {
        handler.captured_at_ms(self.captured_at_ms().map_err(H::Error::from)?)?;
        handler.revision(self.revision().map_err(H::Error::from)?)?;
        handler.backend(self.backend().map_err(H::Error::from)?)?;
        handler.session(self.session().map_err(H::Error::from)?)?;
        handler.source(self.source().map_err(H::Error::from)?)?;
        Ok(())
    }
}
