#![doc = include_str!("../docs/approve.md")]
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use sha2::{Digest, Sha256};

use tui_test::{PinnedFontError, RenderError, render_snapshot_png};

use crate::check::{
    APPROVAL_LOCK_TIMEOUT_SECS, ApprovalFileLock, ApprovalLockError, CheckError, FrozenStore,
    MISSING_BASELINE, MismatchReason, PresentComparison, validate_test_id,
};
use crate::render_profile::{ProfileError, resolve_profile};
use crate::sample::{
    Gate, RenderContract, Sample, SampleError, SampleImage, read_bounded, read_bounded_optional,
};

#[derive(Debug)]
#[non_exhaustive]
#[doc = "Every failure an approval transaction can report, with sources retained."]
pub enum ApproveError {
    #[doc = "The `CI` variable is set, so approval is refused before any write."]
    CiApprovalForbidden,
    #[doc = "A digest argument is missing, malformed, or a zero placeholder."]
    InvalidArgument {
        #[doc = "Which argument failed and how."]
        detail: String,
    },
    #[doc = "The candidate bytes disagree with the supplied candidate digest."]
    StaleCandidate {
        #[doc = "The supplied candidate digest."]
        expected: String,
        #[doc = "The digest of the bytes actually read."]
        actual: String,
    },
    #[doc = "The present baseline disagrees with the expected previous digest."]
    StaleBaseline {
        #[doc = "The expected previous digest, or `missing` for a first approval."]
        expected: String,
        #[doc = "The digest of the bytes actually present, or `missing` when absent."]
        actual: String,
    },
    #[doc = "The approval lock stayed held past the acquisition deadline."]
    LockTimeout,
    #[doc = "The candidate names a render profile outside the D02 registry."]
    UnknownProfile {
        #[doc = "The rejected profile name."]
        requested: String,
        #[doc = "The baseline digest observed under the lock, or `None` before the baseline read."]
        observed_baseline: Option<String>,
    },
    #[doc = "The upstream renderer refused the candidate snapshot during the re-render."]
    RenderFailed {
        #[doc = "The upstream rejection, with its source chain."]
        source: RenderError,
        #[doc = "The baseline digest observed under the lock, or `None` before the baseline read."]
        observed_baseline: Option<String>,
    },
    #[doc = "No specified font can render mandatory text in the candidate snapshot."]
    MissingGlyph {
        #[doc = "The upstream missing-glyph rejection."]
        source: PinnedFontError,
        #[doc = "The baseline digest observed under the lock, or `None` before the baseline read."]
        observed_baseline: Option<String>,
    },
    #[doc = "The re-rendered pixels differ from the stored candidate pixels."]
    InconsistentSample {
        #[doc = "What disagreed (dimensions and compared byte counts)."]
        detail: String,
        #[doc = "The baseline digest observed under the lock, or `None` before the baseline read."]
        observed_baseline: Option<String>,
    },
    #[doc = "A baseline or evidence path is empty, unresolvable, or escapes its root."]
    InvalidStorePath {
        #[doc = "What the path broke."]
        detail: String,
        #[doc = "The baseline digest observed under the lock, or `None` before the baseline read."]
        observed_baseline: Option<String>,
    },
    #[doc = "The inner frozen comparison, candidate validation, or record serialization failed."]
    Check {
        #[doc = "The frozen-store failure, with its source chain."]
        source: CheckError,
        #[doc = "The baseline digest observed under the lock, or `None` before the baseline read."]
        observed_baseline: Option<String>,
    },
    #[doc = "A filesystem failure outside sample validation."]
    Io {
        #[doc = "The filesystem failure."]
        source: std::io::Error,
        #[doc = "The baseline digest observed under the lock, or `None` before the baseline read."]
        observed_baseline: Option<String>,
    },
}

impl Display for ApproveError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CiApprovalForbidden => {
                write!(formatter, "approval is forbidden while CI is set")
            }
            Self::InvalidArgument { detail } => {
                write!(formatter, "invalid approval argument: {detail}")
            }
            Self::StaleCandidate { expected, actual } => write!(
                formatter,
                "candidate changed: expected digest {expected}, read bytes digest {actual}"
            ),
            Self::StaleBaseline { expected, actual } => write!(
                formatter,
                "baseline changed: expected previous digest {expected}, present digest {actual}"
            ),
            Self::LockTimeout => write!(formatter, "approval lock deadline passed"),
            Self::UnknownProfile { requested, .. } => {
                write!(formatter, "unknown render profile {requested:?}")
            }
            Self::RenderFailed { source, .. } => {
                write!(formatter, "candidate re-render failed: {source}")
            }
            Self::MissingGlyph { source, .. } => {
                write!(
                    formatter,
                    "candidate re-render needs an unavailable glyph: {source}"
                )
            }
            Self::InconsistentSample { detail, .. } => {
                write!(formatter, "candidate re-render is inconsistent: {detail}")
            }
            Self::InvalidStorePath { detail, .. } => {
                write!(formatter, "invalid store path: {detail}")
            }
            Self::Check { source, .. } => write!(formatter, "{source}"),
            Self::Io { source, .. } => write!(formatter, "approval I/O failed: {source}"),
        }
    }
}

impl std::error::Error for ApproveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Check { source, .. } => Some(source),
            Self::RenderFailed { source, .. } => Some(source),
            Self::MissingGlyph { source, .. } => Some(source),
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<CheckError> for ApproveError {
    fn from(source: CheckError) -> Self {
        Self::Check {
            source,
            observed_baseline: None,
        }
    }
}

impl From<SampleError> for ApproveError {
    fn from(source: SampleError) -> Self {
        Self::Check {
            source: CheckError::from(source),
            observed_baseline: None,
        }
    }
}

impl From<std::io::Error> for ApproveError {
    fn from(source: std::io::Error) -> Self {
        Self::Io {
            source,
            observed_baseline: None,
        }
    }
}

impl ApproveError {
    #[must_use]
    #[doc = "The stable machine code for this failure, shared with the CLI map."]
    pub fn code(&self) -> &'static str {
        match self {
            Self::CiApprovalForbidden => "CI_APPROVAL_FORBIDDEN",
            Self::InvalidArgument { .. } => "INVALID_SAMPLE",
            Self::StaleCandidate { .. } => "STALE_CANDIDATE",
            Self::StaleBaseline { .. } => "STALE_BASELINE",
            Self::LockTimeout => "LOCK_TIMEOUT",
            Self::UnknownProfile { .. } => "UNKNOWN_PROFILE",
            Self::RenderFailed { .. } => "RENDER_FAILED",
            Self::MissingGlyph { .. } => "MISSING_GLYPH",
            Self::InconsistentSample { .. } => "INCONSISTENT_SAMPLE",
            Self::InvalidStorePath { .. } => "INVALID_STORE_PATH",
            Self::Check { source, .. } => match source {
                CheckError::Sample(source) => sample_code(source),
                CheckError::InvalidTestId { .. } | CheckError::InvalidStorePath { .. } => {
                    "INVALID_STORE_PATH"
                }
                CheckError::Pixel(_) => "INVALID_IMAGE",
                CheckError::EvidenceCorrupt { .. }
                | CheckError::Image { .. }
                | CheckError::Io { .. }
                | CheckError::Report(_) => "IO",
            },
            Self::Io { .. } => "IO",
        }
    }

    #[must_use]
    #[doc = "The digest the baseline root actually held when this failure was reported."]
    pub fn observed_baseline(&self) -> Option<&str> {
        match self {
            Self::StaleBaseline { actual, .. } => Some(actual),
            Self::UnknownProfile {
                observed_baseline, ..
            }
            | Self::RenderFailed {
                observed_baseline, ..
            }
            | Self::MissingGlyph {
                observed_baseline, ..
            }
            | Self::InconsistentSample {
                observed_baseline, ..
            }
            | Self::InvalidStorePath {
                observed_baseline, ..
            }
            | Self::Check {
                observed_baseline, ..
            }
            | Self::Io {
                observed_baseline, ..
            } => observed_baseline.as_deref(),
            _ => None,
        }
    }

    #[must_use]
    fn with_observed_baseline(mut self, observed: &str) -> Self {
        let (Self::UnknownProfile {
            observed_baseline: slot,
            ..
        }
        | Self::RenderFailed {
            observed_baseline: slot,
            ..
        }
        | Self::MissingGlyph {
            observed_baseline: slot,
            ..
        }
        | Self::InconsistentSample {
            observed_baseline: slot,
            ..
        }
        | Self::InvalidStorePath {
            observed_baseline: slot,
            ..
        }
        | Self::Check {
            observed_baseline: slot,
            ..
        }
        | Self::Io {
            observed_baseline: slot,
            ..
        }) = &mut self
        else {
            return self;
        };
        *slot = Some(observed.to_string());
        self
    }
}

#[must_use]
#[doc = "The Section 16.3 code for one validated sample failure."]
pub fn sample_code(source: &SampleError) -> &'static str {
    match source {
        SampleError::TooLarge { .. }
        | SampleError::DimensionTooLarge { .. }
        | SampleError::RgbaTooLarge { .. } => "RESOURCE_LIMIT",
        SampleError::UnsupportedSchema { .. } => "UNSUPPORTED_SCHEMA",
        SampleError::PngInvalid { .. }
        | SampleError::PngDecode { .. }
        | SampleError::Base64Invalid { .. }
        | SampleError::DigestMismatch { .. }
        | SampleError::DimensionMismatch { .. } => "INVALID_IMAGE",
        SampleError::Render { source } => match source {
            RenderError::PinnedFont(PinnedFontError::MissingGlyph { .. }) => "MISSING_GLYPH",
            _ => "RENDER_FAILED",
        },
        SampleError::Io { .. } => "IO",
        _ => "INVALID_SAMPLE",
    }
}

#[derive(Debug, Clone)]
#[doc = "One completed approval transaction: the reviewed candidate is now the approved baseline."]
pub struct AcceptOutcome {
    #[doc = "The approved test ID."]
    test_id: String,
    #[doc = "The gate both the candidate and the new baseline carry."]
    gate: Gate,
    #[doc = "SHA-256 over the approved candidate bytes."]
    candidate_sha256: String,
    #[doc = "The previous baseline digest, or `missing` for a first approval."]
    previous_baseline_sha256: String,
    #[doc = "SHA-256 over the written baseline bytes, always the candidate digest."]
    baseline_sha256: String,
    #[doc = "The upserted baseline file."]
    baseline_path: PathBuf,
    #[doc = "The structured change reason for this approval."]
    reason: Option<MismatchReason>,
    #[doc = "Changed fields recorded for this approval, in comparison order."]
    changed_fields: Vec<String>,
}

impl AcceptOutcome {
    #[must_use]
    #[doc = "The approved test ID."]
    pub fn test_id(&self) -> &str {
        &self.test_id
    }

    #[must_use]
    #[doc = "The gate both the candidate and the new baseline carry."]
    pub const fn gate(&self) -> Gate {
        self.gate
    }

    #[must_use]
    #[doc = "SHA-256 over the approved candidate bytes."]
    pub fn candidate_sha256(&self) -> &str {
        &self.candidate_sha256
    }

    #[must_use]
    #[doc = "The previous baseline digest, or `missing` for a first approval."]
    pub fn previous_baseline_sha256(&self) -> &str {
        &self.previous_baseline_sha256
    }

    #[must_use]
    #[doc = "SHA-256 over the written baseline bytes, always the candidate digest."]
    pub fn baseline_sha256(&self) -> &str {
        &self.baseline_sha256
    }

    #[must_use]
    #[doc = "The upserted baseline file."]
    pub fn baseline_path(&self) -> &Path {
        &self.baseline_path
    }

    #[must_use]
    #[doc = "The structured change reason for this approval."]
    pub const fn reason(&self) -> Option<MismatchReason> {
        self.reason
    }

    #[must_use]
    #[doc = "Changed fields recorded for this approval, in comparison order."]
    pub fn changed_fields(&self) -> &[String] {
        &self.changed_fields
    }
}

#[derive(Debug, Clone)]
#[doc = "An explicit approval handle over one baseline root."]
pub struct ApprovalStore {
    #[doc = "The canonical baseline root this handle approves into."]
    baselines: PathBuf,
}

impl ApprovalStore {
    #[doc = "Open an approval handle over a baseline root.\n\n# Errors\n\nReturns [`ApproveError`] for an empty or unresolvable root."]
    pub fn new(baseline_root: &Path) -> Result<Self, ApproveError> {
        let baselines = resolve_baseline_root(baseline_root)?;
        Ok(Self { baselines })
    }

    #[must_use]
    #[doc = "The canonical baseline root this handle approves into."]
    pub fn baseline_root(&self) -> &Path {
        &self.baselines
    }

    #[doc = "Approve one candidate for one test ID.\n\n# Errors\n\nReturns [`ApproveError`] for `CI` refusal, digest rejections, stale candidates or baselines, invalid samples or paths, lock timeouts, and I/O failures."]
    pub fn accept(
        &self,
        candidate_path: &Path,
        candidate_sha256: &str,
        expected_baseline_sha256: &str,
    ) -> Result<AcceptOutcome, ApproveError> {
        require_no_ci()?;
        self.accept_core(candidate_path, candidate_sha256, expected_baseline_sha256)
    }

    fn accept_core(
        &self,
        candidate_path: &Path,
        candidate_sha256: &str,
        expected_baseline_sha256: &str,
    ) -> Result<AcceptOutcome, ApproveError> {
        require_digest("candidate", candidate_sha256, false)?;
        require_digest("expected baseline", expected_baseline_sha256, true)?;
        let baselines = resolve_baseline_root(&self.baselines)?;
        if !baselines.exists() {
            fs::create_dir_all(&baselines).map_err(|source| ApproveError::Io {
                source,
                observed_baseline: None,
            })?;
        }
        let lock =
            ApprovalFileLock::acquire(&baselines, Duration::from_secs(APPROVAL_LOCK_TIMEOUT_SECS))
                .map_err(|error| match error {
                    ApprovalLockError::Timeout => ApproveError::LockTimeout,
                    ApprovalLockError::Io(source) => ApproveError::Io {
                        source,
                        observed_baseline: None,
                    },
                })?;
        let transaction = Self::accept_locked(
            &baselines,
            candidate_path,
            candidate_sha256,
            expected_baseline_sha256,
        );
        drop(lock);
        transaction
    }

    fn accept_locked(
        baselines: &Path,
        candidate_path: &Path,
        candidate_sha256: &str,
        expected_baseline: &str,
    ) -> Result<AcceptOutcome, ApproveError> {
        let candidate_bytes = read_candidate_bytes(candidate_path)?;
        let actual_candidate = sha256_hex(&candidate_bytes);
        if actual_candidate != candidate_sha256 {
            return Err(ApproveError::StaleCandidate {
                expected: candidate_sha256.to_string(),
                actual: actual_candidate,
            });
        }
        let candidate = Sample::from_bytes(&candidate_bytes)?;
        validate_test_id(candidate.test_id())?;
        let relative = baseline_relative(&candidate)?;
        ensure_no_symlink(baselines, &relative)?;
        let baseline_bytes = read_baseline_bytes(baselines, &relative)?;
        let previous_digest = baseline_bytes
            .as_deref()
            .map_or_else(|| MISSING_BASELINE.to_string(), sha256_hex);
        if previous_digest != expected_baseline {
            return Err(ApproveError::StaleBaseline {
                expected: expected_baseline.to_string(),
                actual: previous_digest,
            });
        }
        rerender_check(&candidate)
            .map_err(|error| error.with_observed_baseline(&previous_digest))?;
        let changes = match baseline_bytes {
            None => ChangeSet::missing(),
            Some(bytes) => {
                let baseline = Sample::from_bytes(&bytes)
                    .map_err(ApproveError::from)
                    .map_err(|error| error.with_observed_baseline(&previous_digest))?;
                let comparison = FrozenStore::compare_present(&candidate, &baseline)
                    .map_err(ApproveError::from)
                    .map_err(|error| error.with_observed_baseline(&previous_digest))?;
                ChangeSet::from_present(comparison)
            }
        };
        write_baseline(
            baselines,
            &relative,
            &candidate,
            &candidate_bytes,
            &actual_candidate,
            &previous_digest,
            changes,
        )
        .map_err(|error| error.with_observed_baseline(&previous_digest))
    }
}

fn baseline_relative(candidate: &Sample) -> Result<PathBuf, ApproveError> {
    let mut relative = PathBuf::new();
    for component in Path::new(candidate.test_id()).components() {
        match component {
            Component::Normal(part) => relative.push(part),
            other => {
                return Err(ApproveError::InvalidStorePath {
                    detail: format!(
                        "test ID {:?} carries a non-normal path component ({other:?})",
                        candidate.test_id()
                    ),
                    observed_baseline: None,
                });
            }
        }
    }
    relative.set_extension("tui.json");
    if relative.parent().is_none() {
        return Err(ApproveError::InvalidStorePath {
            detail: "baseline path has no parent directory".to_string(),
            observed_baseline: None,
        });
    }
    if relative
        .file_name()
        .and_then(|name| name.to_str())
        .is_none()
    {
        return Err(ApproveError::InvalidStorePath {
            detail: "baseline path has no file name".to_string(),
            observed_baseline: None,
        });
    }
    Ok(relative)
}

fn read_baseline_bytes(baselines: &Path, relative: &Path) -> Result<Option<Vec<u8>>, ApproveError> {
    read_bounded_optional(&baselines.join(relative)).map_err(|error| match error {
        SampleError::Io { source } => ApproveError::Io {
            source,
            observed_baseline: None,
        },
        other => ApproveError::from(other),
    })
}

fn write_baseline(
    baselines: &Path,
    relative: &Path,
    candidate: &Sample,
    candidate_bytes: &[u8],
    candidate_digest: &str,
    previous_digest: &str,
    changes: ChangeSet,
) -> Result<AcceptOutcome, ApproveError> {
    let parent = relative
        .parent()
        .ok_or_else(|| ApproveError::InvalidStorePath {
            detail: "baseline path has no parent directory".to_string(),
            observed_baseline: None,
        })?;
    let file_name = relative
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| ApproveError::InvalidStorePath {
            detail: "baseline path has no file name".to_string(),
            observed_baseline: None,
        })?;
    let parent_dir = baselines.join(parent);
    fs::create_dir_all(&parent_dir).map_err(|source| ApproveError::Io {
        source,
        observed_baseline: None,
    })?;
    let (mut staged, temp) =
        TempFile::create(&parent_dir, file_name).map_err(|source| ApproveError::Io {
            source,
            observed_baseline: None,
        })?;
    staged
        .write_all(candidate_bytes)
        .map_err(|source| ApproveError::Io {
            source,
            observed_baseline: None,
        })?;
    staged.flush().map_err(|source| ApproveError::Io {
        source,
        observed_baseline: None,
    })?;
    drop(staged);
    let outcome = AcceptOutcome {
        test_id: candidate.test_id().to_string(),
        gate: candidate.gate(),
        candidate_sha256: candidate_digest.to_string(),
        previous_baseline_sha256: previous_digest.to_string(),
        baseline_sha256: candidate_digest.to_string(),
        baseline_path: parent_dir.join(file_name),
        reason: changes.reason,
        changed_fields: changes.fields,
    };
    temp.commit(&outcome.baseline_path)
        .map_err(|source| ApproveError::Io {
            source,
            observed_baseline: None,
        })?;
    Ok(outcome)
}

fn rerender_check(candidate: &Sample) -> Result<(), ApproveError> {
    let Some(image) = candidate.image() else {
        return Ok(());
    };
    let contract = candidate
        .render_contract()
        .ok_or_else(|| ApproveError::Check {
            source: CheckError::Sample(SampleError::GateMismatch {
                detail: "state-and-pixels gate requires a render contract".to_string(),
            }),
            observed_baseline: None,
        })?;
    let profile = resolve_profile(contract.profile()).map_err(|error| match error {
        ProfileError::UnknownProfile { requested } => ApproveError::UnknownProfile {
            requested,
            observed_baseline: None,
        },
        other => ApproveError::RenderFailed {
            source: RenderError::RasterFailed {
                message: format!("render profile does not build: {other}"),
            },
            observed_baseline: None,
        },
    })?;
    let rendered = match render_snapshot_png(candidate.snapshot(), &profile.render_options()) {
        Ok(rendered) => rendered,
        Err(RenderError::PinnedFont(source @ PinnedFontError::MissingGlyph { .. })) => {
            return Err(ApproveError::MissingGlyph {
                source,
                observed_baseline: None,
            });
        }
        Err(other) => {
            return Err(ApproveError::RenderFailed {
                source: other,
                observed_baseline: None,
            });
        }
    };
    let expected = RenderContract::from_rendered(profile, &rendered)?;
    if let Some(field) = contract_identity_change(contract, &expected) {
        return Err(ApproveError::InconsistentSample {
            detail: format!(
                "recorded render contract field {field} differs from the re-rendered identity"
            ),
            observed_baseline: None,
        });
    }
    let fresh = decode_rendered_png(rendered.png())?;
    let stored = decode_stored_png(image)?;
    if fresh != stored {
        return Err(ApproveError::InconsistentSample {
            detail: format!(
                "re-rendered {}x{} pixels differ from the candidate {}x{} pixels",
                fresh.width, fresh.height, stored.width, stored.height,
            ),
            observed_baseline: None,
        });
    }
    Ok(())
}

fn contract_identity_change(
    recorded: &RenderContract,
    expected: &RenderContract,
) -> Option<&'static str> {
    if recorded.profile() != expected.profile() {
        return Some("profile");
    }
    if recorded.contract_version() != expected.contract_version() {
        return Some("contract_version");
    }
    if recorded.renderer_version() != expected.renderer_version() {
        return Some("renderer_version");
    }
    if recorded.raster_policy() != expected.raster_policy() {
        return Some("raster_policy");
    }
    if recorded.fonts() != expected.fonts() {
        return Some("fonts");
    }
    if recorded.cell_width().to_bits() != expected.cell_width().to_bits() {
        return Some("cell_width");
    }
    if recorded.cell_height().to_bits() != expected.cell_height().to_bits() {
        return Some("cell_height");
    }
    if recorded.font_size().to_bits() != expected.font_size().to_bits() {
        return Some("font_size");
    }
    if recorded.baseline().to_bits() != expected.baseline().to_bits() {
        return Some("baseline");
    }
    if recorded.scale().to_bits() != expected.scale().to_bits() {
        return Some("scale");
    }
    if recorded.title_bar() != expected.title_bar() {
        return Some("title_bar");
    }
    if recorded.palette_digest() != expected.palette_digest() {
        return Some("palette_digest");
    }
    if recorded.cursor_phase() != expected.cursor_phase() {
        return Some("cursor_phase");
    }
    if recorded.blink() != expected.blink() {
        return Some("blink");
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DecodedPng {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

fn decode_rendered_png(png: &[u8]) -> Result<DecodedPng, ApproveError> {
    decode_png(png).map_err(|detail| ApproveError::RenderFailed {
        source: RenderError::RasterFailed {
            message: format!("re-rendered PNG does not decode: {detail}"),
        },
        observed_baseline: None,
    })
}

fn decode_stored_png(image: &SampleImage) -> Result<DecodedPng, ApproveError> {
    use base64::Engine as _;
    let raw = base64::engine::general_purpose::STANDARD
        .decode(image.png_base64().as_bytes())
        .map_err(|source| ApproveError::Check {
            source: CheckError::Sample(SampleError::Base64Invalid { source }),
            observed_baseline: None,
        })?;
    if sha256_hex(&raw) != image.png_sha256() {
        return Err(ApproveError::Check {
            source: CheckError::Sample(SampleError::DigestMismatch {
                field: "png_sha256",
                expected: image.png_sha256().to_string(),
                actual: sha256_hex(&raw),
            }),
            observed_baseline: None,
        });
    }
    decode_png(&raw).map_err(|detail| ApproveError::Check {
        source: CheckError::Sample(SampleError::PngInvalid { detail }),
        observed_baseline: None,
    })
}

fn decode_png(png: &[u8]) -> Result<DecodedPng, String> {
    use image::ImageDecoder as _;
    let decoder = image::codecs::png::PngDecoder::new(std::io::Cursor::new(png))
        .map_err(|error| format!("PNG does not decode: {error}"))?;
    if decoder.color_type() != image::ColorType::Rgba8 {
        return Err(format!(
            "PNG color type is {:?}; only 8-bit RGBA is supported",
            decoder.color_type()
        ));
    }
    let (width, height) = decoder.dimensions();
    let total = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4));
    let Some(total) = total.and_then(|bytes| usize::try_from(bytes).ok()) else {
        return Err("PNG dimensions overflow the pixel buffer".to_string());
    };
    let mut pixels = vec![0u8; total];
    decoder
        .read_image(&mut pixels)
        .map_err(|error| format!("PNG pixels do not read: {error}"))?;
    Ok(DecodedPng {
        width,
        height,
        pixels,
    })
}

fn ci_is_set() -> bool {
    std::env::var_os("CI").is_some()
}

fn require_no_ci() -> Result<(), ApproveError> {
    if ci_is_set() {
        return Err(ApproveError::CiApprovalForbidden);
    }
    Ok(())
}

fn require_digest(role: &str, value: &str, allow_missing: bool) -> Result<(), ApproveError> {
    let invalid = |detail: String| ApproveError::InvalidArgument { detail };
    if value.is_empty() {
        return Err(invalid(format!("{role} digest is missing")));
    }
    if allow_missing && value == MISSING_BASELINE {
        return Ok(());
    }
    if !is_hex_digest(value) {
        return Err(invalid(format!(
            "{role} digest {value:?} is not a 64-character lowercase hex digest"
        )));
    }
    if value.bytes().all(|byte| byte == b'0') {
        return Err(invalid(format!(
            "{role} digest must not be a zero placeholder"
        )));
    }
    Ok(())
}

fn is_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
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

fn read_candidate_bytes(path: &Path) -> Result<Vec<u8>, ApproveError> {
    read_bounded(path).map_err(|error| match error {
        SampleError::Io { source } => ApproveError::Io {
            source: std::io::Error::new(
                source.kind(),
                format!("cannot read candidate {}: {source}", path.display()),
            ),
            observed_baseline: None,
        },
        other => ApproveError::from(other),
    })
}

fn resolve_baseline_root(path: &Path) -> Result<PathBuf, ApproveError> {
    if path.as_os_str().is_empty() {
        return Err(ApproveError::InvalidStorePath {
            detail: "baseline root path is empty".to_string(),
            observed_baseline: None,
        });
    }
    let anchored: PathBuf;
    let path: &Path = if path.is_relative() {
        match std::env::current_dir() {
            Ok(cwd) => {
                anchored = cwd.join(path);
                &anchored
            }
            Err(_) => path,
        }
    } else {
        path
    };
    if let Ok(canonical) = path.canonicalize() {
        return Ok(canonical);
    }
    let mut remainder: Vec<std::ffi::OsString> = Vec::new();
    let mut cursor = path;
    loop {
        let parent = cursor.parent();
        let name = cursor.file_name();
        let (Some(parent), Some(name)) = (parent, name) else {
            return Err(ApproveError::InvalidStorePath {
                detail: format!(
                    "baseline root {} has no resolvable ancestor",
                    path.display()
                ),
                observed_baseline: None,
            });
        };
        if name == ".." {
            return Err(ApproveError::InvalidStorePath {
                detail: format!(
                    "baseline root {} carries an unresolvable `..`",
                    path.display()
                ),
                observed_baseline: None,
            });
        }
        remainder.push(name.to_os_string());
        if let Ok(canonical) = parent.canonicalize() {
            let mut resolved = canonical;
            for component in remainder.iter().rev() {
                resolved.push(component);
            }
            return Ok(resolved);
        }
        cursor = parent;
    }
}

fn ensure_no_symlink(root: &Path, relative: &Path) -> Result<(), ApproveError> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        let meta = match fs::symlink_metadata(&current) {
            Ok(meta) => meta,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(source) => {
                return Err(ApproveError::Io {
                    source,
                    observed_baseline: None,
                });
            }
        };
        if meta.file_type().is_symlink() {
            return Err(ApproveError::InvalidStorePath {
                detail: format!(
                    "store-relative path {} crosses a symlink",
                    relative.display()
                ),
                observed_baseline: None,
            });
        }
    }
    Ok(())
}

struct TempFile {
    path: Option<PathBuf>,
}

impl TempFile {
    fn create(dir: &Path, name: &str) -> Result<(fs::File, Self), std::io::Error> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = dir.join(format!(".tmp-accept-{}-{id}-{name}", std::process::id()));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let file = options.open(&path)?;
        Ok((file, Self { path: Some(path) }))
    }

    fn commit(mut self, target: &Path) -> Result<(), std::io::Error> {
        let Some(path) = self.path.take() else {
            return Err(std::io::Error::other(
                "temporary approval file is already committed",
            ));
        };
        match fs::rename(&path, target) {
            Ok(()) => Ok(()),
            Err(error) => {
                drop(fs::remove_file(&path));
                Err(error)
            }
        }
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            drop(fs::remove_file(path));
        }
    }
}

struct ChangeSet {
    reason: Option<MismatchReason>,
    fields: Vec<String>,
}

impl ChangeSet {
    fn empty() -> Self {
        Self {
            reason: None,
            fields: Vec::new(),
        }
    }

    fn missing() -> Self {
        Self {
            reason: Some(MismatchReason::Missing),
            fields: vec!["baseline".to_string()],
        }
    }

    fn from_present(comparison: PresentComparison) -> Self {
        match comparison {
            PresentComparison::Pass => Self::empty(),
            PresentComparison::Failure {
                reason,
                changed_fields,
            } => Self {
                reason: Some(reason),
                fields: changed_fields,
            },
        }
    }
}
