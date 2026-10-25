//! Named render profile with an immutable built-in font registry (D02, GOAL 13.2).
//!
//! One logical profile exists: `PROFILE_NAME` (`tuiscotti-pinned-v1`).
//! `resolve_profile` maps that name to its immutable settings; every other
//! name is a `ProfileError::UnknownProfile` rejection. No custom font
//! paths and no network font loading exist on this path.
//!
//! Font-byte source decision (GOAL 13.2, first branch checked 2026-10-25
//! against the U02+U03+U04+U05+U11 upstream patch stack): the upstream public
//! API supplies NO licensed bundled font bytes. The bundled `JetBrains Mono`
//! faces are private to the module (`REGULAR_FONTS`, `STYLED_FONTS` in
//! upstream `render/font.rs`) and the bundled Nerd symbol font is `pub(crate)`
//! (`FONT_DATA` in upstream `render/nerd_font.rs`). Nothing public returns
//! those bytes, so every slot below uses licensed downstream assets from
//! `assets/fonts` (kept under version control, never fetched):
//!
//! - Slots 1-2, primary text: `JetBrains Mono` Nerd Font Mono regular, bold,
//!   italic, and bold-italic (`assets/fonts/JetBrainsMonoNerdFontMono-*`).
//!   The Nerd Mono variant keeps every glyph cell one column wide, which the
//!   terminal grid requires. License: `assets/fonts/LICENSE-JetBrainsMono.txt`
//!   (OFL 1.1).
//! - Slot 3, symbol font: `DejaVu Sans Mono` Nerd Font Mono
//!   (`assets/fonts/DejaVuSansMNerdFontMono-Regular.ttf`). This is the
//!   downstream stand-in for the upstream bundled Nerd symbol font
//!   (`SymbolsNerdFontMono`): it covers Private Use Area icons, powerline
//!   symbols, and miscellaneous symbols. The Noto symbol subsets were
//!   rejected for this slot because neither covers any Private Use Area
//!   codepoint, which would leave the recorded PUA selection rule vacuous.
//!   License: `assets/fonts/LICENSE-DejaVuSansMono.txt`.
//! - Slot 4, CJK: `assets/fonts/NotoSansCJKjp-subset.otf` (678988 bytes,
//!   SHA-256 `777bee41...d897`), the recorded subset. License:
//!   `assets/fonts/LICENSE-Noto.txt`.
//!
//! Fallback order for ordinary text (recorded in every render contract as
//! per-face `fallback_position` entries with their style slots):
//!
//! 1. The selected `JetBrains Mono` styled face.
//! 2. The `JetBrains Mono` regular face.
//! 3. The symbol font.
//! 4. The pinned CJK subset.
//!
//! The upstream pinned rasterizer tries faces in set order within the
//! requested style slot, so each style slot carries its own chain: the
//! styled face, then the regular face in that slot, then the symbol font in
//! that slot, then the CJK subset in that slot. Private Use Area icons
//! resolve to the symbol font first among the faces that can serve them:
//! the symbol font precedes the CJK subset, and every PUA icon outside
//! `JetBrains Mono` coverage (verified class: `U+F6C5`, present in the symbol
//! font but absent from `JetBrains Mono` Nerd Font Mono 3.5.1) renders from
//! the symbol font. No host font, replacement glyph, or unselected
//! synthetic style is ever substituted: a grapheme no selected face
//! contains (mandatory case: `U+1F980`) is a typed missing-glyph error.
//!
//! Metrics: 16-pixel font, 10-by-20 cell, baseline 15 pixels from the cell
//! top, scale 1, zero canvas and content padding, no title bar, controls,
//! shadow, border, or radius. Resolved terminal colors come from the
//! snapshot; a cursor the snapshot does not hide draws in the fixed shown
//! phase. Metric clipping was verified before candidate construction (see
//! `crates/tuiscotti/tests/samples.rs`): the mandatory glyphs
//! (`e`+U+0301, `a`+U+0308, U+6771, last-column wide cell, cursor over
//! combined and wide cells) keep every ink pixel inside their cell bounds
//! at these metrics, so no metric changed from its initial value.

use std::fmt::{Display, Formatter};
use std::sync::OnceLock;

use sha2::{Digest, Sha256};
use tui_test::render::style::Padding;
use tui_test::{
    PinnedFace, PinnedFontError, PinnedFontSet, RASTER_POLICY_PINNED_V1, SnapshotRenderOptions,
};

/// The single logical profile name this registry resolves.
pub const PROFILE_NAME: &str = "tuiscotti-pinned-v1";

/// Grid font size in pixels.
pub const FONT_SIZE_PX: f32 = 16.0;
/// Terminal cell width in pixels.
pub const CELL_WIDTH_PX: f32 = 10.0;
/// Terminal cell height in pixels.
pub const CELL_HEIGHT_PX: f32 = 20.0;
/// Text baseline measured from the top of each cell, in pixels.
pub const BASELINE_PX: f32 = 15.0;
/// Output scale factor.
pub const SCALE: f64 = 1.0;
/// Terminal cell width in whole pixels: the integer twin of
/// [`CELL_WIDTH_PX`], pinned for checked grid arithmetic without casts.
pub const CELL_WIDTH_U32: u32 = 10;
/// Terminal cell height in whole pixels: the integer twin of
/// [`CELL_HEIGHT_PX`], pinned for checked grid arithmetic without casts.
pub const CELL_HEIGHT_U32: u32 = 20;

/// Face index inside every asset: each file holds one face.
const FACE_INDEX: u32 = 0;

// Embedded asset bytes. Paths resolve from this file to the repository root.
const JB_REGULAR_BYTES: &[u8] =
    include_bytes!("../../../assets/fonts/JetBrainsMonoNerdFontMono-Regular.ttf");
const JB_BOLD_BYTES: &[u8] =
    include_bytes!("../../../assets/fonts/JetBrainsMonoNerdFontMono-Bold.ttf");
const JB_ITALIC_BYTES: &[u8] =
    include_bytes!("../../../assets/fonts/JetBrainsMonoNerdFontMono-Italic.ttf");
const JB_BOLD_ITALIC_BYTES: &[u8] =
    include_bytes!("../../../assets/fonts/JetBrainsMonoNerdFontMono-BoldItalic.ttf");
const SYMBOL_BYTES: &[u8] =
    include_bytes!("../../../assets/fonts/DejaVuSansMNerdFontMono-Regular.ttf");
const CJK_BYTES: &[u8] = include_bytes!("../../../assets/fonts/NotoSansCJKjp-subset.otf");

/// Pinned byte length of each asset, verified at profile build.
const JB_REGULAR_LEN: usize = 2_573_248;
const JB_BOLD_LEN: usize = 2_577_016;
const JB_ITALIC_LEN: usize = 2_575_368;
const JB_BOLD_ITALIC_LEN: usize = 2_578_280;
const SYMBOL_LEN: usize = 2_814_292;
const CJK_LEN: usize = 678_988;

/// Pinned SHA-256 of each asset, verified at profile build.
const JB_REGULAR_SHA256: &str = "f2a5ea6cfab397445ffab00c0370927b66d61e560a05db5db271b42006381c1a";
const JB_BOLD_SHA256: &str = "bfcf9a917276ffc058867d87cbc8a5b2f1ab0f4b710e9170dc02763ccb80bd4b";
const JB_ITALIC_SHA256: &str = "31efd6ead98746f5b0afa1ee6dba60267ad48db36428360bee327bec10621f97";
const JB_BOLD_ITALIC_SHA256: &str =
    "9dba502e00e35209f6ed2a151c7376c051657b067cdebbc6e52d06cb9002cf31";
const SYMBOL_SHA256: &str = "9b55ade625f2d3f2a273ed16d9db2d924ad6236222920f9fd1324941cdc3c712";
const CJK_SHA256: &str = "777bee41f0c6076c00ad919384359a6e396b8822cf9056041fca8fcf2759d897";

/// Every failure a profile lookup or build can report, with sources retained.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ProfileError {
    /// The requested profile name is not in the built-in registry.
    UnknownProfile {
        /// The rejected name.
        requested: String,
    },
    /// Embedded font bytes do not match their pinned length or digest.
    AssetMismatch {
        /// Which asset failed its pin check.
        face: &'static str,
        /// What the pin check found.
        detail: String,
    },
    /// The upstream validated constructor rejected committed font bytes.
    FontBuild {
        /// Which face failed validation.
        face: &'static str,
        /// The upstream rejection.
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

/// One face in the built-in registry, in fallback order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileFace {
    role: &'static str,
    bytes_sha256: String,
    byte_len: usize,
    index: u32,
    bold: bool,
    italic: bool,
}

impl ProfileFace {
    /// What this face serves (for example `"primary-bold"`).
    #[must_use]
    pub const fn role(&self) -> &'static str {
        self.role
    }

    /// SHA-256 over the face bytes: the registry key with [`ProfileFace::index`].
    #[must_use]
    pub fn bytes_sha256(&self) -> &str {
        &self.bytes_sha256
    }

    /// Pinned byte length of the face.
    #[must_use]
    pub const fn byte_len(&self) -> usize {
        self.byte_len
    }

    /// The face index inside its bytes (zero for every single-face asset here).
    #[must_use]
    pub const fn index(&self) -> u32 {
        self.index
    }

    /// Whether this face serves the bold style slot.
    #[must_use]
    pub const fn bold(&self) -> bool {
        self.bold
    }

    /// Whether this face serves the italic style slot.
    #[must_use]
    pub const fn italic(&self) -> bool {
        self.italic
    }
}

/// One named, immutable set of render settings and font references.
#[derive(Debug, Clone)]
pub struct Profile {
    name: &'static str,
    fonts: PinnedFontSet,
    faces: Vec<ProfileFace>,
}

impl Profile {
    /// The logical profile name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// The registry faces in fallback order.
    #[must_use]
    pub fn faces(&self) -> &[ProfileFace] {
        &self.faces
    }

    /// The raster policy every render through this profile records.
    #[must_use]
    pub const fn raster_policy() -> &'static str {
        RASTER_POLICY_PINNED_V1
    }

    /// Offline render options for this profile: pinned fonts with the
    /// profile metrics, zero chrome, and no canvas background override (the
    /// grid fills every pixel from resolved snapshot colors).
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

/// The platform key recorded in render contracts: `<os>-<arch>` from the
/// Rust target names (for example `linux-x86_64`, `macos-aarch64`).
#[must_use]
pub fn platform_key() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// Resolve a profile name through the immutable built-in registry.
///
/// Only [`PROFILE_NAME`] resolves. Unknown names fail without touching
/// fonts, the network, or the filesystem. The profile builds once behind
/// a lock; no panic path exists on repeated lookups.
///
/// # Errors
///
/// Returns [`ProfileError::UnknownProfile`] for every other name, and the
/// build failure if committed font assets ever fail their pin checks.
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

/// The single built-in profile, constructed once and never mutated.
static PINNED_V1: OnceLock<Result<Profile, ProfileError>> = OnceLock::new();

/// One registry row: role, bytes, pinned length, pinned digest, style slot.
struct FaceSpec {
    role: &'static str,
    bytes: &'static [u8],
    len: usize,
    sha256: &'static str,
    bold: bool,
    italic: bool,
}

/// Build the registry in fallback order within each style slot: the styled
/// `JetBrains Mono` face, the `JetBrains Mono` regular face, the symbol font,
/// then the CJK subset.
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
        // Regular slot: the styled face is the regular face itself.
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
        // Bold slot.
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
        // Italic slot.
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
        // Bold-italic slot.
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

/// SHA-256 hex over raw bytes.
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

/// Verify one asset against its pins and select it for its style slot.
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
    // Synthetic styles stay off: every slot has explicit faces, so the
    // renderer fails loudly instead of restyling bytes behind the profile.
    debug_assert!(!fonts.allow_synthetic_style());
    Ok(Profile {
        name: PROFILE_NAME,
        fonts,
        faces,
    })
}
