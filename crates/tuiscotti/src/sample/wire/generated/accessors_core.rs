#[derive(Clone, Copy)]
enum RawField<'a> {
    Varint(u64),
    Fixed64(u64),
    Bytes(Span<'a>),
    Fixed32(u32),
}
fn find_raw(span: Span<'_>, tag: u32) -> Result<Option<RawField<'_>>, DecodeError> {
    let mut cursor = span.start;
    while let Some((found, raw)) = next_raw(span, &mut cursor)? {
        if found == tag {
            return Ok(Some(raw));
        }
    }
    Ok(None)
}
fn next_raw<'a>(
    span: Span<'a>,
    cursor: &mut usize,
) -> Result<Option<(u32, RawField<'a>)>, DecodeError> {
    if *cursor == span.end {
        return Ok(None);
    }
    let path = WirePath::new();
    let key = read_varint(span.bytes, cursor, span.end, path).map_err(to_decode_error)?;
    let tag = u32::try_from(key >> 3).map_err(|_| DecodeError::Invariant {
        detail: "validated field tag changed",
    })?;
    let raw = match key & 7 {
        0 => RawField::Varint(
            read_varint(span.bytes, cursor, span.end, path).map_err(to_decode_error)?,
        ),
        1 => {
            let end = cursor.checked_add(8).ok_or(DecodeError::Invariant {
                detail: "validated fixed64 changed",
            })?;
            let bytes = span.bytes.get(*cursor..end).ok_or(DecodeError::Invariant {
                detail: "validated fixed64 changed",
            })?;
            *cursor = end;
            RawField::Fixed64(u64::from_le_bytes(bytes.try_into().map_err(|_| {
                DecodeError::Invariant {
                    detail: "validated fixed64 changed",
                }
            })?))
        }
        2 => {
            let (start, end) =
                read_length(span.bytes, cursor, span.end, path).map_err(to_decode_error)?;
            RawField::Bytes(Span {
                bytes: span.bytes,
                start,
                end,
            })
        }
        5 => {
            let end = cursor.checked_add(4).ok_or(DecodeError::Invariant {
                detail: "validated fixed32 changed",
            })?;
            let bytes = span.bytes.get(*cursor..end).ok_or(DecodeError::Invariant {
                detail: "validated fixed32 changed",
            })?;
            *cursor = end;
            RawField::Fixed32(u32::from_le_bytes(bytes.try_into().map_err(|_| {
                DecodeError::Invariant {
                    detail: "validated fixed32 changed",
                }
            })?))
        }
        _ => {
            return Err(DecodeError::Invariant {
                detail: "validated wire type changed",
            });
        }
    };
    Ok(Some((tag, raw)))
}
