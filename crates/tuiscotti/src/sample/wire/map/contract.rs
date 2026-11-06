use super::super::super::decode_error::DecodeError;
use super::super::super::{
    ContractFont, CursorPhase, Gate as DomainGate, Observation, RenderContract,
};
use super::super::generated::{
    CursorPhase as WireCursorPhase, DecodeFont, DecodeObservation, DecodeRenderContract,
    Gate as WireGate, ValidatedFont, ValidatedObservation, ValidatedRenderContract,
};
use super::allocation::{copied_string, copied_vec};

pub(super) fn gate(value: WireGate) -> DomainGate {
    match value {
        WireGate::State => DomainGate::State,
        WireGate::StateAndPixels => DomainGate::StateAndPixels,
    }
}

#[derive(Default)]
struct FontFields {
    digest: Option<String>,
    face_index: Option<u32>,
    bold: Option<bool>,
    italic: Option<bool>,
    fallback_position: Option<usize>,
}
impl<'a> DecodeFont<'a> for FontFields {
    type Error = DecodeError;
    fn digest(&mut self, value: &'a str) -> Result<(), Self::Error> {
        self.digest = Some(copied_string(value, "font digest")?);
        Ok(())
    }
    fn face_index(&mut self, value: u32) -> Result<(), Self::Error> {
        self.face_index = Some(value);
        Ok(())
    }
    fn bold(&mut self, value: bool) -> Result<(), Self::Error> {
        self.bold = Some(value);
        Ok(())
    }
    fn italic(&mut self, value: bool) -> Result<(), Self::Error> {
        self.italic = Some(value);
        Ok(())
    }
    fn fallback_position(&mut self, value: u64) -> Result<(), Self::Error> {
        self.fallback_position =
            Some(usize::try_from(value).map_err(|_| DecodeError::Invariant {
                detail: "validated fallback position does not fit usize",
            })?);
        Ok(())
    }
}

fn map_font(value: ValidatedFont<'_>) -> Result<ContractFont, DecodeError> {
    let mut fields = FontFields::default();
    value.visit(&mut fields)?;
    Ok(ContractFont {
        digest: fields.digest.ok_or(DecodeError::Invariant {
            detail: "validated font omitted digest",
        })?,
        face_index: fields.face_index.ok_or(DecodeError::Invariant {
            detail: "validated font omitted face index",
        })?,
        bold: fields.bold.ok_or(DecodeError::Invariant {
            detail: "validated font omitted bold",
        })?,
        italic: fields.italic.ok_or(DecodeError::Invariant {
            detail: "validated font omitted italic",
        })?,
        fallback_position: fields.fallback_position.ok_or(DecodeError::Invariant {
            detail: "validated font omitted fallback position",
        })?,
    })
}

#[derive(Default)]
struct RenderContractFields {
    profile: Option<String>,
    contract_version: Option<u32>,
    renderer_version: Option<String>,
    raster_policy: Option<String>,
    fonts: Vec<ContractFont>,
    cell_width: Option<f32>,
    cell_height: Option<f32>,
    font_size: Option<f32>,
    baseline: Option<f32>,
    scale: Option<f64>,
    title_bar: Option<bool>,
    palette_digest: Option<String>,
    cursor_phase: Option<CursorPhase>,
    blink: super::FieldPresence<Option<bool>>,
    platform_key: Option<String>,
}

impl RenderContractFields {
    fn new(font_capacity: usize) -> Result<Self, DecodeError> {
        Ok(Self {
            fonts: copied_vec(font_capacity, "render contract fonts")?,
            ..Self::default()
        })
    }
    fn finish(self) -> Result<RenderContract, DecodeError> {
        Ok(RenderContract {
            profile: take(self.profile, "render contract omitted profile")?,
            contract_version: take(self.contract_version, "render contract omitted version")?,
            renderer_version: take(
                self.renderer_version,
                "render contract omitted renderer version",
            )?,
            raster_policy: take(self.raster_policy, "render contract omitted raster policy")?,
            fonts: self.fonts,
            cell_width: take(self.cell_width, "render contract omitted cell width")?,
            cell_height: take(self.cell_height, "render contract omitted cell height")?,
            font_size: take(self.font_size, "render contract omitted font size")?,
            baseline: take(self.baseline, "render contract omitted baseline")?,
            scale: take(self.scale, "render contract omitted scale")?,
            title_bar: take(self.title_bar, "render contract omitted title bar")?,
            palette_digest: take(
                self.palette_digest,
                "render contract omitted palette digest",
            )?,
            cursor_phase: take(self.cursor_phase, "render contract omitted cursor phase")?,
            blink: self.blink.take("render contract omitted blink presence")?,
            platform_key: take(self.platform_key, "render contract omitted platform key")?,
        })
    }
}

impl<'a> DecodeRenderContract<'a> for RenderContractFields {
    type Error = DecodeError;
    fn profile(&mut self, value: &'a str) -> Result<(), Self::Error> {
        self.profile = Some(copied_string(value, "render profile")?);
        Ok(())
    }
    fn contract_version(&mut self, value: u32) -> Result<(), Self::Error> {
        self.contract_version = Some(value);
        Ok(())
    }
    fn renderer_version(&mut self, value: &'a str) -> Result<(), Self::Error> {
        self.renderer_version = Some(copied_string(value, "renderer version")?);
        Ok(())
    }
    fn raster_policy(&mut self, value: &'a str) -> Result<(), Self::Error> {
        self.raster_policy = Some(copied_string(value, "raster policy")?);
        Ok(())
    }
    fn fonts(
        &mut self,
        values: super::super::generated::RenderContractFontsIter<'a>,
    ) -> Result<(), Self::Error> {
        for value in values {
            self.fonts.push(map_font(value?)?);
        }
        Ok(())
    }
    fn cell_width(&mut self, value: f32) -> Result<(), Self::Error> {
        self.cell_width = Some(value);
        Ok(())
    }
    fn cell_height(&mut self, value: f32) -> Result<(), Self::Error> {
        self.cell_height = Some(value);
        Ok(())
    }
    fn font_size(&mut self, value: f32) -> Result<(), Self::Error> {
        self.font_size = Some(value);
        Ok(())
    }
    fn baseline(&mut self, value: f32) -> Result<(), Self::Error> {
        self.baseline = Some(value);
        Ok(())
    }
    fn scale(&mut self, value: f64) -> Result<(), Self::Error> {
        self.scale = Some(value);
        Ok(())
    }
    fn title_bar(&mut self, value: bool) -> Result<(), Self::Error> {
        self.title_bar = Some(value);
        Ok(())
    }
    fn palette_digest(&mut self, value: &'a str) -> Result<(), Self::Error> {
        self.palette_digest = Some(copied_string(value, "palette digest")?);
        Ok(())
    }
    fn cursor_phase(&mut self, value: WireCursorPhase) -> Result<(), Self::Error> {
        self.cursor_phase = Some(match value {
            WireCursorPhase::Shown => CursorPhase::Shown,
            WireCursorPhase::Hidden => CursorPhase::Hidden,
        });
        Ok(())
    }
    fn blink(&mut self, value: Option<bool>) -> Result<(), Self::Error> {
        self.blink = super::FieldPresence::Set(value);
        Ok(())
    }
    fn platform_key(&mut self, value: &'a str) -> Result<(), Self::Error> {
        self.platform_key = Some(copied_string(value, "platform key")?);
        Ok(())
    }
}

pub(super) fn map_contract(
    value: ValidatedRenderContract<'_>,
    font_capacity: usize,
) -> Result<RenderContract, DecodeError> {
    let mut fields = RenderContractFields::new(font_capacity)?;
    value.visit(&mut fields)?;
    fields.finish()
}

#[derive(Default)]
struct ObservationFields(super::FieldPresence<Option<String>>);
impl<'a> DecodeObservation<'a> for ObservationFields {
    type Error = DecodeError;
    fn note(&mut self, value: Option<&'a str>) -> Result<(), Self::Error> {
        self.0 = super::FieldPresence::Set(
            value
                .map(|note| copied_string(note, "observation note"))
                .transpose()?,
        );
        Ok(())
    }
}

pub(super) fn map_observation(value: ValidatedObservation<'_>) -> Result<Observation, DecodeError> {
    let mut fields = ObservationFields::default();
    value.visit(&mut fields)?;
    match fields
        .0
        .take("validated observation omitted note presence")?
    {
        Some(note) => Ok(Observation::noted(note)),
        None => Ok(Observation::empty()),
    }
}

fn take<T>(value: Option<T>, detail: &'static str) -> Result<T, DecodeError> {
    value.ok_or(DecodeError::Invariant { detail })
}
