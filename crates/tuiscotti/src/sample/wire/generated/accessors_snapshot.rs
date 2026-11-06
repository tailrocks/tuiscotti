// @generated typed borrowed accessors for Snapshot.
impl<'a> ValidatedSnapshot<'a> {
    pub(super) fn schema(&self) -> Result<u32, DecodeError> {
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
    pub(super) fn cols(&self) -> Result<u32, DecodeError> {
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
    pub(super) fn rows(&self) -> Result<u32, DecodeError> {
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
    pub(super) fn scope(&self) -> Result<Scope, DecodeError> {
        let raw = find_raw(self.span, 4u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Varint(value) => {
                Scope::from_wire(i32::try_from(value).map_err(|_| DecodeError::Invariant {
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
    pub(super) fn viewport_origin(&self) -> Result<u64, DecodeError> {
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
    pub(super) fn cells(&self) -> SnapshotCellsIter<'a> {
        SnapshotCellsIter {
            span: self.span,
            cursor: self.span.start,
            finished: false,
        }
    }
    pub(super) fn palette(&self) -> SnapshotPaletteIter<'a> {
        SnapshotPaletteIter {
            span: self.span,
            cursor: self.span.start,
            finished: false,
        }
    }
    pub(super) fn default_foreground(&self) -> Result<ValidatedRgb<'a>, DecodeError> {
        let raw = find_raw(self.span, 8u32)?;
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
    pub(super) fn default_background(&self) -> Result<ValidatedRgb<'a>, DecodeError> {
        let raw = find_raw(self.span, 9u32)?;
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
    pub(super) fn default_cursor(&self) -> Result<ValidatedRgb<'a>, DecodeError> {
        let raw = find_raw(self.span, 10u32)?;
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
    pub(super) fn cursor(&self) -> Result<ValidatedCursor<'a>, DecodeError> {
        let raw = find_raw(self.span, 11u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Bytes(span) => ValidatedCursor { span },
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated bytes changed",
                });
            }
        })
    }
    pub(super) fn title(&self) -> Result<Option<&'a str>, DecodeError> {
        let raw = find_raw(self.span, 12u32)?;
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
    pub(super) fn metadata(&self) -> Result<ValidatedMetadata<'a>, DecodeError> {
        let raw = find_raw(self.span, 13u32)?;
        let raw = raw.ok_or(DecodeError::Invariant {
            detail: "required field missing after scan",
        })?;
        Ok(match raw {
            RawField::Bytes(span) => ValidatedMetadata { span },
            _ => {
                return Err(DecodeError::Invariant {
                    detail: "validated bytes changed",
                });
            }
        })
    }
    pub(super) fn visit<H: DecodeSnapshot<'a>>(&self, handler: &mut H) -> Result<(), H::Error> {
        handler.schema(self.schema().map_err(H::Error::from)?)?;
        handler.cols(self.cols().map_err(H::Error::from)?)?;
        handler.rows(self.rows().map_err(H::Error::from)?)?;
        handler.scope(self.scope().map_err(H::Error::from)?)?;
        handler.viewport_origin(self.viewport_origin().map_err(H::Error::from)?)?;
        handler.cells(self.cells())?;
        handler.palette(self.palette())?;
        handler.default_foreground(self.default_foreground().map_err(H::Error::from)?)?;
        handler.default_background(self.default_background().map_err(H::Error::from)?)?;
        handler.default_cursor(self.default_cursor().map_err(H::Error::from)?)?;
        handler.cursor(self.cursor().map_err(H::Error::from)?)?;
        handler.title(self.title().map_err(H::Error::from)?)?;
        handler.metadata(self.metadata().map_err(H::Error::from)?)?;
        handler.upstream_snapshot_invariants(*self)?;
        Ok(())
    }
}
pub(super) struct SnapshotCellsIter<'a> {
    span: Span<'a>,
    cursor: usize,
    finished: bool,
}
impl<'a> Iterator for SnapshotCellsIter<'a> {
    type Item = Result<ValidatedCell<'a>, DecodeError>;
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
                Ok(Some((6, raw))) => {
                    let RawField::Bytes(span) = raw else {
                        self.finished = true;
                        return Some(Err(DecodeError::Invariant {
                            detail: "repeated message changed after scan",
                        }));
                    };
                    return Some(Ok(ValidatedCell { span }));
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
pub(super) struct SnapshotPaletteIter<'a> {
    span: Span<'a>,
    cursor: usize,
    finished: bool,
}
impl<'a> Iterator for SnapshotPaletteIter<'a> {
    type Item = Result<ValidatedRgb<'a>, DecodeError>;
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
                Ok(Some((7, raw))) => {
                    let RawField::Bytes(span) = raw else {
                        self.finished = true;
                        return Some(Err(DecodeError::Invariant {
                            detail: "repeated message changed after scan",
                        }));
                    };
                    return Some(Ok(ValidatedRgb { span }));
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
