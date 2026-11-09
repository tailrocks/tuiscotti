// @generated typed borrowed accessors for Font.
impl<'a> ValidatedFont<'a> {
    pub(super) fn digest(&self) -> Result<&'a str, DecodeError> {
        let raw = find_raw(self.span, 1u32)?;
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
    pub(super) fn face_index(&self) -> Result<u32, DecodeError> {
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
    pub(super) fn bold(&self) -> Result<bool, DecodeError> {
        let raw = find_raw(self.span, 3u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Varint(value) => match value {
                0 => false,
                1 => true,
                _ => {
                    return Err(DecodeError::Invariant {
                        detail: "validated bool changed",
                    });
                }
            },
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated scalar changed",
                });
            }
        })
    }
    pub(super) fn italic(&self) -> Result<bool, DecodeError> {
        let raw = find_raw(self.span, 4u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Varint(value) => match value {
                0 => false,
                1 => true,
                _ => {
                    return Err(DecodeError::Invariant {
                        detail: "validated bool changed",
                    });
                }
            },
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated scalar changed",
                });
            }
        })
    }
    pub(super) fn fallback_position(&self) -> Result<u64, DecodeError> {
        let raw = find_raw(self.span, 5u32)?;
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
    pub(super) fn visit<H: DecodeFont<'a>>(&self, handler: &mut H) -> Result<(), H::Error> {
        handler.digest(self.digest().map_err(H::Error::from)?)?;
        handler.face_index(self.face_index().map_err(H::Error::from)?)?;
        handler.bold(self.bold().map_err(H::Error::from)?)?;
        handler.italic(self.italic().map_err(H::Error::from)?)?;
        handler.fallback_position(self.fallback_position().map_err(H::Error::from)?)?;
        Ok(())
    }
}
