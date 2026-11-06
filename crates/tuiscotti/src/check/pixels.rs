use tui_test::pixel::compare_rgba;

use crate::sample::{
    MAX_IMAGE_DIM, MAX_RGBA_BYTES, Sample, SampleError, decode_png_with_budget,
    png_decode::{DynamicBudget, DynamicCharge, PngBudget, PngPixels, decode_base64},
};

use super::CheckError;

pub(super) struct DiffPng {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) pixels: Vec<u8>,
}

pub(super) enum PixelVerdict {
    Equal,
    Dimensions,
    Different {
        changed: u64,
        compared: u64,
        bounds: Option<[u32; 4]>,
        diff: DiffPng,
    },
}

pub(super) fn decide_pixels(
    actual: &Sample,
    baseline: &Sample,
) -> Result<PixelVerdict, CheckError> {
    let mut budget = DynamicBudget::bounded()?;
    decide_pixels_with_budget(actual, baseline, &mut budget)
}

fn decide_pixels_with_budget(
    actual: &Sample,
    baseline: &Sample,
    budget: &mut DynamicBudget,
) -> Result<PixelVerdict, CheckError> {
    let actual_image = actual.image().ok_or_else(|| SampleError::GateMismatch {
        detail: "state-and-pixels gate requires an image".to_string(),
    })?;
    let baseline_image = baseline.image().ok_or_else(|| SampleError::GateMismatch {
        detail: "state-and-pixels baseline requires an image".to_string(),
    })?;
    let actual_pixels = decode_sample_png(
        actual_image.png_base64(),
        actual_image.width(),
        actual_image.height(),
        budget,
    )?;
    let baseline_pixels = decode_sample_png(
        baseline_image.png_base64(),
        baseline_image.width(),
        baseline_image.height(),
        budget,
    )?;
    if (actual_pixels.width, actual_pixels.height)
        != (baseline_pixels.width, baseline_pixels.height)
    {
        release_pixels(budget, actual_pixels)?;
        release_pixels(budget, baseline_pixels)?;
        return Ok(PixelVerdict::Dimensions);
    }
    let (width, height) = (actual_pixels.width, actual_pixels.height);
    let diff_bytes = byte_total(actual_pixels.width, actual_pixels.height)?;
    let diff_charge = budget.charge(
        "PNG comparison difference output",
        u64::try_from(diff_bytes).map_err(|_| SampleError::ResourceLimit {
            resource: "PNG comparison difference output",
            claimed: None,
            limit: MAX_RGBA_BYTES,
            source: None,
        })?,
    )?;
    let comparison = compare_rgba(
        width,
        height,
        &actual_pixels.rgba,
        baseline_pixels.width,
        baseline_pixels.height,
        &baseline_pixels.rgba,
    )?;
    if comparison.is_equal() {
        budget.release(diff_charge)?;
        release_pixels(budget, actual_pixels)?;
        release_pixels(budget, baseline_pixels)?;
        return Ok(PixelVerdict::Equal);
    }
    let bounds = comparison
        .bounds
        .map(|found| [found.min_x, found.min_y, found.max_x, found.max_y]);
    let Some(difference_image) = comparison.difference_image else {
        return Err(SampleError::MissingDifferenceImage {
            detail: "upstream pixel comparison reports changed pixels without a difference image"
                .to_string(),
        }
        .into());
    };
    release_pixels(budget, actual_pixels)?;
    release_pixels(budget, baseline_pixels)?;
    let _ = diff_charge;
    Ok(PixelVerdict::Different {
        changed: comparison.changed_pixels,
        compared: comparison.compared_pixels,
        bounds,
        diff: DiffPng {
            width,
            height,
            pixels: difference_image,
        },
    })
}

fn decode_sample_png(
    base64_png: &str,
    claimed_width: u32,
    claimed_height: u32,
    budget: &mut DynamicBudget,
) -> Result<PngPixels<DynamicCharge>, CheckError> {
    check_png_dims(claimed_width, claimed_height)?;
    let (raw, raw_charge) = decode_base64(base64_png, budget)?;
    let pixels = decode_png_with_budget(&raw, budget).map_err(|error| match error {
        SampleError::PngColor { .. } => SampleError::PngInvalid {
            detail: "check PNG is not 8-bit RGBA".to_string(),
        },
        error => error,
    })?;
    drop(raw);
    budget.release(raw_charge)?;
    if (pixels.width, pixels.height) != (claimed_width, claimed_height) {
        return Err(SampleError::DimensionMismatch {
            detail: format!(
                "check image claims {claimed_width}x{claimed_height} but decodes to {}x{}",
                pixels.width, pixels.height
            ),
        }
        .into());
    }
    check_png_dims(pixels.width, pixels.height)?;
    Ok(pixels)
}

fn release_pixels(
    budget: &mut DynamicBudget,
    pixels: PngPixels<DynamicCharge>,
) -> Result<(), CheckError> {
    drop(pixels.rgba);
    budget.release(pixels.charge)?;
    Ok(())
}

fn check_png_dims(width: u32, height: u32) -> Result<(), CheckError> {
    if width == 0 || height == 0 {
        return Err(SampleError::DimensionMismatch {
            detail: "check image dimensions are zero".to_string(),
        }
        .into());
    }
    if width > MAX_IMAGE_DIM || height > MAX_IMAGE_DIM {
        return Err(SampleError::DimensionTooLarge {
            width,
            height,
            limit: MAX_IMAGE_DIM,
        }
        .into());
    }
    let bytes = byte_total(width, height)?;
    let bytes_u64 = u64::try_from(bytes).map_err(|_| SampleError::RgbaTooLarge {
        bytes: u64::MAX,
        limit: MAX_RGBA_BYTES,
    })?;
    if bytes_u64 > MAX_RGBA_BYTES {
        return Err(SampleError::RgbaTooLarge {
            bytes: bytes_u64,
            limit: MAX_RGBA_BYTES,
        }
        .into());
    }
    Ok(())
}

fn byte_total(width: u32, height: u32) -> Result<usize, CheckError> {
    u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or(SampleError::RgbaTooLarge {
            bytes: u64::MAX,
            limit: MAX_RGBA_BYTES,
        })
        .map_err(CheckError::from)
}

pub(super) fn encode_diff_png(diff: &DiffPng) -> Result<Vec<u8>, CheckError> {
    use image::ImageEncoder as _;
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(
            &diff.pixels,
            diff.width,
            diff.height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|source| CheckError::Image { source })?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_profile::{PROFILE_NAME, resolve_profile};
    use crate::sample::{Observation, png_decode::DynamicBudget};
    use crate::view::{ViewSettings, capture_view};
    use std::error::Error;

    #[test]
    fn pixel_comparison_accounts_for_both_live_rgba_outputs() -> Result<(), Box<dyn Error>> {
        let snapshot = capture_view(12, 3, &ViewSettings::builtin(), |frame| {
            frame.buffer_mut()[(0, 0)].set_char('X');
        })?;
        let profile = resolve_profile(PROFILE_NAME)?;
        let test_id = format!(
            "check/budget/pixels-{}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
        let sample = Sample::with_pixels(&test_id, &snapshot, Observation::empty(), profile)?;
        let image = sample.image().ok_or("pixel sample is missing image")?;
        let rgba_bytes = u64::from(image.width()) * u64::from(image.height()) * 4;
        let mut budget = DynamicBudget::bounded()?;

        let verdict = decide_pixels_with_budget(&sample, &sample, &mut budget)?;

        assert!(matches!(verdict, PixelVerdict::Equal));
        assert!(budget.peak() >= rgba_bytes * 2);
        Ok(())
    }
}
