use super::SampleError;
use super::png_decode::{DynamicBudget, PngBudget, PngPixels, decode_png};

pub(crate) fn decode_png_with_budget<B: PngBudget>(
    png: &[u8],
    budget: &mut B,
) -> Result<PngPixels<B::Token>, SampleError> {
    decode_png(png, budget).map_err(|error| match error {
        SampleError::PngCodec { source } => SampleError::PngDecode {
            source: image::ImageError::Decoding(image::error::DecodingError::new(
                image::error::ImageFormatHint::Exact(image::ImageFormat::Png),
                source,
            )),
        },
        error => error,
    })
}

pub(crate) fn decode_png_bounded(png: &[u8]) -> Result<(u32, u32, Vec<u8>), SampleError> {
    let mut budget = DynamicBudget::bounded()?;
    let pixels = decode_png_with_budget(png, &mut budget)?;
    Ok((pixels.width, pixels.height, pixels.rgba))
}
