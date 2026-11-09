use crate::render_profile::{ProfileError, resolve_profile};
use crate::sample::png_decode::{
    DynamicBudget, DynamicCharge, PngBudget, PngPixels, decode_base64, decode_png,
};
use crate::sample::{RenderContract, Sample, SampleError, SampleImage};
use tui_test::{PinnedFontError, RenderError, render_snapshot_png};

use super::digest::sha256_hex;
use super::errors::ApproveError;
use crate::check::CheckError;

pub(super) fn rerender_check(candidate: &Sample) -> Result<(), ApproveError> {
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
    let mut budget = DynamicBudget::bounded()?;
    let fresh = decode_rendered_png(rendered.png(), &mut budget)?;
    let stored = decode_stored_png(image, &mut budget)?;
    let consistent =
        fresh.width == stored.width && fresh.height == stored.height && fresh.rgba == stored.rgba;
    let dimensions = ((fresh.width, fresh.height), (stored.width, stored.height));
    drop(fresh.rgba);
    budget.release(fresh.charge)?;
    drop(stored.rgba);
    budget.release(stored.charge)?;
    if !consistent {
        return Err(ApproveError::InconsistentSample {
            detail: format!(
                "re-rendered {}x{} pixels differ from the candidate {}x{} pixels",
                dimensions.0.0, dimensions.0.1, dimensions.1.0, dimensions.1.1,
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

fn decode_rendered_png(
    png: &[u8],
    budget: &mut DynamicBudget,
) -> Result<PngPixels<DynamicCharge>, ApproveError> {
    decode_png(png, budget).map_err(|source| match source {
        SampleError::ResourceLimit { .. }
        | SampleError::PngResourceLimit { .. }
        | SampleError::DimensionTooLarge { .. }
        | SampleError::RgbaTooLarge { .. }
        | SampleError::Allocation { .. }
        | SampleError::DecodeInvariant { .. } => ApproveError::from(source),
        source => ApproveError::RenderFailed {
            source: RenderError::RasterFailed {
                message: format!("re-rendered PNG does not decode: {source}"),
            },
            observed_baseline: None,
        },
    })
}

fn decode_stored_png(
    image: &SampleImage,
    budget: &mut DynamicBudget,
) -> Result<PngPixels<DynamicCharge>, ApproveError> {
    let (raw, raw_charge) = decode_base64(image.png_base64(), budget)?;
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
    let decoded = decode_png(&raw, budget).map_err(|source| match source {
        SampleError::ResourceLimit { .. }
        | SampleError::PngResourceLimit { .. }
        | SampleError::DimensionTooLarge { .. }
        | SampleError::RgbaTooLarge { .. }
        | SampleError::Allocation { .. }
        | SampleError::DecodeInvariant { .. } => ApproveError::from(source),
        source => ApproveError::Check {
            source: CheckError::Sample(SampleError::PngInvalid {
                detail: source.to_string(),
            }),
            observed_baseline: None,
        },
    })?;
    drop(raw);
    budget.release(raw_charge)?;
    let dimensions_match = (decoded.width, decoded.height) == (image.width(), image.height());
    if !dimensions_match {
        return Err(ApproveError::Check {
            source: CheckError::Sample(SampleError::ImageDimensionsMismatch {
                claimed: (image.width(), image.height()),
                actual: (decoded.width, decoded.height),
            }),
            observed_baseline: None,
        });
    }
    Ok(decoded)
}
