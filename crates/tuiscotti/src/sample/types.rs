#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[doc = "The comparison policy a sample carries."]
pub enum Gate {
    #[doc = "Compare the full canonical state only."]
    State,
    #[doc = "Compare canonical state plus render contract and decoded RGBA pixels."]
    StateAndPixels,
}

impl Gate {
    #[doc = "The wire value: `state` or `state-and-pixels`."]
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::State => "state",
            Self::StateAndPixels => "state-and-pixels",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "The schema identifier as an explicit type: only `SAMPLE_SCHEMA` parses."]
pub struct SampleSchema;

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Sample-level observation: evidence metadata that never joins the compared state."]
pub struct Observation {
    #[doc = "The reviewer note, when present."]
    pub(super) note: Option<String>,
}

impl Observation {
    #[doc = "Empty observation."]
    #[must_use]
    pub const fn empty() -> Self {
        Self { note: None }
    }

    #[doc = "Observation carrying a reviewer note."]
    #[must_use]
    pub fn noted(note: impl Into<String>) -> Self {
        Self {
            note: Some(note.into()),
        }
    }

    #[doc = "The reviewer note, when present."]
    #[must_use]
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "One image in a sample: encoded PNG bytes plus digests over the encoding and over the decoded pixels."]
pub struct SampleImage {
    #[doc = "The image format; only PNG exists."]
    pub(super) format: ImageFormat,
    #[doc = "Decoded image width in pixels."]
    pub(super) width: u32,
    #[doc = "Decoded image height in pixels."]
    pub(super) height: u32,
    #[doc = "SHA-256 over the encoded PNG bytes."]
    pub(super) png_sha256: String,
    #[doc = "SHA-256 over the decoded RGBA pixels (row-major, 8 bits per channel)."]
    pub(super) rgba_sha256: String,
    #[doc = "The encoded PNG bytes."]
    pub(super) png_base64: String,
}

impl SampleImage {
    #[doc = "The image format; only PNG exists."]
    #[must_use]
    pub const fn format(&self) -> ImageFormat {
        self.format
    }

    #[doc = "Decoded image width in pixels."]
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[doc = "Decoded image height in pixels."]
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    #[doc = "SHA-256 over the encoded PNG bytes."]
    #[must_use]
    pub fn png_sha256(&self) -> &str {
        &self.png_sha256
    }

    #[doc = "SHA-256 over the decoded RGBA pixels (row-major, 8 bits per channel)."]
    #[must_use]
    pub fn rgba_sha256(&self) -> &str {
        &self.rgba_sha256
    }

    #[doc = "The encoded PNG bytes."]
    #[must_use]
    pub fn png_base64(&self) -> &str {
        &self.png_base64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Image formats a sample can carry. Only PNG exists."]
pub enum ImageFormat {
    #[doc = "Portable Network Graphics, 8-bit RGBA."]
    Png,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "One font entry in a downstream render contract."]
pub struct ContractFont {
    #[doc = "SHA-256 over the style slot, face index, and bytes of the face."]
    pub(super) digest: String,
    #[doc = "The face index inside its bytes."]
    pub(super) face_index: u32,
    #[doc = "Whether the face serves the bold style slot."]
    pub(super) bold: bool,
    #[doc = "Whether the face serves the italic style slot."]
    pub(super) italic: bool,
    #[doc = "The position of the face in the profile fallback order."]
    pub(super) fallback_position: usize,
}

impl ContractFont {
    #[doc = "SHA-256 over the style slot, face index, and bytes of the face."]
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }

    #[doc = "The face index inside its bytes."]
    #[must_use]
    pub const fn face_index(&self) -> u32 {
        self.face_index
    }

    #[doc = "Whether the face serves the bold style slot."]
    #[must_use]
    pub const fn bold(&self) -> bool {
        self.bold
    }

    #[doc = "Whether the face serves the italic style slot."]
    #[must_use]
    pub const fn italic(&self) -> bool {
        self.italic
    }

    #[doc = "The position of the face in the profile fallback order."]
    #[must_use]
    pub const fn fallback_position(&self) -> usize {
        self.fallback_position
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Cursor visibility recorded in a render contract."]
pub enum CursorPhase {
    #[doc = "The cursor was drawn: the snapshot does not hide it."]
    Shown,
    #[doc = "The cursor was hidden or clipped out of the capture."]
    Hidden,
}

impl From<tui_test::CursorPhase> for CursorPhase {
    fn from(phase: tui_test::CursorPhase) -> Self {
        match phase {
            tui_test::CursorPhase::Shown => Self::Shown,
            tui_test::CursorPhase::Hidden => Self::Hidden,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[doc = "The renderer, font, geometry, and color settings that produced an image."]
pub struct RenderContract {
    #[doc = "The logical profile name the image was rendered with."]
    pub(super) profile: String,
    #[doc = "The downstream contract schema version."]
    pub(super) contract_version: u32,
    #[doc = "The renderer crate version that produced the image."]
    pub(super) renderer_version: String,
    #[doc = "The raster policy version: `pinned-v1` for strict renders."]
    pub(super) raster_policy: String,
    #[doc = "The pinned fonts in fallback order."]
    pub(super) fonts: Vec<ContractFont>,
    #[doc = "The cell width in pixels."]
    pub(super) cell_width: f32,
    #[doc = "The cell height in pixels."]
    pub(super) cell_height: f32,
    #[doc = "The grid font size in pixels."]
    pub(super) font_size: f32,
    #[doc = "The text baseline from the top of each cell in pixels."]
    pub(super) baseline: f32,
    #[doc = "The output scale factor."]
    pub(super) scale: f64,
    #[doc = "Whether a title bar was drawn."]
    pub(super) title_bar: bool,
    #[doc = "SHA-256 over the resolved palette and default colors."]
    pub(super) palette_digest: String,
    #[doc = "Whether the cursor was drawn."]
    pub(super) cursor_phase: CursorPhase,
    #[doc = "The blink intent kept from the snapshot state."]
    pub(super) blink: Option<bool>,
    #[doc = "The `<os>-<arch>` platform the image was rendered on."]
    pub(super) platform_key: String,
}

impl RenderContract {
    pub(crate) fn from_rendered(
        profile: &Profile,
        rendered: &RenderedPng,
    ) -> Result<Self, SampleError> {
        if profile.name() != PROFILE_NAME {
            return Err(SampleError::GateMismatch {
                detail: format!(
                    "profile {:?} is not the pinned profile {PROFILE_NAME:?}",
                    profile.name()
                ),
            });
        }
        let upstream = rendered.contract();
        if upstream.raster_policy() != RASTER_POLICY_PINNED_V1 {
            return Err(SampleError::UnexpectedPolicy {
                policy: upstream.raster_policy().to_string(),
            });
        }
        Ok(Self {
            profile: PROFILE_NAME.to_string(),
            contract_version: CONTRACT_VERSION,
            renderer_version: upstream.renderer_version().to_string(),
            raster_policy: upstream.raster_policy().to_string(),
            fonts: upstream
                .fonts()
                .iter()
                .map(|font| ContractFont {
                    digest: font.digest().to_string(),
                    face_index: font.face_index(),
                    bold: font.bold(),
                    italic: font.italic(),
                    fallback_position: font.fallback_position(),
                })
                .collect(),
            cell_width: upstream.cell_width(),
            cell_height: upstream.cell_height(),
            font_size: upstream.font_size(),
            baseline: upstream.baseline(),
            scale: upstream.scale(),
            title_bar: upstream.title_bar(),
            palette_digest: upstream.palette_digest().to_string(),
            cursor_phase: CursorPhase::from(upstream.cursor_phase()),
            blink: upstream.blink(),
            platform_key: platform_key(),
        })
    }

    #[doc = "The logical profile name the image was rendered with."]
    #[must_use]
    pub fn profile(&self) -> &str {
        &self.profile
    }

    #[doc = "The downstream contract schema version."]
    #[must_use]
    pub const fn contract_version(&self) -> u32 {
        self.contract_version
    }

    #[doc = "The renderer crate version that produced the image."]
    #[must_use]
    pub fn renderer_version(&self) -> &str {
        &self.renderer_version
    }

    #[doc = "The raster policy version: `pinned-v1` for strict renders."]
    #[must_use]
    pub fn raster_policy(&self) -> &str {
        &self.raster_policy
    }

    #[doc = "The pinned fonts in fallback order."]
    #[must_use]
    pub fn fonts(&self) -> &[ContractFont] {
        &self.fonts
    }

    #[doc = "The cell width in pixels."]
    #[must_use]
    pub const fn cell_width(&self) -> f32 {
        self.cell_width
    }

    #[doc = "The cell height in pixels."]
    #[must_use]
    pub const fn cell_height(&self) -> f32 {
        self.cell_height
    }

    #[doc = "The grid font size in pixels."]
    #[must_use]
    pub const fn font_size(&self) -> f32 {
        self.font_size
    }

    #[doc = "The text baseline from the top of each cell in pixels."]
    #[must_use]
    pub const fn baseline(&self) -> f32 {
        self.baseline
    }

    #[doc = "The output scale factor."]
    #[must_use]
    pub const fn scale(&self) -> f64 {
        self.scale
    }

    #[doc = "Whether a title bar was drawn."]
    #[must_use]
    pub const fn title_bar(&self) -> bool {
        self.title_bar
    }

    #[doc = "SHA-256 over the resolved palette and default colors."]
    #[must_use]
    pub fn palette_digest(&self) -> &str {
        &self.palette_digest
    }

    #[doc = "Whether the cursor was drawn."]
    #[must_use]
    pub const fn cursor_phase(&self) -> CursorPhase {
        self.cursor_phase
    }

    #[doc = "The blink intent kept from the snapshot state."]
    #[must_use]
    pub const fn blink(&self) -> Option<bool> {
        self.blink
    }

    #[doc = "The `<os>-<arch>` platform the image was rendered on."]
    #[must_use]
    pub fn platform_key(&self) -> &str {
        &self.platform_key
    }
}
use super::{CONTRACT_VERSION, SampleError};
use crate::render_profile::{PROFILE_NAME, Profile, platform_key};
use tui_test::{RASTER_POLICY_PINNED_V1, RenderedPng};
