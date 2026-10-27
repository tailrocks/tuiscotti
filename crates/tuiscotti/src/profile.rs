#[doc = include_str!("../docs/profile.md")]
use std::fmt::{Display, Formatter};
use std::sync::OnceLock;

use sha2::{Digest, Sha256};
use tui_test::render::style::Padding;
use tui_test::{
    PinnedFace, PinnedFontError, PinnedFontSet, RASTER_POLICY_PINNED_V1, SnapshotRenderOptions,
};

#[doc = "The single logical profile name this registry resolves."]
pub const PROFILE_NAME: &str = "tuiscotti-pinned-v1";

#[doc = "Grid font size in pixels."]
pub const FONT_SIZE_PX: f32 = 16.0;
#[doc = "Terminal cell width in pixels."]
pub const CELL_WIDTH_PX: f32 = 10.0;
#[doc = "Terminal cell height in pixels."]
pub const CELL_HEIGHT_PX: f32 = 20.0;
#[doc = "Text baseline measured from the top of each cell, in pixels."]
pub const BASELINE_PX: f32 = 15.0;
#[doc = "Output scale factor."]
pub const SCALE: f64 = 1.0;
#[doc = "Terminal cell width in whole pixels, the integer twin of `CELL_WIDTH_PX`."]
pub const CELL_WIDTH_U32: u32 = 10;
#[doc = "Terminal cell height in whole pixels, the integer twin of `CELL_HEIGHT_PX`."]
pub const CELL_HEIGHT_U32: u32 = 20;

const FACE_INDEX: u32 = 0;

const JB_REGULAR_BYTES: &[u8] = include_bytes!("../fonts/JetBrainsMonoNerdFontMono-Regular.ttf");
const JB_BOLD_BYTES: &[u8] = include_bytes!("../fonts/JetBrainsMonoNerdFontMono-Bold.ttf");
const JB_ITALIC_BYTES: &[u8] = include_bytes!("../fonts/JetBrainsMonoNerdFontMono-Italic.ttf");
const JB_BOLD_ITALIC_BYTES: &[u8] =
    include_bytes!("../fonts/JetBrainsMonoNerdFontMono-BoldItalic.ttf");
const SYMBOL_BYTES: &[u8] = include_bytes!("../fonts/DejaVuSansMNerdFontMono-Regular.ttf");
const CJK_BYTES: &[u8] = include_bytes!("../fonts/NotoSansCJKjp-subset.otf");

const JB_REGULAR_LEN: usize = 2_573_248;
const JB_BOLD_LEN: usize = 2_577_016;
const JB_ITALIC_LEN: usize = 2_575_368;
const JB_BOLD_ITALIC_LEN: usize = 2_578_280;
const SYMBOL_LEN: usize = 2_814_292;
const CJK_LEN: usize = 678_988;

const JB_REGULAR_SHA256: &str = "f2a5ea6cfab397445ffab00c0370927b66d61e560a05db5db271b42006381c1a";
const JB_BOLD_SHA256: &str = "bfcf9a917276ffc058867d87cbc8a5b2f1ab0f4b710e9170dc02763ccb80bd4b";
const JB_ITALIC_SHA256: &str = "31efd6ead98746f5b0afa1ee6dba60267ad48db36428360bee327bec10621f97";
const JB_BOLD_ITALIC_SHA256: &str =
    "9dba502e00e35209f6ed2a151c7376c051657b067cdebbc6e52d06cb9002cf31";
const SYMBOL_SHA256: &str = "9b55ade625f2d3f2a273ed16d9db2d924ad6236222920f9fd1324941cdc3c712";
const CJK_SHA256: &str = "777bee41f0c6076c00ad919384359a6e396b8822cf9056041fca8fcf2759d897";

#[doc = "Every failure a profile lookup or build can report, with sources retained."]
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ProfileError {
    #[doc = "The requested profile name is not in the built-in registry."]
    UnknownProfile {
        #[doc = "The rejected name."]
        requested: String,
    },
    #[doc = "Embedded font bytes do not match their pinned length or digest."]
    AssetMismatch {
        #[doc = "Which asset failed its pin check."]
        face: &'static str,
        #[doc = "What the pin check found."]
        detail: String,
    },
    #[doc = "The upstream validated constructor rejected committed font bytes."]
    FontBuild {
        #[doc = "Which face failed validation."]
        face: &'static str,
        #[doc = "The upstream rejection."]
        source: PinnedFontError,
    },
}

impl Display for ProfileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownProfile { requested } => {
                write!(
                    f,
                    "unknown render profile {requested:?}; available: [{PROFILE_NAME}]"
                )
            }
            Self::AssetMismatch { face, detail } => {
                write!(f, "pinned font asset {face} failed its pin check: {detail}")
            }
            Self::FontBuild { face, source } => {
                write!(f, "pinned font face {face} is not a valid font: {source}")
            }
        }
    }
}

impl std::error::Error for ProfileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UnknownProfile { .. } | Self::AssetMismatch { .. } => None,
            Self::FontBuild { source, .. } => Some(source),
        }
    }
}

#[doc = "One face in the built-in registry, in fallback order."]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileFace {
    #[doc = "What this face serves (for example \"primary-bold\")."]
    role: &'static str,
    #[doc = "SHA-256 over the face bytes, the registry key with the face index."]
    bytes_sha256: String,
    #[doc = "Pinned byte length of the face."]
    byte_len: usize,
    #[doc = "The face index inside its bytes (zero for every single-face asset here)."]
    index: u32,
    #[doc = "Whether this face serves the bold style slot."]
    bold: bool,
    #[doc = "Whether this face serves the italic style slot."]
    italic: bool,
}

impl ProfileFace {
    #[doc = "What this face serves (for example \"primary-bold\")."]
    #[must_use]
    pub const fn role(&self) -> &'static str {
        self.role
    }

    #[doc = "SHA-256 over the face bytes, the registry key with the face index."]
    #[must_use]
    pub fn bytes_sha256(&self) -> &str {
        &self.bytes_sha256
    }

    #[doc = "Pinned byte length of the face."]
    #[must_use]
    pub const fn byte_len(&self) -> usize {
        self.byte_len
    }

    #[doc = "The face index inside its bytes (zero for every single-face asset here)."]
    #[must_use]
    pub const fn index(&self) -> u32 {
        self.index
    }

    #[doc = "Whether this face serves the bold style slot."]
    #[must_use]
    pub const fn bold(&self) -> bool {
        self.bold
    }

    #[doc = "Whether this face serves the italic style slot."]
    #[must_use]
    pub const fn italic(&self) -> bool {
        self.italic
    }
}

#[doc = "One named, immutable set of render settings and font references."]
#[derive(Debug, Clone)]
pub struct Profile {
    #[doc = "The logical profile name."]
    name: &'static str,
    #[doc = "The pinned upstream font set backing every render through this profile."]
    fonts: PinnedFontSet,
    #[doc = "The registry faces in fallback order."]
    faces: Vec<ProfileFace>,
}

impl Profile {
    #[doc = "The logical profile name."]
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    #[doc = "The registry faces in fallback order."]
    #[must_use]
    pub fn faces(&self) -> &[ProfileFace] {
        &self.faces
    }

    #[doc = "The raster policy every render through this profile records."]
    #[must_use]
    pub const fn raster_policy() -> &'static str {
        RASTER_POLICY_PINNED_V1
    }

    #[doc = "Offline render options for this profile: pinned fonts with the profile metrics and zero chrome."]
    #[must_use]
    pub fn render_options(&self) -> SnapshotRenderOptions {
        SnapshotRenderOptions::default()
            .with_font_size(FONT_SIZE_PX)
            .with_cell_width(CELL_WIDTH_PX)
            .with_cell_height(CELL_HEIGHT_PX)
            .with_baseline(BASELINE_PX)
            .with_scale(SCALE)
            .with_canvas_padding(Padding::Uniform(0))
            .with_content_padding(Padding::Uniform(0))
            .with_title_bar(false)
            .with_title_controls(false)
            .with_shadow(false)
            .with_border_width(0.0)
            .with_border_radius(0.0)
            .with_pinned_fonts(self.fonts.clone())
    }
}

#[doc = "The platform key recorded in render contracts: `<os>-<arch>` from the Rust target names."]
#[must_use]
pub fn platform_key() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

#[doc = "Resolve a profile name through the immutable built-in registry; only `PROFILE_NAME` resolves.\n\n# Errors\n\nReturns `ProfileError::UnknownProfile` for every other name, and the build failure if committed font assets ever fail their pin checks."]
pub fn resolve_profile(name: &str) -> Result<&'static Profile, ProfileError> {
    if name != PROFILE_NAME {
        return Err(ProfileError::UnknownProfile {
            requested: name.to_string(),
        });
    }
    PINNED_V1
        .get_or_init(build_pinned_v1)
        .as_ref()
        .map_err(std::clone::Clone::clone)
}

static PINNED_V1: OnceLock<Result<Profile, ProfileError>> = OnceLock::new();

struct FaceSpec {
    role: &'static str,
    bytes: &'static [u8],
    len: usize,
    sha256: &'static str,
    bold: bool,
    italic: bool,
}

fn face_specs() -> Vec<FaceSpec> {
    let regular = |role: &'static str, bold: bool, italic: bool| FaceSpec {
        role,
        bytes: JB_REGULAR_BYTES,
        len: JB_REGULAR_LEN,
        sha256: JB_REGULAR_SHA256,
        bold,
        italic,
    };
    let symbol = |role: &'static str, bold: bool, italic: bool| FaceSpec {
        role,
        bytes: SYMBOL_BYTES,
        len: SYMBOL_LEN,
        sha256: SYMBOL_SHA256,
        bold,
        italic,
    };
    let cjk = |role: &'static str, bold: bool, italic: bool| FaceSpec {
        role,
        bytes: CJK_BYTES,
        len: CJK_LEN,
        sha256: CJK_SHA256,
        bold,
        italic,
    };
    vec![
        FaceSpec {
            role: "primary-regular",
            bytes: JB_REGULAR_BYTES,
            len: JB_REGULAR_LEN,
            sha256: JB_REGULAR_SHA256,
            bold: false,
            italic: false,
        },
        symbol("symbol-regular", false, false),
        cjk("cjk-regular", false, false),
        FaceSpec {
            role: "primary-bold",
            bytes: JB_BOLD_BYTES,
            len: JB_BOLD_LEN,
            sha256: JB_BOLD_SHA256,
            bold: true,
            italic: false,
        },
        regular("primary-regular-fallback-bold", true, false),
        symbol("symbol-bold", true, false),
        cjk("cjk-bold", true, false),
        FaceSpec {
            role: "primary-italic",
            bytes: JB_ITALIC_BYTES,
            len: JB_ITALIC_LEN,
            sha256: JB_ITALIC_SHA256,
            bold: false,
            italic: true,
        },
        regular("primary-regular-fallback-italic", false, true),
        symbol("symbol-italic", false, true),
        cjk("cjk-italic", false, true),
        FaceSpec {
            role: "primary-bold-italic",
            bytes: JB_BOLD_ITALIC_BYTES,
            len: JB_BOLD_ITALIC_LEN,
            sha256: JB_BOLD_ITALIC_SHA256,
            bold: true,
            italic: true,
        },
        regular("primary-regular-fallback-bold-italic", true, true),
        symbol("symbol-bold-italic", true, true),
        cjk("cjk-bold-italic", true, true),
    ]
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 15) as usize] as char);
    }
    out
}

fn select_face(spec: &FaceSpec) -> Result<(PinnedFace, ProfileFace), ProfileError> {
    if spec.bytes.len() != spec.len {
        return Err(ProfileError::AssetMismatch {
            face: spec.role,
            detail: format!(
                "byte length {} does not match the pinned length {}",
                spec.bytes.len(),
                spec.len
            ),
        });
    }
    let digest = sha256_hex(spec.bytes);
    if digest != spec.sha256 {
        return Err(ProfileError::AssetMismatch {
            face: spec.role,
            detail: format!(
                "SHA-256 {digest} does not match the pinned digest {}",
                spec.sha256
            ),
        });
    }
    let face = PinnedFace::new(spec.bytes.to_vec(), FACE_INDEX, spec.bold, spec.italic).map_err(
        |source| ProfileError::FontBuild {
            face: spec.role,
            source,
        },
    )?;
    let descriptor = ProfileFace {
        role: spec.role,
        bytes_sha256: digest,
        byte_len: spec.bytes.len(),
        index: FACE_INDEX,
        bold: spec.bold,
        italic: spec.italic,
    };
    Ok((face, descriptor))
}

fn build_pinned_v1() -> Result<Profile, ProfileError> {
    let mut fonts = Vec::new();
    let mut faces = Vec::new();
    for spec in face_specs() {
        let (face, descriptor) = select_face(&spec)?;
        fonts.push(face);
        faces.push(descriptor);
    }
    let fonts = PinnedFontSet::new(fonts).map_err(|source| ProfileError::FontBuild {
        face: "pinned set",
        source,
    })?;
    debug_assert!(!fonts.allow_synthetic_style());
    Ok(Profile {
        name: PROFILE_NAME,
        fonts,
        faces,
    })
}
