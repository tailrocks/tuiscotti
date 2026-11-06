use tui_test::SnapshotCell;
use tui_test::terminal::cell::Attrs;

use super::super::super::decode_error::DecodeError;
use super::super::generated::{
    DecodeCell, DecodeColor, DecodeDefaultColor, ValidatedCell, ValidatedColor,
    ValidatedDefaultColor, ValidatedRgb,
};
use super::allocation::{copied_string, domain_error};
use super::snapshot_components::map_underline;

#[derive(Default)]
struct CellFields {
    text: Option<String>,
    width: Option<u16>,
    fg: Option<tui_test::SnapshotColor>,
    bg: Option<tui_test::SnapshotColor>,
    bold: Option<bool>,
    dim: Option<bool>,
    italic: Option<bool>,
    inverse: Option<bool>,
    invisible: Option<bool>,
    strike: Option<bool>,
    blink: Option<bool>,
    underline: Option<tui_test::SnapshotUnderline>,
    underline_color: Option<tui_test::SnapshotColor>,
    link_uri: super::FieldPresence<Option<String>>,
    link_id: super::FieldPresence<Option<String>>,
}

impl<'a> DecodeCell<'a> for CellFields {
    type Error = DecodeError;
    fn text(&mut self, value: &'a str) -> Result<(), Self::Error> {
        self.text = Some(copied_string(value, "snapshot cell text")?);
        Ok(())
    }
    fn width(&mut self, value: u32) -> Result<(), Self::Error> {
        self.width = Some(u16::try_from(value).map_err(|_| DecodeError::Invariant {
            detail: "validated cell width changed",
        })?);
        Ok(())
    }
    fn fg(&mut self, value: ValidatedColor<'a>) -> Result<(), Self::Error> {
        self.fg = Some(map_color(value)?);
        Ok(())
    }
    fn bg(&mut self, value: ValidatedColor<'a>) -> Result<(), Self::Error> {
        self.bg = Some(map_color(value)?);
        Ok(())
    }
    fn bold(&mut self, value: bool) -> Result<(), Self::Error> {
        self.bold = Some(value);
        Ok(())
    }
    fn dim(&mut self, value: bool) -> Result<(), Self::Error> {
        self.dim = Some(value);
        Ok(())
    }
    fn italic(&mut self, value: bool) -> Result<(), Self::Error> {
        self.italic = Some(value);
        Ok(())
    }
    fn inverse(&mut self, value: bool) -> Result<(), Self::Error> {
        self.inverse = Some(value);
        Ok(())
    }
    fn invisible(&mut self, value: bool) -> Result<(), Self::Error> {
        self.invisible = Some(value);
        Ok(())
    }
    fn strike(&mut self, value: bool) -> Result<(), Self::Error> {
        self.strike = Some(value);
        Ok(())
    }
    fn blink(&mut self, value: bool) -> Result<(), Self::Error> {
        self.blink = Some(value);
        Ok(())
    }
    fn underline(&mut self, value: super::super::generated::Underline) -> Result<(), Self::Error> {
        self.underline = Some(map_underline(value));
        Ok(())
    }
    fn underline_color(&mut self, value: ValidatedColor<'a>) -> Result<(), Self::Error> {
        self.underline_color = Some(map_color(value)?);
        Ok(())
    }
    fn link_uri(&mut self, value: Option<&'a str>) -> Result<(), Self::Error> {
        self.link_uri = super::FieldPresence::Set(
            value
                .map(|text| copied_string(text, "cell link URI"))
                .transpose()?,
        );
        Ok(())
    }
    fn link_id(&mut self, value: Option<&'a str>) -> Result<(), Self::Error> {
        self.link_id = super::FieldPresence::Set(
            value
                .map(|text| copied_string(text, "cell link ID"))
                .transpose()?,
        );
        Ok(())
    }
}

#[derive(Default)]
struct ColorFields(Option<tui_test::SnapshotColor>);
impl<'a> DecodeColor<'a> for ColorFields {
    type Error = DecodeError;
    fn default_color(&mut self, _value: ValidatedDefaultColor) -> Result<(), Self::Error> {
        ValidatedDefaultColor::visit(&mut DefaultColorFields);
        self.0 = Some(tui_test::SnapshotColor::Default);
        Ok(())
    }
    fn indexed(&mut self, value: u32) -> Result<(), Self::Error> {
        self.0 = Some(tui_test::SnapshotColor::Indexed(
            u8::try_from(value).map_err(|_| DecodeError::Invariant {
                detail: "validated palette index changed",
            })?,
        ));
        Ok(())
    }
    fn rgb(&mut self, value: ValidatedRgb<'a>) -> Result<(), Self::Error> {
        let rgb = super::snapshot_components::map_rgb(value)?;
        self.0 = Some(tui_test::SnapshotColor::Rgb(rgb.r, rgb.g, rgb.b));
        Ok(())
    }
}

struct DefaultColorFields;
impl DecodeDefaultColor for DefaultColorFields {
    type Error = DecodeError;
}

pub(super) fn map_color(value: ValidatedColor<'_>) -> Result<tui_test::SnapshotColor, DecodeError> {
    let mut fields = ColorFields::default();
    value.visit(&mut fields)?;
    fields.0.ok_or(DecodeError::Invariant {
        detail: "validated color has no selected branch",
    })
}

pub(super) fn map_cell(value: ValidatedCell<'_>) -> Result<SnapshotCell, DecodeError> {
    let mut fields = CellFields::default();
    value.visit(&mut fields)?;
    let mut attrs = Attrs::empty();
    if fields.bold.ok_or(DecodeError::Invariant {
        detail: "validated cell omitted bold",
    })? {
        attrs |= Attrs::BOLD;
    }
    if fields.dim.ok_or(DecodeError::Invariant {
        detail: "validated cell omitted dim",
    })? {
        attrs |= Attrs::DIM;
    }
    if fields.italic.ok_or(DecodeError::Invariant {
        detail: "validated cell omitted italic",
    })? {
        attrs |= Attrs::ITALIC;
    }
    if fields.inverse.ok_or(DecodeError::Invariant {
        detail: "validated cell omitted inverse",
    })? {
        attrs |= Attrs::INVERSE;
    }
    if fields.invisible.ok_or(DecodeError::Invariant {
        detail: "validated cell omitted invisible",
    })? {
        attrs |= Attrs::INVISIBLE;
    }
    if fields.strike.ok_or(DecodeError::Invariant {
        detail: "validated cell omitted strike",
    })? {
        attrs |= Attrs::STRIKE;
    }
    if fields.blink.ok_or(DecodeError::Invariant {
        detail: "validated cell omitted blink",
    })? {
        attrs |= Attrs::BLINK;
    }
    SnapshotCell::new(
        fields.text.ok_or(DecodeError::Invariant {
            detail: "validated cell omitted text",
        })?,
        fields.width.ok_or(DecodeError::Invariant {
            detail: "validated cell omitted width",
        })?,
        fields.fg.ok_or(DecodeError::Invariant {
            detail: "validated cell omitted foreground",
        })?,
        fields.bg.ok_or(DecodeError::Invariant {
            detail: "validated cell omitted background",
        })?,
        attrs,
        fields.underline.ok_or(DecodeError::Invariant {
            detail: "validated cell omitted underline",
        })?,
        fields.underline_color.ok_or(DecodeError::Invariant {
            detail: "validated cell omitted underline color",
        })?,
        fields
            .link_uri
            .take("validated cell omitted link URI presence")?,
        fields
            .link_id
            .take("validated cell omitted link ID presence")?,
    )
    .map_err(domain_error)
}
