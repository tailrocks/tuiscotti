// @generated typed borrowed accessors for RenderContract.
impl<'a> ValidatedRenderContract<'a> {
    pub(super) fn profile(&self) -> Result<&'a str, DecodeError> {
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
    pub(super) fn contract_version(&self) -> Result<u32, DecodeError> {
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
    pub(super) fn renderer_version(&self) -> Result<&'a str, DecodeError> {
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
    pub(super) fn raster_policy(&self) -> Result<&'a str, DecodeError> {
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
    pub(super) fn fonts(&self) -> RenderContractFontsIter<'a> {
        RenderContractFontsIter {
            span: self.span,
            cursor: self.span.start,
            finished: false,
        }
    }
    pub(super) fn cell_width(&self) -> Result<f32, DecodeError> {
        let raw = find_raw(self.span, 6u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Fixed32(value) => f32::from_bits(value),
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated float changed",
                });
            }
        })
    }
    pub(super) fn cell_height(&self) -> Result<f32, DecodeError> {
        let raw = find_raw(self.span, 7u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Fixed32(value) => f32::from_bits(value),
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated float changed",
                });
            }
        })
    }
    pub(super) fn font_size(&self) -> Result<f32, DecodeError> {
        let raw = find_raw(self.span, 8u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Fixed32(value) => f32::from_bits(value),
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated float changed",
                });
            }
        })
    }
    pub(super) fn baseline(&self) -> Result<f32, DecodeError> {
        let raw = find_raw(self.span, 9u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Fixed32(value) => f32::from_bits(value),
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated float changed",
                });
            }
        })
    }
    pub(super) fn scale(&self) -> Result<f64, DecodeError> {
        let raw = find_raw(self.span, 10u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Fixed64(value) => f64::from_bits(value),
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated double changed",
                });
            }
        })
    }
    pub(super) fn title_bar(&self) -> Result<bool, DecodeError> {
        let raw = find_raw(self.span, 11u32)?;
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
    pub(super) fn palette_digest(&self) -> Result<&'a str, DecodeError> {
        let raw = find_raw(self.span, 12u32)?;
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
    pub(super) fn cursor_phase(&self) -> Result<CursorPhase, DecodeError> {
        let raw = find_raw(self.span, 13u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Varint(value) => {
                CursorPhase::from_wire(i32::try_from(value).map_err(|_| {
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
    pub(super) fn blink(&self) -> Result<Option<bool>, DecodeError> {
        let raw = find_raw(self.span, 14u32)?;
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
    pub(super) fn platform_key(&self) -> Result<&'a str, DecodeError> {
        let raw = find_raw(self.span, 15u32)?;
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
    pub(super) fn visit<H: DecodeRenderContract<'a>>(
        &self,
        handler: &mut H,
    ) -> Result<(), H::Error> {
        handler.profile(self.profile().map_err(H::Error::from)?)?;
        handler.contract_version(self.contract_version().map_err(H::Error::from)?)?;
        handler.renderer_version(self.renderer_version().map_err(H::Error::from)?)?;
        handler.raster_policy(self.raster_policy().map_err(H::Error::from)?)?;
        handler.fonts(self.fonts())?;
        handler.cell_width(self.cell_width().map_err(H::Error::from)?)?;
        handler.cell_height(self.cell_height().map_err(H::Error::from)?)?;
        handler.font_size(self.font_size().map_err(H::Error::from)?)?;
        handler.baseline(self.baseline().map_err(H::Error::from)?)?;
        handler.scale(self.scale().map_err(H::Error::from)?)?;
        handler.title_bar(self.title_bar().map_err(H::Error::from)?)?;
        handler.palette_digest(self.palette_digest().map_err(H::Error::from)?)?;
        handler.cursor_phase(self.cursor_phase().map_err(H::Error::from)?)?;
        handler.blink(self.blink().map_err(H::Error::from)?)?;
        handler.platform_key(self.platform_key().map_err(H::Error::from)?)?;
        Ok(())
    }
}
pub(super) struct RenderContractFontsIter<'a> {
    span: Span<'a>,
    cursor: usize,
    finished: bool,
}
impl<'a> Iterator for RenderContractFontsIter<'a> {
    type Item = Result<ValidatedFont<'a>, DecodeError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        loop {
            match next_raw(self.span, &mut self.cursor) {
                Ok(None) => {
                    self.finished = true;
                    return None;
                }
                Ok(Some((5, raw))) => {
                    let RawField::Bytes(span) = raw else {
                        self.finished = true;
                        return Some(Err(DecodeError::Invariant {
                            detail: "repeated message changed after scan",
                        }));
                    };
                    return Some(Ok(ValidatedFont { span }));
                }
                Ok(Some(_)) => {}
                Err(error) => {
                    self.finished = true;
                    return Some(Err(error));
                }
            }
        }
    }
}
