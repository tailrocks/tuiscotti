// @generated typed borrowed accessors for Sample.
impl<'a> ValidatedSample<'a> {
    pub(super) fn schema(&self) -> Result<&'a str, DecodeError> {
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
    pub(super) fn test_id(&self) -> Result<&'a str, DecodeError> {
        let raw = find_raw(self.span, 2u32)?;
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
    pub(super) fn gate(&self) -> Result<Gate, DecodeError> {
        let raw = find_raw(self.span, 3u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Varint(value) => {
                Gate::from_wire(i32::try_from(value).map_err(|_| DecodeError::Invariant {
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
    pub(super) fn snapshot(&self) -> Result<ValidatedSnapshot<'a>, DecodeError> {
        let raw = find_raw(self.span, 4u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Bytes(span) => ValidatedSnapshot { span },
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated bytes changed",
                });
            }
        })
    }
    pub(super) fn render_contract(
        &self,
    ) -> Result<Option<ValidatedRenderContract<'a>>, DecodeError> {
        let raw = find_raw(self.span, 5u32)?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        Ok(Some(match raw {
            RawField::Bytes(span) => ValidatedRenderContract { span },
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated bytes changed",
                });
            }
        }))
    }
    pub(super) fn image(&self) -> Result<Option<ValidatedImage<'a>>, DecodeError> {
        let raw = find_raw(self.span, 6u32)?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        Ok(Some(match raw {
            RawField::Bytes(span) => ValidatedImage { span },
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated bytes changed",
                });
            }
        }))
    }
    pub(super) fn observation(&self) -> Result<ValidatedObservation<'a>, DecodeError> {
        let raw = find_raw(self.span, 7u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Bytes(span) => ValidatedObservation { span },
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated bytes changed",
                });
            }
        })
    }
    pub(super) fn visit<H: DecodeSample<'a>>(&self, handler: &mut H) -> Result<(), H::Error> {
        handler.schema(self.schema().map_err(H::Error::from)?)?;
        handler.test_id(self.test_id().map_err(H::Error::from)?)?;
        handler.gate(self.gate().map_err(H::Error::from)?)?;
        handler.snapshot(self.snapshot().map_err(H::Error::from)?)?;
        handler.render_contract(self.render_contract().map_err(H::Error::from)?)?;
        handler.image(self.image().map_err(H::Error::from)?)?;
        handler.observation(self.observation().map_err(H::Error::from)?)?;
        Ok(())
    }
}
