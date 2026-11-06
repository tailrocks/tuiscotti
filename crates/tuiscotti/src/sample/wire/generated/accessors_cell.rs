// @generated typed borrowed accessors for Cell.
impl<'a> ValidatedCell<'a> {
    pub(super) fn text(&self) -> Result<&'a str, DecodeError> {
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
    pub(super) fn width(&self) -> Result<u32, DecodeError> {
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
    pub(super) fn fg(&self) -> Result<ValidatedColor<'a>, DecodeError> {
        let raw = find_raw(self.span, 3u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Bytes(span) => ValidatedColor { span },
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated bytes changed",
                });
            }
        })
    }
    pub(super) fn bg(&self) -> Result<ValidatedColor<'a>, DecodeError> {
        let raw = find_raw(self.span, 4u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Bytes(span) => ValidatedColor { span },
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated bytes changed",
                });
            }
        })
    }
    pub(super) fn bold(&self) -> Result<bool, DecodeError> {
        let raw = find_raw(self.span, 5u32)?;
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
    pub(super) fn dim(&self) -> Result<bool, DecodeError> {
        let raw = find_raw(self.span, 6u32)?;
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
        let raw = find_raw(self.span, 7u32)?;
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
    pub(super) fn inverse(&self) -> Result<bool, DecodeError> {
        let raw = find_raw(self.span, 8u32)?;
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
    pub(super) fn invisible(&self) -> Result<bool, DecodeError> {
        let raw = find_raw(self.span, 9u32)?;
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
    pub(super) fn strike(&self) -> Result<bool, DecodeError> {
        let raw = find_raw(self.span, 10u32)?;
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
    pub(super) fn blink(&self) -> Result<bool, DecodeError> {
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
    pub(super) fn underline(&self) -> Result<Underline, DecodeError> {
        let raw = find_raw(self.span, 12u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Varint(value) => {
                Underline::from_wire(i32::try_from(value).map_err(|_| DecodeError::Invariant {
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
    pub(super) fn underline_color(&self) -> Result<ValidatedColor<'a>, DecodeError> {
        let raw = find_raw(self.span, 13u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Bytes(span) => ValidatedColor { span },
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated bytes changed",
                });
            }
        })
    }
    pub(super) fn link_uri(&self) -> Result<Option<&'a str>, DecodeError> {
        let raw = find_raw(self.span, 14u32)?;
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
    pub(super) fn link_id(&self) -> Result<Option<&'a str>, DecodeError> {
        let raw = find_raw(self.span, 15u32)?;
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
    pub(super) fn visit<H: DecodeCell<'a>>(&self, handler: &mut H) -> Result<(), H::Error> {
        handler.text(self.text().map_err(H::Error::from)?)?;
        handler.width(self.width().map_err(H::Error::from)?)?;
        handler.fg(self.fg().map_err(H::Error::from)?)?;
        handler.bg(self.bg().map_err(H::Error::from)?)?;
        handler.bold(self.bold().map_err(H::Error::from)?)?;
        handler.dim(self.dim().map_err(H::Error::from)?)?;
        handler.italic(self.italic().map_err(H::Error::from)?)?;
        handler.inverse(self.inverse().map_err(H::Error::from)?)?;
        handler.invisible(self.invisible().map_err(H::Error::from)?)?;
        handler.strike(self.strike().map_err(H::Error::from)?)?;
        handler.blink(self.blink().map_err(H::Error::from)?)?;
        handler.underline(self.underline().map_err(H::Error::from)?)?;
        handler.underline_color(self.underline_color().map_err(H::Error::from)?)?;
        handler.link_uri(self.link_uri().map_err(H::Error::from)?)?;
        handler.link_id(self.link_id().map_err(H::Error::from)?)?;
        Ok(())
    }
}
