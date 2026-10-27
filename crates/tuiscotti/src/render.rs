#![doc = include_str!("../docs/render.md")]
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use tui_test::{PinnedFontError, RenderError as UpstreamRenderError};

use crate::approve::sample_code;
use crate::render_profile::{PROFILE_NAME, ProfileError, resolve_profile};
use crate::report::escape_html;
use crate::sample::{Sample, SampleError};

#[doc = "Every failure an offline sample render can report."]
#[derive(Debug)]
#[non_exhaustive]
pub enum RenderError {
    #[doc = "The output path carries neither a `.png` nor an `.html` extension."]
    UnsupportedExtension {
        #[doc = "The rejected output path, for the diagnostic."]
        path: PathBuf,
    },
    #[doc = "The sample bytes fail `Sample` validation."]
    Sample(SampleError),
    #[doc = "A stored image sample carries PNG bytes that do not validate."]
    StoredImage {
        #[doc = "What the stored PNG broke."]
        detail: String,
    },
    #[doc = "A state-only sample names a render profile that cannot be used."]
    Profile(ProfileError),
    #[doc = "The upstream renderer refuses a state-only sample snapshot."]
    Render {
        #[doc = "The upstream rejection, with its source chain."]
        source: UpstreamRenderError,
    },
    #[doc = "A filesystem failure outside sample validation."]
    Io {
        #[doc = "The filesystem failure."]
        source: std::io::Error,
    },
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedExtension { path } => write!(
                formatter,
                "unsupported output extension: expected a `.png` or `.html` path for {}",
                path.display()
            ),
            Self::Sample(source) => write!(formatter, "{source}"),
            Self::StoredImage { detail } => write!(formatter, "{detail}"),
            Self::Profile(source) => write!(formatter, "{source}"),
            Self::Render { source } => write!(formatter, "state-only render failed: {source}"),
            Self::Io { source } => write!(formatter, "render I/O failed: {source}"),
        }
    }
}

impl std::error::Error for RenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sample(source) => Some(source),
            Self::Profile(source) => Some(source),
            Self::Render { source } => Some(source),
            Self::Io { source } => Some(source),
            _ => None,
        }
    }
}

impl From<SampleError> for RenderError {
    fn from(source: SampleError) -> Self {
        Self::Sample(source)
    }
}

impl From<std::io::Error> for RenderError {
    fn from(source: std::io::Error) -> Self {
        Self::Io { source }
    }
}

impl RenderError {
    #[doc = "The stable Section 16.3 machine code for this failure."]
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedExtension { .. } => "INVALID_SAMPLE",
            Self::Sample(source) => sample_code(source),
            Self::StoredImage { .. } => "INVALID_IMAGE",
            Self::Profile(source) => match source {
                ProfileError::UnknownProfile { .. } => "UNKNOWN_PROFILE",
                _ => "RENDER_FAILED",
            },
            Self::Render { source } => match source {
                UpstreamRenderError::PinnedFont(PinnedFontError::MissingGlyph { .. }) => {
                    "MISSING_GLYPH"
                }
                _ => "RENDER_FAILED",
            },
            Self::Io { .. } => "IO",
        }
    }
}

#[doc = "One completed offline render: the reviewed bytes written to one file."]
#[derive(Debug, Clone)]
pub struct RenderOutcome {
    #[doc = "The rendered test ID."]
    test_id: String,
    #[doc = "SHA-256 over the rendered sample bytes."]
    candidate_sha256: String,
    #[doc = "The render profile for a rendered state-only sample, else `None`."]
    reason: Option<String>,
    #[doc = "The written output file."]
    output_path: PathBuf,
}

impl RenderOutcome {
    #[doc = "The rendered test ID."]
    #[must_use]
    pub fn test_id(&self) -> &str {
        &self.test_id
    }

    #[doc = "SHA-256 over the rendered sample bytes."]
    #[must_use]
    pub fn candidate_sha256(&self) -> &str {
        &self.candidate_sha256
    }

    #[doc = "The render profile for a rendered state-only sample, else `None`."]
    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    #[doc = "The written output file."]
    #[must_use]
    pub fn output_path(&self) -> &Path {
        &self.output_path
    }
}

#[doc = "Render saved sample bytes to `out` and return the outcome.\n\n# Errors\n\nReturns `RenderError` for unsupported extensions, invalid samples, stored-image failures, unknown profiles, renderer refusals, and I/O failures."]
pub fn render_sample(sample_bytes: &[u8], out: &Path) -> Result<RenderOutcome, RenderError> {
    if !wants_png(out) && !wants_html(out) {
        return Err(RenderError::UnsupportedExtension {
            path: out.to_path_buf(),
        });
    }
    let candidate_sha256 = Sample::candidate_id(sample_bytes);
    let sample = Sample::from_bytes(sample_bytes)?;
    let test_id = sample.test_id().to_string();
    let (png_bytes, profile_text, reason) = if let Some(image) = sample.image() {
        match decode_stored_png(image.png_base64(), image.width(), image.height()) {
            Ok(png) => (png, "stored image".to_string(), None),
            Err(detail) => return Err(RenderError::StoredImage { detail }),
        }
    } else {
        let png = render_state_only(&sample)?;
        (
            png,
            PROFILE_NAME.to_string(),
            Some(PROFILE_NAME.to_string()),
        )
    };
    write_output(out, &sample, &profile_text, &png_bytes)?;
    Ok(RenderOutcome {
        test_id,
        candidate_sha256,
        reason,
        output_path: out.to_path_buf(),
    })
}

fn wants_png(out: &Path) -> bool {
    out.extension().and_then(OsStr::to_str) == Some("png")
}

fn wants_html(out: &Path) -> bool {
    out.extension().and_then(OsStr::to_str) == Some("html")
}

fn write_output(
    out: &Path,
    sample: &Sample,
    profile_text: &str,
    png_bytes: &[u8],
) -> Result<(), RenderError> {
    if wants_png(out) {
        return std::fs::write(out, png_bytes).map_err(|source| RenderError::Io {
            source: write_error(out, &source),
        });
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(png_bytes);
    let (width, height) = image_dimensions(png_bytes);
    let page = render_page(sample, profile_text, &encoded, width, height);
    std::fs::write(out, page.as_bytes()).map_err(|source| RenderError::Io {
        source: write_error(out, &source),
    })?;
    Ok(())
}

fn render_page(
    sample: &Sample,
    profile: &str,
    png_base64: &str,
    png_width: u32,
    png_height: u32,
) -> String {
    let test_id = escape_html(sample.test_id());
    let gate = sample.gate().as_str();
    let profile = escape_html(profile);
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head><meta charset=\"utf-8\">\n<title>tuiscotti render {test_id}</title>\n</head>\n<body>\n<p>test: {test_id}</p>\n<p>gate: {gate}</p>\n<p>profile: {profile}</p>\n<p>image: {png_width}x{png_height}</p>\n<img src=\"data:image/png;base64,{png_base64}\" alt=\"rendered sample {test_id}\">\n</body>\n</html>\n"
    )
}

fn decode_stored_png(encoded: &str, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(encoded.as_bytes())
        .map_err(|error| format!("stored PNG is not valid Base64: {error}"))?;
    let parsed = image::load_from_memory(&raw)
        .map_err(|error| format!("stored PNG does not parse: {error}"))?;
    let rgba = parsed.to_rgba8();
    if rgba.width() != width || rgba.height() != height {
        return Err(format!(
            "stored PNG decodes to {}x{} but the sample records {width}x{height}",
            rgba.width(),
            rgba.height()
        ));
    }
    Ok(raw)
}

fn render_state_only(sample: &Sample) -> Result<Vec<u8>, RenderError> {
    let profile = resolve_profile(PROFILE_NAME).map_err(RenderError::Profile)?;
    let rendered = tui_test::render_snapshot_png(sample.snapshot(), &profile.render_options())
        .map_err(|source| RenderError::Render { source })?;
    Ok(rendered.png().to_vec())
}

fn write_error(out: &Path, source: &std::io::Error) -> std::io::Error {
    std::io::Error::new(
        source.kind(),
        format!("cannot write {}: {source}", out.display()),
    )
}

fn image_dimensions(png: &[u8]) -> (u32, u32) {
    match image::load_from_memory(png) {
        Ok(parsed) => {
            let rgba = parsed.to_rgba8();
            (rgba.width(), rgba.height())
        }
        Err(_) => (0, 0),
    }
}
