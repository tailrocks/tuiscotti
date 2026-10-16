//! Decoded-pixel comparison engine (strict + perceptual diagnostic).

use super::{AlphaPolicy, DiffError, PixelVerdict};
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder, RgbImage, RgbaImage};

fn decode_png(bytes: &[u8], label: &str) -> Result<RgbaImage, DiffError> {
    image::load_from_memory(bytes)
        .map_err(|e| DiffError(format!("cannot decode {label} PNG: {e}")))
        .map(|d| d.to_rgba8())
}

fn pixels_equal_under(expected: &RgbaImage, actual: &RgbaImage, policy: AlphaPolicy) -> bool {
    if expected.dimensions() != actual.dimensions() {
        return false;
    }
    match policy {
        AlphaPolicy::StraightRgba => expected.as_raw() == actual.as_raw(),
        AlphaPolicy::Opaque => expected
            .as_raw()
            .as_chunks::<4>()
            .0
            .iter()
            .zip(actual.as_raw().as_chunks::<4>().0)
            .all(|(e, a)| e[3] == 255 && a[3] == 255 && e[..3] == a[..3]),
    }
}

fn rgb_channels_equal(expected: &RgbaImage, actual: &RgbaImage) -> bool {
    expected
        .as_raw()
        .as_chunks::<4>()
        .0
        .iter()
        .zip(actual.as_raw().as_chunks::<4>().0)
        .all(|(e, a)| e[..3] == a[..3])
}

/// Downsample large RGBA pairs to RGB for the diagnostic `rgb_hybrid_compare`
/// pass. Each `step × step` tile keeps the pixel pair with the largest L1 RGB
/// delta in that tile so single-pixel differences never average out to zero.
/// Small images (`w * h <= 320 * 240`) stay at `step = 1` (1:1 copy).
fn diagnostic_rgb_pair(expected: &RgbaImage, actual: &RgbaImage) -> (RgbImage, RgbImage) {
    let (w, h) = (expected.width() as usize, expected.height() as usize);
    let total = w * h;
    let target = 160 * 100;
    let step = if total <= 320 * 240 {
        1
    } else {
        (total / target).isqrt().max(2)
    };
    let ea = expected.as_raw();
    let eb = actual.as_raw();
    if step == 1 {
        let mut out_e = vec![0u8; total * 3];
        let mut out_a = vec![0u8; total * 3];
        for ((se, sa), (de, da)) in ea.as_chunks::<4>().0.iter().zip(eb.as_chunks::<4>().0).zip(
            out_e
                .as_chunks_mut::<3>()
                .0
                .iter_mut()
                .zip(out_a.as_chunks_mut::<3>().0),
        ) {
            de.copy_from_slice(&se[..3]);
            da.copy_from_slice(&sa[..3]);
        }
        let img_e = RgbImage::from_raw(expected.width(), expected.height(), out_e)
            .unwrap_or_else(|| RgbImage::new(expected.width(), expected.height()));
        let img_a = RgbImage::from_raw(actual.width(), actual.height(), out_a)
            .unwrap_or_else(|| RgbImage::new(actual.width(), actual.height()));
        return (img_e, img_a);
    }
    let small_w = w.div_ceil(step);
    let small_h = h.div_ceil(step);
    let mut out_e = vec![0u8; small_w * small_h * 3];
    let mut out_a = vec![0u8; small_w * small_h * 3];
    for ty in 0..small_h {
        let y0 = ty * step;
        let y1 = (y0 + step).min(h);
        for tx in 0..small_w {
            let x0 = tx * step;
            let x1 = (x0 + step).min(w);
            let first = (y0 * w + x0) * 4;
            let mut best_e = [ea[first], ea[first + 1], ea[first + 2]];
            let mut best_a = [eb[first], eb[first + 1], eb[first + 2]];
            let mut best_delta = 0u16;
            for py in y0..y1 {
                let row_start = (py * w + x0) * 4;
                let row_end = (py * w + x1) * 4;
                if ea[row_start..row_end] == eb[row_start..row_end] {
                    continue;
                }
                for (se, sa) in ea[row_start..row_end]
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(eb[row_start..row_end].as_chunks::<4>().0)
                {
                    let delta = u16::from(se[0].abs_diff(sa[0]))
                        + u16::from(se[1].abs_diff(sa[1]))
                        + u16::from(se[2].abs_diff(sa[2]));
                    if delta > best_delta {
                        best_delta = delta;
                        best_e = [se[0], se[1], se[2]];
                        best_a = [sa[0], sa[1], sa[2]];
                    }
                }
            }
            let dst = (ty * small_w + tx) * 3;
            out_e[dst..dst + 3].copy_from_slice(&best_e);
            out_a[dst..dst + 3].copy_from_slice(&best_a);
        }
    }
    let width_u32 = u32::try_from(small_w).unwrap_or(u32::MAX);
    let height_u32 = u32::try_from(small_h).unwrap_or(u32::MAX);
    let img_e = RgbImage::from_raw(width_u32, height_u32, out_e)
        .unwrap_or_else(|| RgbImage::new(width_u32, height_u32));
    let img_a = RgbImage::from_raw(width_u32, height_u32, out_a)
        .unwrap_or_else(|| RgbImage::new(width_u32, height_u32));
    (img_e, img_a)
}

/// Hybrid structural+chroma similarity score over the RGB channels.
/// Diagnostic material only: the score must never establish strict equality —
/// callers cap it below 1.0 for non-identical pixels.
fn hybrid_score(expected: &RgbaImage, actual: &RgbaImage) -> Result<f64, DiffError> {
    if rgb_channels_equal(expected, actual) {
        return Ok(1.0);
    }
    let (expected_rgb, actual_rgb) = diagnostic_rgb_pair(expected, actual);
    let result = image_compare::rgb_hybrid_compare(&expected_rgb, &actual_rgb)
        .map_err(|e| DiffError(format!("comparison failed: {e}")))?;
    Ok(result.score)
}

/// Build a full-resolution red-overlay diff PNG directly in one slice pass
/// and encode with fast PNG settings (`CompressionType::Fast`, `FilterType::Sub`).
#[expect(
    clippy::cast_possible_truncation,
    reason = "sum of three u8s divided by 9 is at most 85, which fits in u8"
)]
fn encode_diff_png(
    expected: &RgbaImage,
    actual: &RgbaImage,
    policy: AlphaPolicy,
) -> Result<Vec<u8>, DiffError> {
    let (w, h) = (expected.width(), expected.height());
    let total = (w as usize) * (h as usize);
    let mut raw = vec![0u8; total * 3];
    for ((e, a), dst) in expected
        .as_raw()
        .as_chunks::<4>()
        .0
        .iter()
        .zip(actual.as_raw().as_chunks::<4>().0)
        .zip(raw.as_chunks_mut::<3>().0)
    {
        let same = match policy {
            AlphaPolicy::StraightRgba => e == a,
            AlphaPolicy::Opaque => e[3] == 255 && a[3] == 255 && e[..3] == a[..3],
        };
        if same {
            let luma = ((u16::from(e[0]) + u16::from(e[1]) + u16::from(e[2])) / 9) as u8;
            *dst = [luma, luma, luma];
        } else {
            *dst = [255, 32, 64];
        }
    }
    let mut diff_png = Vec::with_capacity(total / 4 + 1024);
    PngEncoder::new_with_quality(&mut diff_png, CompressionType::Fast, FilterType::Sub)
        .write_image(&raw, w, h, ExtendedColorType::Rgb8)
        .map_err(|e| DiffError(format!("diff PNG encode: {e}")))?;
    Ok(diff_png)
}

/// Strict comparison over decoded pixels with the default
/// [`AlphaPolicy::StraightRgba`]. Never compares compressed bytes:
/// re-encoding identical pixels passes. Takes no threshold.
///
/// # Errors
///
/// Returns `DiffError` when either input does not decode as a PNG.
pub fn compare_png(expected_png: &[u8], actual_png: &[u8]) -> Result<PixelVerdict, DiffError> {
    compare_png_with_alpha(expected_png, actual_png, AlphaPolicy::default())
}

/// Strict comparison over decoded pixels with an explicit [`AlphaPolicy`].
/// Equality is equal dimensions plus exact decoded-pixel identity — no
/// threshold, no perceptual rounding.
///
/// # Errors
///
/// Returns `DiffError` when either input does not decode as a PNG.
pub fn compare_png_with_alpha(
    expected_png: &[u8],
    actual_png: &[u8],
    alpha_policy: AlphaPolicy,
) -> Result<PixelVerdict, DiffError> {
    // Sound fast path: identical compressed bytes decode to identical
    // pixels. Still decodes once so corrupt-but-identical bytes error
    // instead of passing.
    if expected_png == actual_png {
        let decoded = decode_png(expected_png, "expected")?;
        let dims = (decoded.width(), decoded.height());
        let equal = match alpha_policy {
            AlphaPolicy::StraightRgba => true,
            AlphaPolicy::Opaque => decoded
                .as_raw()
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| p[3] == 255),
        };
        if equal {
            return Ok(PixelVerdict {
                dims_equal: true,
                expected_dims: dims,
                actual_dims: dims,
                pixels_equal: true,
                alpha_policy,
                score: 1.0,
                diff_png: Vec::new(),
            });
        }
    }
    let expected = decode_png(expected_png, "expected")?;
    let actual = decode_png(actual_png, "actual")?;
    let expected_dims = (expected.width(), expected.height());
    let actual_dims = (actual.width(), actual.height());
    if expected_dims != actual_dims {
        return Ok(PixelVerdict {
            dims_equal: false,
            expected_dims,
            actual_dims,
            pixels_equal: false,
            alpha_policy,
            score: 0.0,
            diff_png: Vec::new(),
        });
    }
    if pixels_equal_under(&expected, &actual, alpha_policy) {
        return Ok(PixelVerdict {
            dims_equal: true,
            expected_dims,
            actual_dims,
            pixels_equal: true,
            alpha_policy,
            score: 1.0,
            diff_png: Vec::new(),
        });
    }
    let raw_score = hybrid_score(&expected, &actual)?;
    let diff_png = encode_diff_png(&expected, &actual, alpha_policy)?;
    // A perceptual value must never read as equality (C04): identical RGB
    // with differing alpha scores a hybrid 1.0, so clamp below 1.0 here.
    // `score >= 1.0` therefore implies decoded-pixel identity.
    let score = raw_score.min(1.0 - f64::EPSILON);
    Ok(PixelVerdict {
        dims_equal: true,
        expected_dims,
        actual_dims,
        pixels_equal: false,
        alpha_policy,
        score,
        diff_png,
    })
}

/// Diagnostic-only perceptual similarity in `[0.0, 1.0]` (hybrid
/// structural+chroma over RGB). NEVER establishes strict equality — not
/// even at 1.0: alpha-only differences are invisible to it, and rounding
/// can hide small changes. Only [`PixelVerdict::pixels_equal`] proves pixel
/// identity. Returns 0.0 when dimensions differ (no pixel correspondence
/// exists).
///
/// # Errors
///
/// Returns `DiffError` when either input does not decode as a PNG.
pub fn perceptual_score(expected_png: &[u8], actual_png: &[u8]) -> Result<f64, DiffError> {
    let expected = decode_png(expected_png, "expected")?;
    let actual = decode_png(actual_png, "actual")?;
    if expected.dimensions() != actual.dimensions() {
        return Ok(0.0);
    }
    if expected.as_raw() == actual.as_raw() {
        return Ok(1.0);
    }
    hybrid_score(&expected, &actual)
}
