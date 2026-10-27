//! Versioned visual samples binding a snapshot to its evidence (D02, GOAL 13.1/13.3).
//!
//! One sample is one UTF-8 JSON file with the schema [`SAMPLE_SCHEMA`]
//! (`tuiscotti.sample.v1`) and this field structure, in this order:
//!
//! ```text
//! schema
//! test_id
//! gate
//! snapshot            # public upstream serialization, row-major cells
//! render_contract     # null for a state sample without an image
//! image               # null for a state sample without an image
//!   format            # "png"
//!   width
//!   height
//!   png_sha256
//!   rgba_sha256
//!   png_base64
//! observation
//! ```
//!
//! The `snapshot` field uses the public upstream [`TerminalSnapshot`]
//! serialization unchanged. Types are explicit everywhere: no unstructured
//! maps. Unknown fields are rejected, as are unsupported schemas.
//!
//! Comparison rules (comparison itself is D03; this module supplies the
//! primitives): the compared state is the gate, the snapshot without its
//! observation metadata, the render contract, and the decoded RGBA pixels.
//! Test IDs, file encodings, and sample-level observation are addressing
//! and evidence metadata: they change the candidate ID but never the
//! compared state. The candidate ID is the SHA-256 of the sample file bytes.
//!
//! Resource limits (GOAL 8.4) apply before allocation or rendering:
//! encoded sample file 64 MiB (bounded reads), one sample 1,000,000 cells,
//! one cell string 4,096 UTF-8 bytes, decoded RGBA 256 MiB with decoded
//! image dimension validation before allocation, one image dimension 32,768
//! pixels, test ID 240 ASCII bytes with 64 ASCII bytes per segment.

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

/// Sample schema identifier. Only this value parses.
pub const SAMPLE_SCHEMA: &str = "tuiscotti.sample.v1";

/// Schema version of the downstream render contract in samples.
pub const CONTRACT_VERSION: u32 = 1;

/// Encoded sample file limit: 64 MiB, enforced with bounded reads.
pub const MAX_SAMPLE_FILE_BYTES: u64 = 64 * 1024 * 1024;
/// Cells in one sample.
pub const MAX_SAMPLE_CELLS: usize = 1_000_000;
/// UTF-8 bytes in one cell string.
pub const MAX_CELL_TEXT_BYTES: usize = 4_096;
/// Decoded RGBA byte limit: 256 MiB, validated before allocation.
pub const MAX_RGBA_BYTES: u64 = 256 * 1024 * 1024;
/// One image dimension limit: 32,768 pixels.
pub const MAX_IMAGE_DIM: u32 = 32_768;
/// Test ID length limit: 240 ASCII bytes.
pub const MAX_TEST_ID_BYTES: usize = 240;
/// One test ID segment limit: 64 ASCII bytes.
pub const MAX_TEST_ID_SEGMENT_BYTES: usize = 64;

/// Read one evidence file through a shared bounded read: bytes past
/// [`MAX_SAMPLE_FILE_BYTES`] fail as a resource limit instead of loading,
/// so no evidence read can exhaust memory before validation runs.
/// A missing file maps to `Ok(None)` so callers keep the
/// missing-versus-unreadable distinction; every other I/O failure is `Io`.
///
/// # Errors
///
/// Returns [`SampleError::TooLarge`] when the file passes the limit and
/// [`SampleError::Io`] when it cannot be read.
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

/// Read one evidence file that must exist through the shared bounded read.
///
/// # Errors
///
/// Returns [`SampleError::Io`] for a missing or unreadable file and
/// [`SampleError::TooLarge`] when the file passes the limit.
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

/// Every failure a sample operation can report, with sources retained.
#[derive(Debug)]
#[non_exhaustive]
pub enum SampleError {
    /// The encoded file passes the 64 MiB limit.
    TooLarge {
        /// What was measured.
        what: &'static str,
        /// The rejected byte count.
        bytes: u64,
        /// The accepted limit.
        limit: u64,
    },
    /// A test ID breaks its shape rules.
    InvalidTestId {
        /// What the test ID broke.
        detail: String,
    },
    /// A gate and its evidence disagree, or a test ID and gate disagree.
    GateMismatch {
        /// What disagreed.
        detail: String,
    },
    /// The schema identifier is not the supported sample schema.
    UnsupportedSchema {
        /// The rejected schema value.
        version: String,
    },
    /// The encoded sample carries a field the schema does not define.
    UnknownField {
        /// The upstream serde report, kept verbatim.
        detail: String,
    },
    /// The encoded envelope is not valid JSON for this schema.
    InvalidFormat {
        /// The decoding failure.
        source: serde_json::Error,
    },
    /// The embedded snapshot fails upstream validation.
    InvalidSnapshot {
        /// The upstream validation report, kept verbatim.
        detail: String,
    },
    /// An image dimension passes its limit.
    DimensionTooLarge {
        /// The rejected width.
        width: u32,
        /// The rejected height.
        height: u32,
        /// The accepted per-dimension limit.
        limit: u32,
    },
    /// Decoded RGBA bytes pass the 256 MiB limit.
    RgbaTooLarge {
        /// The rejected byte count.
        bytes: u64,
        /// The accepted limit.
        limit: u64,
    },
    /// Claimed image dimensions disagree with decoded PNG dimensions.
    DimensionMismatch {
        /// What disagreed.
        detail: String,
    },
    /// Upstream pixel comparison reports changed pixels without a
    /// difference image.
    MissingDifferenceImage {
        /// What the comparison reported.
        detail: String,
    },
    /// The embedded PNG does not decode as 8-bit RGBA.
    PngInvalid {
        /// What decoding found.
        detail: String,
    },
    /// PNG decoding failed.
    PngDecode {
        /// The decoding failure.
        source: image::ImageError,
    },
    /// Embedded base64 does not decode.
    Base64Invalid {
        /// The decoding failure.
        source: base64::DecodeError,
    },
    /// A recorded digest disagrees with recomputed bytes.
    DigestMismatch {
        /// Which digest disagreed (`png_sha256` or `rgba_sha256`).
        field: &'static str,
        /// The recorded value.
        expected: String,
        /// The recomputed value.
        actual: String,
    },
    /// The upstream renderer refused the snapshot.
    Render {
        /// The upstream rejection, with its source chain.
        source: RenderError,
    },
    /// The upstream pinned raster policy is not the recorded strict policy.
    UnexpectedPolicy {
        /// The policy the render recorded.
        policy: String,
    },
    /// The downstream contract version is not the supported one.
    UnsupportedContract {
        /// The rejected contract version.
        version: u32,
    },
    /// Filesystem failure while reading or writing a sample file.
    Io {
        /// The filesystem failure.
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

/// The comparison policy a sample carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Gate {
    /// Compare the full canonical state only.
    State,
    /// Compare canonical state plus render contract and decoded RGBA pixels.
    StateAndPixels,
}

impl Gate {
    /// The wire value: `state` or `state-and-pixels`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::State => "state",
            Self::StateAndPixels => "state-and-pixels",
        }
    }
}

/// The schema identifier as an explicit type: only [`SAMPLE_SCHEMA`] parses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Sample-level observation: evidence metadata that never joins the compared
/// state. Different observation gives a different candidate ID with equal
/// visual state.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

impl Observation {
    /// Empty observation.
    #[must_use]
    pub const fn empty() -> Self {
        Self { note: None }
    }

    /// Observation carrying a reviewer note.
    #[must_use]
    pub fn noted(note: impl Into<String>) -> Self {
        Self {
            note: Some(note.into()),
        }
    }

    /// The reviewer note, when present.
    #[must_use]
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

/// One image in a sample: encoded PNG bytes plus digests over the encoding
/// and over the decoded pixels.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SampleImage {
    format: ImageFormat,
    width: u32,
    height: u32,
    png_sha256: String,
    rgba_sha256: String,
    png_base64: String,
}

impl SampleImage {
    /// The image format; only PNG exists.
    #[must_use]
    pub const fn format(&self) -> ImageFormat {
        self.format
    }

    /// Decoded image width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Decoded image height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// SHA-256 over the encoded PNG bytes.
    #[must_use]
    pub fn png_sha256(&self) -> &str {
        &self.png_sha256
    }

    /// SHA-256 over the decoded RGBA pixels (row-major, 8 bits per channel).
    #[must_use]
    pub fn rgba_sha256(&self) -> &str {
        &self.rgba_sha256
    }

    /// The encoded PNG bytes.
    #[must_use]
    pub fn png_base64(&self) -> &str {
        &self.png_base64
    }
}

/// Image formats a sample can carry. Only PNG exists: no pixel-only or
/// masked variants exist in this goal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    /// Portable Network Graphics, 8-bit RGBA.
    Png,
}

/// One font entry in a downstream render contract.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractFont {
    digest: String,
    face_index: u32,
    bold: bool,
    italic: bool,
    fallback_position: usize,
}

impl ContractFont {
    /// SHA-256 over the style slot, face index, and bytes of the face.
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// The face index inside its bytes.
    #[must_use]
    pub const fn face_index(&self) -> u32 {
        self.face_index
    }

    /// Whether the face serves the bold style slot.
    #[must_use]
    pub const fn bold(&self) -> bool {
        self.bold
    }

    /// Whether the face serves the italic style slot.
    #[must_use]
    pub const fn italic(&self) -> bool {
        self.italic
    }

    /// The position of the face in the profile fallback order.
    #[must_use]
    pub const fn fallback_position(&self) -> usize {
        self.fallback_position
    }
}

/// Cursor visibility recorded in a render contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CursorPhase {
    /// The cursor was drawn: the snapshot does not hide it.
    Shown,
    /// The cursor was hidden or clipped out of the capture.
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

/// The renderer, font, geometry, and color settings that produced an image.
///
/// Structural validation (no font resolution) accepts a historical contract
/// that parses under this schema; rendering and approval re-rendering
/// resolve the profile by name instead. Style slots, fallback order, font
/// digests with face indices, geometry, palette digest, cursor phase, and
/// the platform key are all recorded here.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderContract {
    profile: String,
    contract_version: u32,
    renderer_version: String,
    raster_policy: String,
    fonts: Vec<ContractFont>,
    cell_width: f32,
    cell_height: f32,
    font_size: f32,
    baseline: f32,
    scale: f64,
    title_bar: bool,
    palette_digest: String,
    cursor_phase: CursorPhase,
    blink: Option<bool>,
    platform_key: String,
}

impl RenderContract {
    /// Build the recorded render contract from a resolved profile and a
    /// fresh upstream render: the single typed constructor for recorded
    /// render identity.
    ///
    /// [`Sample::with_pixels`] records through it, and approval re-rendering
    /// rebuilds the expected identity through it, so creation and
    /// verification cannot disagree on profile, contract version, renderer
    /// version, raster policy, fonts, geometry, scale, title bar, palette
    /// digest, cursor phase, blink, or platform.
    ///
    /// The pinned-profile gate lives here deliberately, not beside the
    /// approval field comparison: the comparison checks recorded identity
    /// against re-rendered identity, both derived from the recorded profile,
    /// so a self-consistent non-pinned identity would pass it. Only this
    /// constructor (and the single-profile registry behind
    /// `resolve_profile`) pins creation and verification to
    /// [`PROFILE_NAME`].
    ///
    /// # Errors
    ///
    /// Returns [`SampleError`] when the profile is not the pinned profile
    /// or the upstream render records a foreign raster policy.
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

    /// The logical profile name the image was rendered with.
    #[must_use]
    pub fn profile(&self) -> &str {
        &self.profile
    }

    /// The downstream contract schema version.
    #[must_use]
    pub const fn contract_version(&self) -> u32 {
        self.contract_version
    }

    /// The renderer crate version that produced the image.
    #[must_use]
    pub fn renderer_version(&self) -> &str {
        &self.renderer_version
    }

    /// The raster policy version: `pinned-v1` for strict renders.
    #[must_use]
    pub fn raster_policy(&self) -> &str {
        &self.raster_policy
    }

    /// The pinned fonts in fallback order.
    #[must_use]
    pub fn fonts(&self) -> &[ContractFont] {
        &self.fonts
    }

    /// The cell width in pixels.
    #[must_use]
    pub const fn cell_width(&self) -> f32 {
        self.cell_width
    }

    /// The cell height in pixels.
    #[must_use]
    pub const fn cell_height(&self) -> f32 {
        self.cell_height
    }

    /// The grid font size in pixels.
    #[must_use]
    pub const fn font_size(&self) -> f32 {
        self.font_size
    }

    /// The text baseline from the top of each cell in pixels.
    #[must_use]
    pub const fn baseline(&self) -> f32 {
        self.baseline
    }

    /// The output scale factor.
    #[must_use]
    pub const fn scale(&self) -> f64 {
        self.scale
    }

    /// Whether a title bar was drawn.
    #[must_use]
    pub const fn title_bar(&self) -> bool {
        self.title_bar
    }

    /// SHA-256 over the resolved palette and default colors.
    #[must_use]
    pub fn palette_digest(&self) -> &str {
        &self.palette_digest
    }

    /// Whether the cursor was drawn.
    #[must_use]
    pub const fn cursor_phase(&self) -> CursorPhase {
        self.cursor_phase
    }

    /// The blink intent kept from the snapshot state.
    #[must_use]
    pub const fn blink(&self) -> Option<bool> {
        self.blink
    }

    /// The `<os>-<arch>` platform the image was rendered on.
    #[must_use]
    pub fn platform_key(&self) -> &str {
        &self.platform_key
    }
}

/// One versioned visual sample: a snapshot bound to its visual evidence.
///
/// Fields stay private; constructors and accessors are the whole surface.
/// Field order in the file is the declaration order below and never changes.
///
/// There is no derived `Deserialize`: the manual implementation below runs
/// the same complete validation as [`Sample::from_bytes`], so the serde
/// path can never bypass gate pairing or digest checks.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Sample {
    schema: SampleSchema,
    test_id: String,
    gate: Gate,
    snapshot: TerminalSnapshot,
    render_contract: Option<RenderContract>,
    image: Option<SampleImage>,
    observation: Observation,
}

impl<'de> serde::Deserialize<'de> for Sample {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let envelope = SampleWire::deserialize(deserializer)?;
        Self::from_wire(envelope).map_err(serde::de::Error::custom)
    }
}

/// Private wire mirror with a deferred snapshot: lets classification tell
/// an envelope failure (unknown field, bad gate) apart from a snapshot
/// validation failure with one parse. The only way to a [`Sample`].
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
    /// Build a state sample: serialize the snapshot without rendering.
    ///
    /// # Errors
    ///
    /// Returns [`SampleError`] when the test ID breaks its shape rules.
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

    /// Build a state-and-pixels sample: render the immutable snapshot
    /// through the profile, then validate the image before returning.
    ///
    /// The snapshot is never re-captured during rendering: the caller-held
    /// snapshot is the single input to both serialization and rendering.
    ///
    /// # Errors
    ///
    /// Returns [`SampleError`] when the test ID breaks its shape rules,
    /// when the upstream renderer refuses the snapshot, or when the
    /// returned image fails validation.
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

    /// The schema identifier (always [`SAMPLE_SCHEMA`]).
    #[must_use]
    pub const fn schema(&self) -> SampleSchema {
        self.schema
    }

    /// The test ID this sample was constructed for.
    #[must_use]
    pub fn test_id(&self) -> &str {
        &self.test_id
    }

    /// The comparison policy this sample carries.
    #[must_use]
    pub const fn gate(&self) -> Gate {
        self.gate
    }

    /// The embedded snapshot.
    #[must_use]
    pub fn snapshot(&self) -> &TerminalSnapshot {
        &self.snapshot
    }

    /// The render contract, present only for `state-and-pixels`.
    #[must_use]
    pub fn render_contract(&self) -> Option<&RenderContract> {
        self.render_contract.as_ref()
    }

    /// The image evidence, present only for `state-and-pixels`.
    #[must_use]
    pub fn image(&self) -> Option<&SampleImage> {
        self.image.as_ref()
    }

    /// The observation metadata, excluded from the compared state.
    #[must_use]
    pub const fn observation(&self) -> &Observation {
        &self.observation
    }

    /// Serialize to canonical file bytes: declaration field order, stable
    /// upstream snapshot order, row-major cells. Validates before
    /// serializing, then enforces the file limit, so only valid samples
    /// ever reach the wire.
    ///
    /// # Errors
    ///
    /// Returns [`SampleError`] when the sample fails validation, when
    /// serialization fails, or when the bytes pass the file limit.
    pub fn to_bytes(&self) -> Result<Vec<u8>, SampleError> {
        self.validate_structure()?;
        self.validate_image_bytes()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|error| SampleError::InvalidFormat { source: error })?;
        check_file_size(&bytes)?;
        Ok(bytes)
    }

    /// Parse and validate encoded file bytes without font resolution.
    ///
    /// Structural validation only: test ID, gate pairing, contract version,
    /// image dimensions (before allocation), PNG decoding, and both
    /// digests. Historical contracts pass without resolving their fonts.
    ///
    /// # Errors
    ///
    /// Returns [`SampleError`] for every rejected input; see its variants.
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

    /// Build a validated sample from one parsed wire envelope: the single
    /// construction path shared by [`Sample::from_bytes`] and the manual
    /// [`serde::Deserialize`] implementation, so no parse path can bypass
    /// structural or image validation.
    ///
    /// # Errors
    ///
    /// Returns [`SampleError`] for every rejected input; see its variants.
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

    /// Read a sample file with a bounded read: bytes past the file limit
    /// are never loaded.
    ///
    /// # Errors
    ///
    /// Returns [`SampleError`] when the file is unreadable, too large, or
    /// fails [`Sample::from_bytes`] validation.
    pub fn from_file(path: &std::path::Path) -> Result<Self, SampleError> {
        Self::from_bytes(&read_bounded(path)?)
    }

    /// Serialize and write one sample file through one atomic file write: a
    /// temporary file in the destination directory, flushed and closed
    /// before one atomic rename, like all sibling evidence. Readers never
    /// see a partial file.
    ///
    /// # Errors
    ///
    /// Returns [`SampleError`] when serialization fails, when the sample
    /// path has no parent directory or file name, or when the write fails.
    /// A partial write is a failure, never a success value.
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

    /// The candidate ID: SHA-256 over the sample file bytes.
    ///
    /// Different observation metadata gives a different candidate ID with
    /// equal compared state; the ID is never the visual equality rule.
    #[must_use]
    pub fn candidate_id(bytes: &[u8]) -> String {
        sha256_hex(bytes)
    }

    /// Digest over the compared state only: gate, snapshot without its
    /// observation metadata, render contract, and decoded RGBA pixels.
    /// Test ID, file encoding, and sample observation are excluded.
    ///
    /// # Errors
    ///
    /// Returns [`SampleError`] when the embedded image fails to decode.
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

    /// Structural validation without font resolution: gate pairing, test
    /// ID, contract version, grid limits, and cell text limits. Used both
    /// after construction and at comparison time for historical contracts.
    /// The pixels suffix checks against the stored contract identity, so a
    /// sample recorded on another platform parses wherever its own ID and
    /// contract agree; local capture still generates the current-platform
    /// ID through [`Sample::with_pixels`].
    ///
    /// # Errors
    ///
    /// Returns [`SampleError`] for every structural rejection.
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

    /// Decode the embedded image with pre-allocation dimension validation.
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

    /// Digest and dimension agreement between the image record and its bytes.
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

/// Validate a test ID for its gate: the Section 14.1 grammar plus the
/// pixels suffix pairing against the expected platform identity. Callers
/// capturing locally pass the current platform; saved-sample parsing passes
/// the stored contract identity, so historical samples keep parsing off the
/// platform that recorded them.
///
/// This is the single complete test-ID validator: construction
/// ([`Sample::state`], [`Sample::with_pixels`]) and parsing
/// ([`Sample::from_bytes`] through [`Sample::validate_structure`]) all run
/// through it, and the frozen checker delegates its grammar-only check to
/// [`validate_test_id_grammar`], so no second shape rule can drift.
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

/// Validate a test ID against the Section 14.1 grammar on every platform:
///
/// ```text
/// segment = [a-z0-9_-]{1,64}
/// test_id = segment ("/" segment)*
/// ```
///
/// Total length follows Section 8.4 (240 ASCII bytes). Absolute paths,
/// empty segments, backslashes, and `.`/`..` segments are rejected without
/// normalization. Windows device names (`con`, `prn`, `aux`, `nul`,
/// `com1`-`com9`, `lpt1`-`lpt9`, case-insensitive, with or without an
/// extension) are rejected as segments on all platforms.
///
/// # Errors
///
/// Returns [`SampleError::InvalidTestId`] for every rejected shape.
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

/// True for Windows device names on any platform: `con`, `prn`, `aux`,
/// `nul`, `com1`-`com9`, `lpt1`-`lpt9`, case-insensitive, with or without a
/// dotted extension. The caller guarantees ASCII input.
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

/// Enforce the encoded file limit on a loaded byte string.
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

/// Enforce grid and cell-count limits on numbers, before any allocation.
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

/// Preflight the nested snapshot wire value with typed schema and limit
/// checks before [`TerminalSnapshot`] deserialization.
///
/// Upstream snapshot failures arrive as one untyped serde string, which
/// would flatten an unsupported schema or an over-limit grid into
/// [`SampleError::InvalidSnapshot`]. The wire numbers are checked here
/// instead — never by matching upstream message text — so those shapes
/// keep [`SampleError::UnsupportedSchema`] and the resource-limit
/// variants (and their `UNSUPPORTED_SCHEMA`/`RESOURCE_LIMIT` codes).
/// Anything this preflight does not recognize stays for
/// [`TerminalSnapshot`] deserialization, which still fails closed with
/// [`SampleError::InvalidSnapshot`].
fn preflight_snapshot_wire(value: &serde_json::Value) -> Result<(), SampleError> {
    preflight_snapshot_schema(value)?;
    preflight_snapshot_grid(value)?;
    preflight_snapshot_cells(value)?;
    preflight_snapshot_title(value)?;
    Ok(())
}

/// Reject a nested snapshot wire schema other than the supported one.
///
/// A missing schema mirrors the upstream wire default (`0`), which the
/// upstream build rejects; a present non-numeric schema stays for
/// deserialization instead of inventing a version spelling.
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

/// Reject nested snapshot dimensions and cell counts past the limits.
///
/// Bounds come from the downstream limits, which match the upstream
/// snapshot bounds one for one.
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

/// Reject nested snapshot cell text and links past the byte limits.
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

/// Reject a nested snapshot window title past the byte limit.
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

/// Enforce image dimension limits on numbers, before any allocation.
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

/// Check the decoded image against the snapshot grid with checked
/// arithmetic: zero chrome means exactly `cols * cell` by `rows * cell`.
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

/// Decode 8-bit RGBA PNG bytes with dimension validation before the pixel
/// buffer is allocated. The caller checks header dimensions first through
/// [`check_image_dims`]; this decoder reads the IHDR again from untrusted
/// bytes and refuses anything that changed or is not RGBA8.
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

/// SHA-256 hex over raw bytes.
fn sha256_hex(bytes: &[u8]) -> String {
    hex_bytes(&Sha256::digest(bytes))
}

/// Lowercase hex over digest bytes.
fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 15) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{SampleError, preflight_snapshot_wire};
    use crate::approve::sample_code;

    /// One nested wire mutation plus its expected Section 16.3 code, if any.
    type WireCase = (
        &'static str,
        Box<dyn Fn(&mut serde_json::Value)>,
        Option<&'static str>,
    );

    /// A minimal nested snapshot wire value exercising the preflight.
    fn wire() -> serde_json::Value {
        serde_json::json!({
            "schema": 1,
            "cols": 4,
            "rows": 2,
            "cells": [{"text": "a"}, {"text": "b"}],
            "title": null,
        })
    }

    /// Nested schema and limit shapes keep their typed codes without any
    /// message-text classification.
    #[test]
    fn snapshot_wire_preflight_keeps_typed_codes() {
        let cases: Vec<WireCase> = vec![
            ("supported schema", Box::new(|_| {}), None),
            (
                "future schema",
                Box::new(|wire| wire["schema"] = serde_json::json!(999)),
                Some("UNSUPPORTED_SCHEMA"),
            ),
            (
                "missing schema",
                Box::new(|wire| {
                    wire.as_object_mut()
                        .expect("wire is an object")
                        .remove("schema");
                }),
                Some("UNSUPPORTED_SCHEMA"),
            ),
            (
                "oversize dimensions",
                Box::new(|wire| {
                    wire["cols"] = serde_json::json!(40_000);
                    wire["rows"] = serde_json::json!(1);
                    wire["cells"] = serde_json::json!([]);
                }),
                Some("RESOURCE_LIMIT"),
            ),
            (
                "oversize cell count",
                Box::new(|wire| {
                    wire["cols"] = serde_json::json!(2_000);
                    wire["rows"] = serde_json::json!(2_000);
                    wire["cells"] = serde_json::json!([]);
                }),
                Some("RESOURCE_LIMIT"),
            ),
            (
                "oversize cell text",
                Box::new(|wire| wire["cells"][0]["text"] = serde_json::json!("x".repeat(5_000))),
                Some("RESOURCE_LIMIT"),
            ),
            (
                "oversize cell link",
                Box::new(|wire| {
                    wire["cells"][0]["link_uri"] = serde_json::json!("y".repeat(5_000));
                }),
                Some("RESOURCE_LIMIT"),
            ),
            (
                "oversize title",
                Box::new(|wire| wire["title"] = serde_json::json!("t".repeat(70_000))),
                Some("RESOURCE_LIMIT"),
            ),
        ];
        for (name, mutate, expected) in cases {
            let mut value = wire();
            mutate(&mut value);
            match (preflight_snapshot_wire(&value), expected) {
                (Ok(()), None) => {}
                (Err(error), Some(code)) => {
                    assert_eq!(sample_code(&error), code, "{name} keeps {code}");
                    assert!(
                        !matches!(error, SampleError::InvalidSnapshot { .. }),
                        "{name} never flattens to InvalidSnapshot"
                    );
                }
                (outcome, _) => panic!("{name} misclassified: {outcome:?}"),
            }
        }
    }

    /// A non-numeric schema stays for deserialization, which fails closed
    /// on its own; the preflight never invents a version spelling.
    #[test]
    fn snapshot_wire_preflight_ignores_unshaped_fields() {
        let mut value = wire();
        value["schema"] = serde_json::json!("v1");
        assert!(
            preflight_snapshot_wire(&value).is_ok(),
            "unshaped schema stays for deserialization"
        );
        let mut value = wire();
        value
            .as_object_mut()
            .expect("wire is an object")
            .remove("cols");
        assert!(
            preflight_snapshot_wire(&value).is_ok(),
            "missing dims stay for deserialization"
        );
    }
}
