use tui_test::profile::Rgb;
use tui_test::{
    SnapshotCursor, SnapshotCursorShape, SnapshotMetadata, SnapshotScope, SnapshotSource,
    SnapshotUnderline,
};

use super::super::super::decode_error::DecodeError;
use super::super::generated::{
    CursorShape, DecodeCursor, DecodeMetadata, DecodeRgb, Scope, Source, Underline,
    ValidatedCursor, ValidatedMetadata, ValidatedRgb,
};
use super::allocation::copied_string;

pub(in crate::sample::wire::map) fn scope(value: Scope) -> SnapshotScope {
    match value {
        Scope::Viewport => SnapshotScope::Viewport,
        Scope::ActiveScreen => SnapshotScope::ActiveScreen,
        Scope::FullHistory => SnapshotScope::FullHistory,
    }
}

fn cursor_shape(value: CursorShape) -> SnapshotCursorShape {
    match value {
        CursorShape::Block => SnapshotCursorShape::Block,
        CursorShape::Underline => SnapshotCursorShape::Underline,
        CursorShape::Bar => SnapshotCursorShape::Bar,
    }
}

pub(in crate::sample::wire::map) fn map_underline(value: Underline) -> SnapshotUnderline {
    match value {
        Underline::None => SnapshotUnderline::None,
        Underline::Single => SnapshotUnderline::Single,
        Underline::Double => SnapshotUnderline::Double,
        Underline::Curly => SnapshotUnderline::Curly,
        Underline::Dotted => SnapshotUnderline::Dotted,
        Underline::Dashed => SnapshotUnderline::Dashed,
    }
}

#[derive(Default)]
struct RgbFields {
    red: Option<u8>,
    green: Option<u8>,
    blue: Option<u8>,
}
impl DecodeRgb<'_> for RgbFields {
    type Error = DecodeError;
    fn red(&mut self, value: u32) -> Result<(), Self::Error> {
        self.red = Some(u8::try_from(value).map_err(|_| DecodeError::Invariant {
            detail: "validated RGB red channel changed",
        })?);
        Ok(())
    }
    fn green(&mut self, value: u32) -> Result<(), Self::Error> {
        self.green = Some(u8::try_from(value).map_err(|_| DecodeError::Invariant {
            detail: "validated RGB green channel changed",
        })?);
        Ok(())
    }
    fn blue(&mut self, value: u32) -> Result<(), Self::Error> {
        self.blue = Some(u8::try_from(value).map_err(|_| DecodeError::Invariant {
            detail: "validated RGB blue channel changed",
        })?);
        Ok(())
    }
}

pub(in crate::sample::wire::map) fn map_rgb(value: ValidatedRgb<'_>) -> Result<Rgb, DecodeError> {
    let mut fields = RgbFields::default();
    value.visit(&mut fields)?;
    Ok(Rgb::new(
        fields.red.ok_or(DecodeError::Invariant {
            detail: "validated RGB omitted red",
        })?,
        fields.green.ok_or(DecodeError::Invariant {
            detail: "validated RGB omitted green",
        })?,
        fields.blue.ok_or(DecodeError::Invariant {
            detail: "validated RGB omitted blue",
        })?,
    ))
}

#[derive(Default)]
struct MetadataFields {
    captured_at_ms: Option<u64>,
    revision: Option<u64>,
    backend: Option<String>,
    session: Option<String>,
    source: Option<SnapshotSource>,
}
impl<'a> DecodeMetadata<'a> for MetadataFields {
    type Error = DecodeError;
    fn captured_at_ms(&mut self, value: u64) -> Result<(), Self::Error> {
        self.captured_at_ms = Some(value);
        Ok(())
    }
    fn revision(&mut self, value: u64) -> Result<(), Self::Error> {
        self.revision = Some(value);
        Ok(())
    }
    fn backend(&mut self, value: &'a str) -> Result<(), Self::Error> {
        self.backend = Some(copied_string(value, "snapshot metadata backend")?);
        Ok(())
    }
    fn session(&mut self, value: &'a str) -> Result<(), Self::Error> {
        self.session = Some(copied_string(value, "snapshot metadata session")?);
        Ok(())
    }
    fn source(&mut self, value: Source) -> Result<(), Self::Error> {
        self.source = Some(match value {
            Source::Live => SnapshotSource::Live,
            Source::Offline => SnapshotSource::Offline,
        });
        Ok(())
    }
}

pub(in crate::sample::wire::map) fn map_metadata(
    value: ValidatedMetadata<'_>,
) -> Result<SnapshotMetadata, DecodeError> {
    let mut fields = MetadataFields::default();
    value.visit(&mut fields)?;
    Ok(SnapshotMetadata {
        captured_at_ms: fields.captured_at_ms.ok_or(DecodeError::Invariant {
            detail: "validated metadata omitted capture time",
        })?,
        revision: fields.revision.ok_or(DecodeError::Invariant {
            detail: "validated metadata omitted revision",
        })?,
        backend: fields.backend.ok_or(DecodeError::Invariant {
            detail: "validated metadata omitted backend",
        })?,
        session: fields.session.ok_or(DecodeError::Invariant {
            detail: "validated metadata omitted session",
        })?,
        source: fields.source.ok_or(DecodeError::Invariant {
            detail: "validated metadata omitted source",
        })?,
    })
}

#[derive(Default)]
struct CursorFields {
    col: Option<u32>,
    row: Option<u32>,
    in_capture: Option<bool>,
    visible: Option<bool>,
    shape: Option<SnapshotCursorShape>,
    color: Option<Rgb>,
    blink: super::FieldPresence<Option<bool>>,
}
impl<'a> DecodeCursor<'a> for CursorFields {
    type Error = DecodeError;
    fn col(&mut self, value: u32) -> Result<(), Self::Error> {
        self.col = Some(value);
        Ok(())
    }
    fn row(&mut self, value: u32) -> Result<(), Self::Error> {
        self.row = Some(value);
        Ok(())
    }
    fn in_capture(&mut self, value: bool) -> Result<(), Self::Error> {
        self.in_capture = Some(value);
        Ok(())
    }
    fn visible(&mut self, value: bool) -> Result<(), Self::Error> {
        self.visible = Some(value);
        Ok(())
    }
    fn shape(&mut self, value: CursorShape) -> Result<(), Self::Error> {
        self.shape = Some(cursor_shape(value));
        Ok(())
    }
    fn color(&mut self, value: ValidatedRgb<'a>) -> Result<(), Self::Error> {
        self.color = Some(map_rgb(value)?);
        Ok(())
    }
    fn blink(&mut self, value: Option<bool>) -> Result<(), Self::Error> {
        self.blink = super::FieldPresence::Set(value);
        Ok(())
    }
}

pub(in crate::sample::wire::map) fn map_cursor(
    value: ValidatedCursor<'_>,
) -> Result<SnapshotCursor, DecodeError> {
    let mut fields = CursorFields::default();
    value.visit(&mut fields)?;
    Ok(SnapshotCursor::new(
        fields.col.ok_or(DecodeError::Invariant {
            detail: "validated cursor omitted column",
        })?,
        fields.row.ok_or(DecodeError::Invariant {
            detail: "validated cursor omitted row",
        })?,
        fields.in_capture.ok_or(DecodeError::Invariant {
            detail: "validated cursor omitted capture flag",
        })?,
        fields.visible.ok_or(DecodeError::Invariant {
            detail: "validated cursor omitted visibility",
        })?,
        fields.shape.ok_or(DecodeError::Invariant {
            detail: "validated cursor omitted shape",
        })?,
        fields.color.ok_or(DecodeError::Invariant {
            detail: "validated cursor omitted color",
        })?,
        fields
            .blink
            .take("validated cursor omitted blink presence")?,
    ))
}
