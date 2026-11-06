use base64::Engine as _;
use sha2::{Digest, Sha256};

use super::super::super::budget::{DecodeBudget, Phase};
use super::super::super::decode_error::DecodeError;
use super::super::super::png_decode::decode_png;
use super::super::super::{ImageFormat as DomainImageFormat, SampleError, SampleImage};
use super::super::generated::{DecodeImage, ImageFormat, ValidatedImage};
use super::allocation::{allocation, copied_string, hex, matches_hex};

#[derive(Default)]
struct ImageFields<'a> {
    format: Option<DomainImageFormat>,
    width: Option<u32>,
    height: Option<u32>,
    png_sha256: Option<&'a str>,
    rgba_sha256: Option<&'a str>,
    png: Option<&'a [u8]>,
}
impl<'a> DecodeImage<'a> for ImageFields<'a> {
    type Error = DecodeError;
    fn format(&mut self, value: ImageFormat) -> Result<(), Self::Error> {
        self.format = Some(match value {
            ImageFormat::Png => DomainImageFormat::Png,
        });
        Ok(())
    }
    fn width(&mut self, value: u32) -> Result<(), Self::Error> {
        self.width = Some(value);
        Ok(())
    }
    fn height(&mut self, value: u32) -> Result<(), Self::Error> {
        self.height = Some(value);
        Ok(())
    }
    fn png_sha256(&mut self, value: &'a str) -> Result<(), Self::Error> {
        self.png_sha256 = Some(value);
        Ok(())
    }
    fn rgba_sha256(&mut self, value: &'a str) -> Result<(), Self::Error> {
        self.rgba_sha256 = Some(value);
        Ok(())
    }
    fn png(&mut self, value: &'a [u8]) -> Result<(), Self::Error> {
        self.png = Some(value);
        Ok(())
    }
}

pub(super) fn map_image(
    value: ValidatedImage<'_>,
    budget: &mut DecodeBudget,
) -> Result<SampleImage, DecodeError> {
    let mut fields = ImageFields::default();
    value.visit(&mut fields)?;
    let png = take(fields.png, "validated image omitted PNG bytes")?;
    let expected_png = take(fields.png_sha256, "validated image omitted PNG digest")?;
    let expected_rgba = take(fields.rgba_sha256, "validated image omitted RGBA digest")?;
    let width = take(fields.width, "validated image omitted width")?;
    let height = take(fields.height, "validated image omitted height")?;
    let format = take(fields.format, "validated image omitted format")?;
    let actual_png = Sha256::digest(png);
    if !matches_hex(&actual_png, expected_png) {
        return Err(DecodeError::Domain(SampleError::DigestMismatch {
            field: "png_sha256",
            expected: copied_string(expected_png, "expected PNG digest")?,
            actual: hex(&actual_png)?,
        }));
    }

    budget.set_phase(Phase::Image)?;
    let decoded = decode_png(png, budget).map_err(DecodeError::Domain)?;
    if (decoded.width, decoded.height) != (width, height) {
        drop(decoded.rgba);
        budget.release(decoded.charge)?;
        return Err(DecodeError::Domain(SampleError::DimensionMismatch {
            detail: "sample claims dimensions different from its PNG".to_string(),
        }));
    }
    let digest = Sha256::digest(&decoded.rgba);
    drop(decoded.rgba);
    budget.release(decoded.charge)?;
    if !matches_hex(&digest, expected_rgba) {
        return Err(DecodeError::Domain(SampleError::DigestMismatch {
            field: "rgba_sha256",
            expected: copied_string(expected_rgba, "expected RGBA digest")?,
            actual: hex(&digest)?,
        }));
    }
    let encoded_len = png
        .len()
        .checked_add(2)
        .and_then(|length| length.checked_div(3))
        .and_then(|groups| groups.checked_mul(4))
        .ok_or_else(|| DecodeError::resource("base64 image bytes", None, u64::MAX))?;
    let mut png_base64 = String::new();
    png_base64
        .try_reserve_exact(encoded_len)
        .map_err(|source| allocation("base64 image", encoded_len, source))?;
    base64::engine::general_purpose::STANDARD.encode_string(png, &mut png_base64);
    if png_base64.len() != encoded_len {
        return Err(DecodeError::Invariant {
            detail: "base64 encoded length mismatch",
        });
    }
    Ok(SampleImage {
        format,
        width,
        height,
        png_sha256: copied_string(expected_png, "PNG digest")?,
        rgba_sha256: copied_string(expected_rgba, "RGBA digest")?,
        png_base64,
    })
}

fn take<T>(value: Option<T>, detail: &'static str) -> Result<T, DecodeError> {
    value.ok_or(DecodeError::Invariant { detail })
}
