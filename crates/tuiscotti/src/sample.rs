#[doc = include_str!("../docs/sample.md")]
use std::fmt::{Display, Formatter};
use std::io::Read as _;

use base64::Engine as _;
use image::ImageDecoder as _;
use serde::de::Error as _;
use sha2::{Digest, Sha256};
use tui_test::{
    MAX_LINK_BYTES, MAX_TITLE_BYTES, RASTER_POLICY_PINNED_V1, RenderError, RenderedPng,
    SNAPSHOT_SCHEMA_VERSION, TerminalSnapshot,
};

use crate::render_profile::{PROFILE_NAME, Profile, platform_key};
use crate::report::atomic_write;

#[doc = "Sample schema identifier. Only this value parses."]
pub const SAMPLE_SCHEMA: &str = "tuiscotti.sample.v1";

#[doc = "Schema version of the downstream render contract in samples."]
pub const CONTRACT_VERSION: u32 = 1;

#[doc = "Encoded sample file limit: 64 MiB, enforced with bounded reads."]
pub const MAX_SAMPLE_FILE_BYTES: u64 = 64 * 1024 * 1024;
#[doc = "Cells in one sample."]
pub const MAX_SAMPLE_CELLS: usize = 1_000_000;
#[doc = "UTF-8 bytes in one cell string."]
pub const MAX_CELL_TEXT_BYTES: usize = 4_096;
#[doc = "Decoded RGBA byte limit: 256 MiB, validated before allocation."]
pub const MAX_RGBA_BYTES: u64 = 256 * 1024 * 1024;
#[doc = "One image dimension limit: 32,768 pixels."]
pub const MAX_IMAGE_DIM: u32 = 32_768;
#[doc = "Test ID length limit: 240 ASCII bytes."]
pub const MAX_TEST_ID_BYTES: usize = 240;
#[doc = "One test ID segment limit: 64 ASCII bytes."]
pub const MAX_TEST_ID_SEGMENT_BYTES: usize = 64;

pub(crate) fn read_bounded_optional(
    path: &std::path::Path,
) -> Result<Option<Vec<u8>>, SampleError> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(SampleError::Io { source }),
    };
    let mut bounded = file.take(MAX_SAMPLE_FILE_BYTES + 1);
    let mut bytes = Vec::new();
    bounded
        .read_to_end(&mut bytes)
        .map_err(|source| SampleError::Io { source })?;
    if bytes.len() as u64 > MAX_SAMPLE_FILE_BYTES {
        return Err(SampleError::TooLarge {
            what: "evidence file",
            bytes: bytes.len() as u64,
            limit: MAX_SAMPLE_FILE_BYTES,
        });
    }
    Ok(Some(bytes))
}

pub(crate) fn read_bounded(path: &std::path::Path) -> Result<Vec<u8>, SampleError> {
    match read_bounded_optional(path)? {
        Some(bytes) => Ok(bytes),
        None => Err(SampleError::Io {
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("evidence file {} is missing", path.display()),
            ),
        }),
    }
}

#[derive(Debug)]
#[doc = "Every failure a sample operation can report, with sources retained."]
#[non_exhaustive]
pub enum SampleError {
    #[doc = "The encoded file passes the 64 MiB limit."]
    TooLarge {
        #[doc = "What was measured."]
        what: &'static str,
        #[doc = "The rejected byte count."]
        bytes: u64,
        #[doc = "The accepted limit."]
        limit: u64,
    },
    #[doc = "A test ID breaks its shape rules."]
    InvalidTestId {
        #[doc = "What the test ID broke."]
        detail: String,
    },
    #[doc = "A gate and its evidence disagree, or a test ID and gate disagree."]
    GateMismatch {
        #[doc = "What disagreed."]
        detail: String,
    },
    #[doc = "The schema identifier is not the supported sample schema."]
    UnsupportedSchema {
        #[doc = "The rejected schema value."]
        version: String,
    },
    #[doc = "The encoded sample carries a field the schema does not define."]
    UnknownField {
        #[doc = "The upstream serde report, kept verbatim."]
        detail: String,
    },
    #[doc = "The encoded envelope is not valid JSON for this schema."]
    InvalidFormat {
        #[doc = "The decoding failure."]
        source: serde_json::Error,
    },
    #[doc = "The embedded snapshot fails upstream validation."]
    InvalidSnapshot {
        #[doc = "The upstream validation report, kept verbatim."]
        detail: String,
    },
    #[doc = "An image dimension passes its limit."]
    DimensionTooLarge {
        #[doc = "The rejected width."]
        width: u32,
        #[doc = "The rejected height."]
        height: u32,
        #[doc = "The accepted per-dimension limit."]
        limit: u32,
    },
    #[doc = "Decoded RGBA bytes pass the 256 MiB limit."]
    RgbaTooLarge {
        #[doc = "The rejected byte count."]
        bytes: u64,
        #[doc = "The accepted limit."]
        limit: u64,
    },
    #[doc = "Claimed image dimensions disagree with decoded PNG dimensions."]
    DimensionMismatch {
        #[doc = "What disagreed."]
        detail: String,
    },
    #[doc = "Upstream pixel comparison reports changed pixels without a difference image."]
    MissingDifferenceImage {
        #[doc = "What the comparison reported."]
        detail: String,
    },
    #[doc = "The embedded PNG does not decode as 8-bit RGBA."]
    PngInvalid {
        #[doc = "What decoding found."]
        detail: String,
    },
    #[doc = "PNG decoding failed."]
    PngDecode {
        #[doc = "The decoding failure."]
        source: image::ImageError,
    },
    #[doc = "Embedded base64 does not decode."]
    Base64Invalid {
        #[doc = "The decoding failure."]
        source: base64::DecodeError,
    },
    #[doc = "A recorded digest disagrees with recomputed bytes."]
    DigestMismatch {
        #[doc = "Which digest disagreed (`png_sha256` or `rgba_sha256`)."]
        field: &'static str,
        #[doc = "The recorded value."]
        expected: String,
        #[doc = "The recomputed value."]
        actual: String,
    },
    #[doc = "The upstream renderer refused the snapshot."]
    Render {
        #[doc = "The upstream rejection, with its source chain."]
        source: RenderError,
    },
    #[doc = "The upstream pinned raster policy is not the recorded strict policy."]
    UnexpectedPolicy {
        #[doc = "The policy the render recorded."]
        policy: String,
    },
    #[doc = "The downstream contract version is not the supported one."]
    UnsupportedContract {
        #[doc = "The rejected contract version."]
        version: u32,
    },
    #[doc = "Filesystem failure while reading or writing a sample file."]
    Io {
        #[doc = "The filesystem failure."]
        source: std::io::Error,
    },
}

impl Display for SampleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge { what, bytes, limit } => {
                write!(f, "{what} is {bytes} bytes, above the {limit} byte limit")
            }
            Self::InvalidTestId { detail } => write!(f, "invalid test ID: {detail}"),
            Self::GateMismatch { detail } => write!(f, "gate mismatch: {detail}"),
            Self::UnsupportedSchema { version } => {
                write!(
                    f,
                    "unsupported sample schema {version:?}; expected {SAMPLE_SCHEMA:?}"
                )
            }
            Self::UnknownField { detail } => write!(f, "unknown sample field: {detail}"),
            Self::InvalidFormat { source } => write!(f, "invalid sample encoding: {source}"),
            Self::InvalidSnapshot { detail } => {
                write!(f, "invalid embedded snapshot: {detail}")
            }
            Self::DimensionTooLarge {
                width,
                height,
                limit,
            } => {
                write!(
                    f,
                    "image dimensions {width}x{height} pass the {limit} pixel per-dimension limit"
                )
            }
            Self::RgbaTooLarge { bytes, limit } => {
                write!(
                    f,
                    "decoded RGBA is {bytes} bytes, above the {limit} byte limit"
                )
            }
            Self::DimensionMismatch { detail } => {
                write!(f, "image dimension mismatch: {detail}")
            }
            Self::MissingDifferenceImage { detail } => {
                write!(f, "missing difference image: {detail}")
            }
            Self::PngInvalid { detail } => write!(f, "invalid sample PNG: {detail}"),
            Self::PngDecode { source } => write!(f, "sample PNG does not decode: {source}"),
            Self::Base64Invalid { source } => {
                write!(f, "sample image is not valid base64: {source}")
            }
            Self::DigestMismatch {
                field,
                expected,
                actual,
            } => {
                write!(f, "{field} {expected} disagrees with recomputed {actual}")
            }
            Self::Render { source } => write!(f, "sample render failed: {source}"),
            Self::UnexpectedPolicy { policy } => {
                write!(
                    f,
                    "render recorded raster policy {policy:?}; expected {RASTER_POLICY_PINNED_V1:?}"
                )
            }
            Self::UnsupportedContract { version } => {
                write!(
                    f,
                    "unsupported render contract version {version}; expected {CONTRACT_VERSION}"
                )
            }
            Self::Io { source } => write!(f, "sample file I/O failed: {source}"),
        }
    }
}

impl std::error::Error for SampleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidFormat { source } => Some(source),
            Self::PngDecode { source } => Some(source),
            Self::Base64Invalid { source } => Some(source),
            Self::Render { source } => Some(source),
            Self::Io { source } => Some(source),
            _ => None,
        }
    }
}

impl From<RenderError> for SampleError {
    fn from(source: RenderError) -> Self {
        Self::Render { source }
    }
}

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
#[doc = "The schema identifier as an explicit type: only [`SAMPLE_SCHEMA`] parses."]
pub struct SampleSchema;

impl serde::Serialize for SampleSchema {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(SAMPLE_SCHEMA)
    }
}

impl<'de> serde::Deserialize<'de> for SampleSchema {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        if value == SAMPLE_SCHEMA {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom(SampleError::UnsupportedSchema {
                version: value,
            }))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[doc = "Sample-level observation: evidence metadata that never joins the compared state."]
pub struct Observation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[doc = "The reviewer note, when present."]
    note: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[doc = "One image in a sample: encoded PNG bytes plus digests over the encoding and over the decoded pixels."]
pub struct SampleImage {
    #[doc = "The image format; only PNG exists."]
    format: ImageFormat,
    #[doc = "Decoded image width in pixels."]
    width: u32,
    #[doc = "Decoded image height in pixels."]
    height: u32,
    #[doc = "SHA-256 over the encoded PNG bytes."]
    png_sha256: String,
    #[doc = "SHA-256 over the decoded RGBA pixels (row-major, 8 bits per channel)."]
    rgba_sha256: String,
    #[doc = "The encoded PNG bytes."]
    png_base64: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[doc = "Image formats a sample can carry. Only PNG exists."]
pub enum ImageFormat {
    #[doc = "Portable Network Graphics, 8-bit RGBA."]
    Png,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[doc = "One font entry in a downstream render contract."]
pub struct ContractFont {
    #[doc = "SHA-256 over the style slot, face index, and bytes of the face."]
    digest: String,
    #[doc = "The face index inside its bytes."]
    face_index: u32,
    #[doc = "Whether the face serves the bold style slot."]
    bold: bool,
    #[doc = "Whether the face serves the italic style slot."]
    italic: bool,
    #[doc = "The position of the face in the profile fallback order."]
    fallback_position: usize,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[doc = "The renderer, font, geometry, and color settings that produced an image."]
pub struct RenderContract {
    #[doc = "The logical profile name the image was rendered with."]
    profile: String,
    #[doc = "The downstream contract schema version."]
    contract_version: u32,
    #[doc = "The renderer crate version that produced the image."]
    renderer_version: String,
    #[doc = "The raster policy version: `pinned-v1` for strict renders."]
    raster_policy: String,
    #[doc = "The pinned fonts in fallback order."]
    fonts: Vec<ContractFont>,
    #[doc = "The cell width in pixels."]
    cell_width: f32,
    #[doc = "The cell height in pixels."]
    cell_height: f32,
    #[doc = "The grid font size in pixels."]
    font_size: f32,
    #[doc = "The text baseline from the top of each cell in pixels."]
    baseline: f32,
    #[doc = "The output scale factor."]
    scale: f64,
    #[doc = "Whether a title bar was drawn."]
    title_bar: bool,
    #[doc = "SHA-256 over the resolved palette and default colors."]
    palette_digest: String,
    #[doc = "Whether the cursor was drawn."]
    cursor_phase: CursorPhase,
    #[doc = "The blink intent kept from the snapshot state."]
    blink: Option<bool>,
    #[doc = "The `<os>-<arch>` platform the image was rendered on."]
    platform_key: String,
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

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[doc = "One versioned visual sample: a snapshot bound to its visual evidence."]
pub struct Sample {
    #[doc = "The schema identifier (always [`SAMPLE_SCHEMA`])."]
    schema: SampleSchema,
    #[doc = "The test ID this sample was constructed for."]
    test_id: String,
    #[doc = "The comparison policy this sample carries."]
    gate: Gate,
    #[doc = "The embedded snapshot."]
    snapshot: TerminalSnapshot,
    #[doc = "The render contract, present only for `state-and-pixels`."]
    render_contract: Option<RenderContract>,
    #[doc = "The image evidence, present only for `state-and-pixels`."]
    image: Option<SampleImage>,
    #[doc = "The observation metadata, excluded from the compared state."]
    observation: Observation,
}

impl<'de> serde::Deserialize<'de> for Sample {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let envelope = SampleWire::deserialize(deserializer)?;
        Self::from_wire(envelope).map_err(serde::de::Error::custom)
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SampleWire {
    schema: SampleSchema,
    test_id: String,
    gate: Gate,
    snapshot: serde_json::Value,
    render_contract: Option<RenderContract>,
    image: Option<SampleImage>,
    observation: Observation,
}

impl Sample {
    #[doc = "Build a state sample: serialize the snapshot without rendering.\n\n# Errors\n\nReturns [`SampleError`] when the test ID breaks its shape rules."]
    pub fn state(
        test_id: &str,
        snapshot: TerminalSnapshot,
        observation: Observation,
    ) -> Result<Self, SampleError> {
        validate_test_id(test_id, Gate::State, platform_key().as_str())?;
        Ok(Self {
            schema: SampleSchema,
            test_id: test_id.to_string(),
            gate: Gate::State,
            snapshot,
            render_contract: None,
            image: None,
            observation,
        })
    }

    #[doc = "Build a state-and-pixels sample: render the immutable snapshot through the profile, then validate the image before returning.\n\n# Errors\n\nReturns [`SampleError`] when the test ID breaks its shape rules, when the upstream renderer refuses the snapshot, or when the returned image fails validation."]
    pub fn with_pixels(
        test_id: &str,
        snapshot: &TerminalSnapshot,
        observation: Observation,
        profile: &Profile,
    ) -> Result<Self, SampleError> {
        validate_test_id(test_id, Gate::StateAndPixels, platform_key().as_str())?;
        let rendered = tui_test::render_snapshot_png(snapshot, &profile.render_options())?;
        let contract = RenderContract::from_rendered(profile, &rendered)?;
        let png = rendered.png().to_vec();
        let (width, height, pixels) = decode_png_bounded(&png)?;
        if (width, height) != (rendered.width(), rendered.height()) {
            return Err(SampleError::DimensionMismatch {
                detail: format!(
                    "renderer reported {}x{} but the PNG decodes to {}x{}",
                    rendered.width(),
                    rendered.height(),
                    width,
                    height
                ),
            });
        }
        check_grid_image(snapshot, width, height)?;
        let image = SampleImage {
            format: ImageFormat::Png,
            width,
            height,
            png_sha256: sha256_hex(&png),
            rgba_sha256: sha256_hex(&pixels),
            png_base64: base64::engine::general_purpose::STANDARD.encode(&png),
        };
        let sample = Self {
            schema: SampleSchema,
            test_id: test_id.to_string(),
            gate: Gate::StateAndPixels,
            snapshot: snapshot.clone(),
            render_contract: Some(contract),
            image: Some(image),
            observation,
        };
        sample.validate_structure()?;
        Ok(sample)
    }

    #[doc = "The schema identifier (always [`SAMPLE_SCHEMA`])."]
    #[must_use]
    pub const fn schema(&self) -> SampleSchema {
        self.schema
    }

    #[doc = "The test ID this sample was constructed for."]
    #[must_use]
    pub fn test_id(&self) -> &str {
        &self.test_id
    }

    #[doc = "The comparison policy this sample carries."]
    #[must_use]
    pub const fn gate(&self) -> Gate {
        self.gate
    }

    #[doc = "The embedded snapshot."]
    #[must_use]
    pub fn snapshot(&self) -> &TerminalSnapshot {
        &self.snapshot
    }

    #[doc = "The render contract, present only for `state-and-pixels`."]
    #[must_use]
    pub fn render_contract(&self) -> Option<&RenderContract> {
        self.render_contract.as_ref()
    }

    #[doc = "The image evidence, present only for `state-and-pixels`."]
    #[must_use]
    pub fn image(&self) -> Option<&SampleImage> {
        self.image.as_ref()
    }

    #[doc = "The observation metadata, excluded from the compared state."]
    #[must_use]
    pub const fn observation(&self) -> &Observation {
        &self.observation
    }

    #[doc = "Serialize to canonical file bytes: declaration field order, stable upstream snapshot order, row-major cells.\n\n# Errors\n\nReturns [`SampleError`] when the sample fails validation, when serialization fails, or when the bytes pass the file limit."]
    pub fn to_bytes(&self) -> Result<Vec<u8>, SampleError> {
        self.validate_structure()?;
        self.validate_image_bytes()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|error| SampleError::InvalidFormat { source: error })?;
        check_file_size(&bytes)?;
        Ok(bytes)
    }

    #[doc = "Parse and validate encoded file bytes without font resolution.\n\n# Errors\n\nReturns [`SampleError`] for every rejected input; see its variants."]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SampleError> {
        check_file_size(bytes)?;
        let probe: serde_json::Value = serde_json::from_slice(bytes).map_err(|error| {
            let message = error.to_string();
            if message.starts_with("unknown field") {
                SampleError::UnknownField { detail: message }
            } else {
                SampleError::InvalidFormat { source: error }
            }
        })?;
        match probe.get("schema").and_then(|schema| schema.as_str()) {
            Some(version) if version == SAMPLE_SCHEMA => {}
            Some(version) => {
                return Err(SampleError::UnsupportedSchema {
                    version: version.to_string(),
                });
            }
            None => {
                return Err(SampleError::InvalidFormat {
                    source: serde_json::Error::custom("sample schema is missing"),
                });
            }
        }
        let envelope: SampleWire = serde_json::from_value(probe).map_err(|error| {
            let message = error.to_string();
            if message.starts_with("unknown field") {
                SampleError::UnknownField { detail: message }
            } else {
                SampleError::InvalidFormat { source: error }
            }
        })?;
        Self::from_wire(envelope)
    }

    fn from_wire(envelope: SampleWire) -> Result<Self, SampleError> {
        preflight_snapshot_wire(&envelope.snapshot)?;
        let snapshot: TerminalSnapshot =
            serde_json::from_value(envelope.snapshot).map_err(|error| {
                SampleError::InvalidSnapshot {
                    detail: error.to_string(),
                }
            })?;
        let sample = Self {
            schema: envelope.schema,
            test_id: envelope.test_id,
            gate: envelope.gate,
            snapshot,
            render_contract: envelope.render_contract,
            image: envelope.image,
            observation: envelope.observation,
        };
        sample.validate_structure()?;
        sample.validate_image_bytes()?;
        Ok(sample)
    }

    #[doc = "Read a sample file with a bounded read: bytes past the file limit are never loaded.\n\n# Errors\n\nReturns [`SampleError`] when the file is unreadable, too large, or fails [`Sample::from_bytes`] validation."]
    pub fn from_file(path: &std::path::Path) -> Result<Self, SampleError> {
        Self::from_bytes(&read_bounded(path)?)
    }

    #[doc = "Serialize and write one sample file through one atomic file write: a temporary file in the destination directory, flushed and closed before one atomic rename.\n\n# Errors\n\nReturns [`SampleError`] when serialization fails, when the sample path has no parent directory or file name, or when the write fails. A partial write is a failure, never a success value."]
    pub fn save(&self, path: &std::path::Path) -> Result<(), SampleError> {
        let bytes = self.to_bytes()?;
        let invalid = |what: &str| SampleError::Io {
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("sample path {} has {what}", path.display()),
            ),
        };
        let dir = path
            .parent()
            .ok_or_else(|| invalid("no parent directory"))?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| invalid("no file name"))?;
        atomic_write(dir, name, &bytes).map_err(|source| SampleError::Io { source })?;
        Ok(())
    }

    #[doc = "The candidate ID: SHA-256 over the sample file bytes."]
    #[must_use]
    pub fn candidate_id(bytes: &[u8]) -> String {
        sha256_hex(bytes)
    }

    #[doc = "Digest over the compared state only: gate, snapshot without its observation metadata, render contract, and decoded RGBA pixels.\n\n# Errors\n\nReturns [`SampleError`] when the embedded image fails to decode."]
    pub fn state_digest(&self) -> Result<String, SampleError> {
        let mut hasher = Sha256::new();
        hasher.update(b"tuiscotti-state-v1");
        hasher.update([u8::from(self.gate == Gate::StateAndPixels)]);
        let mut snapshot = serde_json::to_value(&self.snapshot)
            .map_err(|error| SampleError::InvalidFormat { source: error })?;
        if let Some(object) = snapshot.as_object_mut() {
            object.remove("metadata");
        }
        let canonical = serde_json::to_vec(&snapshot)
            .map_err(|error| SampleError::InvalidFormat { source: error })?;
        hasher.update(canonical);
        if let Some(contract) = &self.render_contract {
            let canonical = serde_json::to_vec(contract)
                .map_err(|error| SampleError::InvalidFormat { source: error })?;
            hasher.update(canonical);
        }
        if let Some(image) = &self.image {
            let (_, _, pixels) = Self::decoded_pixels(image)?;
            hasher.update(pixels);
        }
        Ok(hex_bytes(&hasher.finalize()))
    }

    #[doc = "Structural validation without font resolution: gate pairing, test ID, contract version, grid limits, and cell text limits.\n\n# Errors\n\nReturns [`SampleError`] for every structural rejection."]
    pub fn validate_structure(&self) -> Result<(), SampleError> {
        let current_platform;
        let pixels_platform = if let Some(contract) = &self.render_contract {
            contract.platform_key.as_str()
        } else {
            current_platform = platform_key();
            current_platform.as_str()
        };
        validate_test_id(&self.test_id, self.gate, pixels_platform)?;
        match self.gate {
            Gate::State => {
                if self.render_contract.is_some() || self.image.is_some() {
                    return Err(SampleError::GateMismatch {
                        detail: "state gate requires null render contract and image".to_string(),
                    });
                }
            }
            Gate::StateAndPixels => {
                let contract =
                    self.render_contract
                        .as_ref()
                        .ok_or_else(|| SampleError::GateMismatch {
                            detail: "state-and-pixels gate requires a render contract".to_string(),
                        })?;
                let image = self
                    .image
                    .as_ref()
                    .ok_or_else(|| SampleError::GateMismatch {
                        detail: "state-and-pixels gate requires an image".to_string(),
                    })?;
                if contract.contract_version != CONTRACT_VERSION {
                    return Err(SampleError::UnsupportedContract {
                        version: contract.contract_version,
                    });
                }
                if image.format != ImageFormat::Png {
                    return Err(SampleError::PngInvalid {
                        detail: "only PNG images are supported".to_string(),
                    });
                }
            }
        }
        check_grid_limits(
            self.snapshot.cols(),
            self.snapshot.rows(),
            self.snapshot.cells().len(),
        )?;
        for cell in self.snapshot.cells() {
            if cell.text().len() > MAX_CELL_TEXT_BYTES {
                return Err(SampleError::TooLarge {
                    what: "cell text",
                    bytes: cell.text().len() as u64,
                    limit: MAX_CELL_TEXT_BYTES as u64,
                });
            }
        }
        Ok(())
    }

    fn decoded_pixels(image: &SampleImage) -> Result<(u32, u32, Vec<u8>), SampleError> {
        check_image_dims(image.width, image.height)?;
        let raw = base64::engine::general_purpose::STANDARD
            .decode(image.png_base64.as_bytes())
            .map_err(|source| SampleError::Base64Invalid { source })?;
        let (width, height, pixels) = decode_png_bounded(&raw)?;
        if (width, height) != (image.width, image.height) {
            return Err(SampleError::DimensionMismatch {
                detail: format!(
                    "sample claims {}x{} but the PNG decodes to {}x{}",
                    image.width, image.height, width, height
                ),
            });
        }
        Ok((width, height, pixels))
    }

    fn validate_image_bytes(&self) -> Result<(), SampleError> {
        let Some(image) = &self.image else {
            return Ok(());
        };
        let raw = base64::engine::general_purpose::STANDARD
            .decode(image.png_base64.as_bytes())
            .map_err(|source| SampleError::Base64Invalid { source })?;
        let actual_png = sha256_hex(&raw);
        if actual_png != image.png_sha256 {
            return Err(SampleError::DigestMismatch {
                field: "png_sha256",
                expected: image.png_sha256.clone(),
                actual: actual_png,
            });
        }
        let (width, height, pixels) = Self::decoded_pixels(image)?;
        let actual_rgba = sha256_hex(&pixels);
        if actual_rgba != image.rgba_sha256 {
            return Err(SampleError::DigestMismatch {
                field: "rgba_sha256",
                expected: image.rgba_sha256.clone(),
                actual: actual_rgba,
            });
        }
        check_grid_image(&self.snapshot, width, height)?;
        Ok(())
    }
}

fn validate_test_id(test_id: &str, gate: Gate, pixels_platform: &str) -> Result<(), SampleError> {
    validate_test_id_grammar(test_id)?;
    let suffix = format!("/pixels-{pixels_platform}");
    match gate {
        Gate::StateAndPixels => {
            if !test_id.ends_with(suffix.as_str()) {
                return Err(SampleError::GateMismatch {
                    detail: format!(
                        "state-and-pixels test ID {test_id:?} does not end with {suffix:?}"
                    ),
                });
            }
        }
        Gate::State => {
            if test_id
                .rsplit('/')
                .next()
                .is_some_and(|last| last.starts_with("pixels-"))
            {
                return Err(SampleError::GateMismatch {
                    detail: format!(
                        "state test ID {test_id:?} carries a pixels suffix; use the logical test ID"
                    ),
                });
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_test_id_grammar(test_id: &str) -> Result<(), SampleError> {
    let invalid = |detail: &str| SampleError::InvalidTestId {
        detail: detail.to_string(),
    };
    if test_id.is_empty() {
        return Err(invalid("test ID is empty"));
    }
    if test_id.len() > MAX_TEST_ID_BYTES {
        return Err(SampleError::InvalidTestId {
            detail: format!(
                "test ID is {} bytes, above the {MAX_TEST_ID_BYTES} byte limit",
                test_id.len()
            ),
        });
    }
    if !test_id.is_ascii() {
        return Err(invalid("test ID is not ASCII"));
    }
    if test_id.contains('\\') {
        return Err(invalid("test ID contains a backslash"));
    }
    if test_id.starts_with('/') {
        return Err(invalid("test ID is an absolute path"));
    }
    for segment in test_id.split('/') {
        if segment.is_empty() {
            return Err(invalid("test ID has an empty segment"));
        }
        if segment.len() > MAX_TEST_ID_SEGMENT_BYTES {
            return Err(SampleError::InvalidTestId {
                detail: format!(
                    "test ID segment {segment:?} passes the {MAX_TEST_ID_SEGMENT_BYTES} byte limit"
                ),
            });
        }
        if segment == "." || segment == ".." {
            return Err(SampleError::InvalidTestId {
                detail: format!("test ID segment {segment:?} is a directory reference"),
            });
        }
        let shaped = segment.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        });
        if !shaped {
            return Err(SampleError::InvalidTestId {
                detail: format!("test ID segment {segment:?} breaks the [a-z0-9_-] shape"),
            });
        }
        if is_device_name(segment) {
            return Err(SampleError::InvalidTestId {
                detail: format!("test ID segment {segment:?} is a reserved device name"),
            });
        }
    }
    Ok(())
}

fn is_device_name(segment: &str) -> bool {
    let lower = segment.to_ascii_lowercase();
    let stem = match lower.split('.').next() {
        Some(stem) => stem,
        None => lower.as_str(),
    };
    if matches!(stem, "con" | "prn" | "aux" | "nul") {
        return true;
    }
    let bytes = stem.as_bytes();
    if bytes.len() == 4
        && (stem.starts_with("com") || stem.starts_with("lpt"))
        && bytes[3].is_ascii_digit()
        && bytes[3] != b'0'
    {
        return true;
    }
    false
}

fn check_file_size(bytes: &[u8]) -> Result<(), SampleError> {
    let len = bytes.len() as u64;
    if len > MAX_SAMPLE_FILE_BYTES {
        return Err(SampleError::TooLarge {
            what: "sample file",
            bytes: len,
            limit: MAX_SAMPLE_FILE_BYTES,
        });
    }
    Ok(())
}

fn check_grid_limits(cols: u32, rows: u32, cells: usize) -> Result<(), SampleError> {
    if cols == 0 || rows == 0 {
        return Err(SampleError::InvalidSnapshot {
            detail: "snapshot grid is empty".to_string(),
        });
    }
    if cols > MAX_IMAGE_DIM || rows > MAX_IMAGE_DIM {
        return Err(SampleError::DimensionTooLarge {
            width: cols,
            height: rows,
            limit: MAX_IMAGE_DIM,
        });
    }
    if cells > MAX_SAMPLE_CELLS {
        return Err(SampleError::TooLarge {
            what: "sample cells",
            bytes: cells as u64,
            limit: MAX_SAMPLE_CELLS as u64,
        });
    }
    Ok(())
}

fn preflight_snapshot_wire(value: &serde_json::Value) -> Result<(), SampleError> {
    preflight_snapshot_schema(value)?;
    preflight_snapshot_grid(value)?;
    preflight_snapshot_cells(value)?;
    preflight_snapshot_title(value)?;
    Ok(())
}

fn preflight_snapshot_schema(value: &serde_json::Value) -> Result<(), SampleError> {
    match value.get("schema") {
        None => Err(SampleError::UnsupportedSchema {
            version: "0".to_string(),
        }),
        Some(schema) => match schema.as_u64() {
            Some(version) if version == u64::from(SNAPSHOT_SCHEMA_VERSION) => Ok(()),
            Some(version) => Err(SampleError::UnsupportedSchema {
                version: version.to_string(),
            }),
            None => Ok(()),
        },
    }
}

fn preflight_snapshot_grid(value: &serde_json::Value) -> Result<(), SampleError> {
    let (Some(cols), Some(rows)) = (
        value.get("cols").and_then(serde_json::Value::as_u64),
        value.get("rows").and_then(serde_json::Value::as_u64),
    ) else {
        return Ok(());
    };
    if cols > u64::from(MAX_IMAGE_DIM) || rows > u64::from(MAX_IMAGE_DIM) {
        return Err(SampleError::DimensionTooLarge {
            width: u32::try_from(cols).unwrap_or(u32::MAX),
            height: u32::try_from(rows).unwrap_or(u32::MAX),
            limit: MAX_IMAGE_DIM,
        });
    }
    let cells = cols.saturating_mul(rows);
    if cells > MAX_SAMPLE_CELLS as u64 {
        return Err(SampleError::TooLarge {
            what: "sample cells",
            bytes: cells,
            limit: MAX_SAMPLE_CELLS as u64,
        });
    }
    Ok(())
}

fn preflight_snapshot_cells(value: &serde_json::Value) -> Result<(), SampleError> {
    let Some(cells) = value.get("cells").and_then(serde_json::Value::as_array) else {
        return Ok(());
    };
    for cell in cells {
        if let Some(text) = cell.get("text").and_then(serde_json::Value::as_str) {
            let bytes = text.len() as u64;
            if bytes > MAX_CELL_TEXT_BYTES as u64 {
                return Err(SampleError::TooLarge {
                    what: "sample cell text",
                    bytes,
                    limit: MAX_CELL_TEXT_BYTES as u64,
                });
            }
        }
        for key in ["link_uri", "link_id"] {
            if let Some(link) = cell.get(key).and_then(serde_json::Value::as_str) {
                let bytes = link.len() as u64;
                if bytes > MAX_LINK_BYTES as u64 {
                    return Err(SampleError::TooLarge {
                        what: "sample cell link",
                        bytes,
                        limit: MAX_LINK_BYTES as u64,
                    });
                }
            }
        }
    }
    Ok(())
}

fn preflight_snapshot_title(value: &serde_json::Value) -> Result<(), SampleError> {
    if let Some(title) = value.get("title").and_then(serde_json::Value::as_str) {
        let bytes = title.len() as u64;
        if bytes > MAX_TITLE_BYTES as u64 {
            return Err(SampleError::TooLarge {
                what: "sample title",
                bytes,
                limit: MAX_TITLE_BYTES as u64,
            });
        }
    }
    Ok(())
}

fn check_image_dims(width: u32, height: u32) -> Result<(), SampleError> {
    if width == 0 || height == 0 {
        return Err(SampleError::DimensionMismatch {
            detail: "image dimensions are zero".to_string(),
        });
    }
    if width > MAX_IMAGE_DIM || height > MAX_IMAGE_DIM {
        return Err(SampleError::DimensionTooLarge {
            width,
            height,
            limit: MAX_IMAGE_DIM,
        });
    }
    let bytes = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(SampleError::RgbaTooLarge {
            bytes: u64::MAX,
            limit: MAX_RGBA_BYTES,
        })?;
    if bytes > MAX_RGBA_BYTES {
        return Err(SampleError::RgbaTooLarge {
            bytes,
            limit: MAX_RGBA_BYTES,
        });
    }
    Ok(())
}

fn check_grid_image(
    snapshot: &TerminalSnapshot,
    width: u32,
    height: u32,
) -> Result<(), SampleError> {
    use crate::render_profile::{CELL_HEIGHT_U32, CELL_WIDTH_U32};
    let expected_width = snapshot.cols().checked_mul(CELL_WIDTH_U32).ok_or_else(|| {
        SampleError::DimensionMismatch {
            detail: "grid width overflows".to_string(),
        }
    })?;
    let expected_height = snapshot
        .rows()
        .checked_mul(CELL_HEIGHT_U32)
        .ok_or_else(|| SampleError::DimensionMismatch {
            detail: "grid height overflows".to_string(),
        })?;
    if (width, height) != (expected_width, expected_height) {
        return Err(SampleError::DimensionMismatch {
            detail: format!(
                "snapshot grid {}x{} needs {expected_width}x{expected_height} pixels, image is {width}x{height}",
                snapshot.cols(),
                snapshot.rows()
            ),
        });
    }
    Ok(())
}

fn decode_png_bounded(png: &[u8]) -> Result<(u32, u32, Vec<u8>), SampleError> {
    let decoder = image::codecs::png::PngDecoder::new(std::io::Cursor::new(png))
        .map_err(|source| SampleError::PngDecode { source })?;
    if decoder.color_type() != image::ColorType::Rgba8 {
        return Err(SampleError::PngInvalid {
            detail: format!(
                "PNG color type is {:?}; only 8-bit RGBA is supported",
                decoder.color_type()
            ),
        });
    }
    let (width, height) = decoder.dimensions();
    check_image_dims(width, height)?;
    let total = usize::try_from(
        u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(SampleError::RgbaTooLarge {
                bytes: u64::MAX,
                limit: MAX_RGBA_BYTES,
            })?,
    )
    .map_err(|_| SampleError::RgbaTooLarge {
        bytes: u64::MAX,
        limit: MAX_RGBA_BYTES,
    })?;
    let mut pixels = vec![0u8; total];
    decoder
        .read_image(&mut pixels)
        .map_err(|source| SampleError::PngDecode { source })?;
    Ok((width, height, pixels))
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex_bytes(&Sha256::digest(bytes))
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 15) as usize] as char);
    }
    out
}
