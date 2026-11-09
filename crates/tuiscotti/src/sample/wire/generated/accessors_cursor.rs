// @generated typed borrowed accessors for Cursor.
impl<'a> ValidatedCursor<'a> {
    pub(super) fn col(&self) -> Result<u32, DecodeError> {
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
    pub(super) fn row(&self) -> Result<u32, DecodeError> {
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
    pub(super) fn in_capture(&self) -> Result<bool, DecodeError> {
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
    pub(super) fn visible(&self) -> Result<bool, DecodeError> {
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
    pub(super) fn shape(&self) -> Result<CursorShape, DecodeError> {
        let raw = find_raw(self.span, 5u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Varint(value) => {
                CursorShape::from_wire(i32::try_from(value).map_err(|_| {
                    DecodeError::Invariant {
                        detail: "validated enum changed",
                    }
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
    pub(super) fn color(&self) -> Result<ValidatedRgb<'a>, DecodeError> {
        let raw = find_raw(self.span, 6u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Bytes(span) => ValidatedRgb { span },
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated bytes changed",
                });
            }
        })
    }
    pub(super) fn blink(&self) -> Result<Option<bool>, DecodeError> {
        let raw = find_raw(self.span, 7u32)?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        Ok(Some(match raw {
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
        }))
    }
    pub(super) fn visit<H: DecodeCursor<'a>>(&self, handler: &mut H) -> Result<(), H::Error> {
        handler.col(self.col().map_err(H::Error::from)?)?;
        handler.row(self.row().map_err(H::Error::from)?)?;
        handler.in_capture(self.in_capture().map_err(H::Error::from)?)?;
        handler.visible(self.visible().map_err(H::Error::from)?)?;
        handler.shape(self.shape().map_err(H::Error::from)?)?;
        handler.color(self.color().map_err(H::Error::from)?)?;
        handler.blink(self.blink().map_err(H::Error::from)?)?;
        Ok(())
    }
}
