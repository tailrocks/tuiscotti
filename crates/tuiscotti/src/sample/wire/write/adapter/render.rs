use base64::Engine as _;

use super::super::super::super::{
    ContractFont, CursorPhase, ImageFormat, Observation, RenderContract, SampleError, SampleImage,
};
use super::super::super::generated::{
    CursorPhase as WireCursorPhase, EncodeFont, EncodeImage, EncodeObservation,
    EncodeRenderContract, ImageFormat as WireImageFormat,
};

pub(in crate::sample::wire) struct RenderContractAdapter<'a>(pub(super) &'a RenderContract);
impl EncodeRenderContract for RenderContractAdapter<'_> {
    fn profile(&self) -> &str {
        self.0.profile()
    }
    fn contract_version(&self) -> u32 {
        self.0.contract_version()
    }
    fn renderer_version(&self) -> &str {
        self.0.renderer_version()
    }
    fn raster_policy(&self) -> &str {
        self.0.raster_policy()
    }
    type Fonts<'a>
        =
        std::iter::Map<std::slice::Iter<'a, ContractFont>, fn(&'a ContractFont) -> FontAdapter<'a>>
    where
        Self: 'a;
    type FontsItem<'a>
        = FontAdapter<'a>
    where
        Self: 'a;
    fn fonts(&self) -> Self::Fonts<'_> {
        fn adapt(font: &ContractFont) -> FontAdapter<'_> {
            FontAdapter(font)
        }
        self.0.fonts().iter().map(adapt)
    }
    fn cell_width(&self) -> f32 {
        self.0.cell_width()
    }
    fn cell_height(&self) -> f32 {
        self.0.cell_height()
    }
    fn font_size(&self) -> f32 {
        self.0.font_size()
    }
    fn baseline(&self) -> f32 {
        self.0.baseline()
    }
    fn scale(&self) -> f64 {
        self.0.scale()
    }
    fn title_bar(&self) -> bool {
        self.0.title_bar()
    }
    fn palette_digest(&self) -> &str {
        self.0.palette_digest()
    }
    fn cursor_phase(&self) -> WireCursorPhase {
        match self.0.cursor_phase() {
            CursorPhase::Shown => WireCursorPhase::Shown,
            CursorPhase::Hidden => WireCursorPhase::Hidden,
        }
    }
    fn blink(&self) -> Option<bool> {
        self.0.blink()
    }
    fn platform_key(&self) -> &str {
        self.0.platform_key()
    }
}

const _: () = assert!(usize::BITS <= u64::BITS);

pub(in crate::sample::wire) struct FontAdapter<'a>(&'a ContractFont);
impl EncodeFont for FontAdapter<'_> {
    fn digest(&self) -> &str {
        self.0.digest()
    }
    fn face_index(&self) -> u32 {
        self.0.face_index()
    }
    fn bold(&self) -> bool {
        self.0.bold()
    }
    fn italic(&self) -> bool {
        self.0.italic()
    }
    fn fallback_position(&self) -> u64 {
        u64::try_from(self.0.fallback_position()).unwrap_or(u64::MAX)
    }
}

pub(super) struct DecodedImage<'a> {
    pub(super) image: &'a SampleImage,
    pub(super) png: Vec<u8>,
}

pub(in crate::sample::wire) struct ImageAdapter<'a> {
    pub(super) image: &'a SampleImage,
    pub(super) png: &'a [u8],
}

impl<'a> DecodedImage<'a> {
    pub(super) fn new(image: &'a SampleImage) -> Result<Self, SampleError> {
        let encoded = image.png_base64();
        let padding = encoded
            .as_bytes()
            .iter()
            .rev()
            .take_while(|byte| **byte == b'=')
            .count();
        let decoded_len = encoded
            .len()
            .checked_div(4)
            .and_then(|groups| groups.checked_mul(3))
            .and_then(|bytes| bytes.checked_sub(padding))
            .ok_or(SampleError::EncodeInvariant {
                detail: "invalid base64 length",
            })?;
        let mut png = Vec::new();
        png.try_reserve_exact(decoded_len)
            .map_err(|source| SampleError::EncodeAllocation { source })?;
        png.resize(decoded_len, 0);
        let written = base64::engine::general_purpose::STANDARD
            .decode_slice(encoded, &mut png)
            .map_err(|error| match error {
                base64::DecodeSliceError::DecodeError(source) => {
                    SampleError::Base64Invalid { source }
                }
                base64::DecodeSliceError::OutputSliceTooSmall => SampleError::EncodeInvariant {
                    detail: "base64 length calculation disagreed with decoder",
                },
            })?;
        if written != decoded_len {
            return Err(SampleError::EncodeInvariant {
                detail: "base64 decoder wrote an unexpected length",
            });
        }
        Ok(Self { image, png })
    }
}
impl EncodeImage for ImageAdapter<'_> {
    fn format(&self) -> WireImageFormat {
        match self.image.format() {
            ImageFormat::Png => WireImageFormat::Png,
        }
    }
    fn width(&self) -> u32 {
        self.image.width()
    }
    fn height(&self) -> u32 {
        self.image.height()
    }
    fn png_sha256(&self) -> &str {
        self.image.png_sha256()
    }
    fn rgba_sha256(&self) -> &str {
        self.image.rgba_sha256()
    }
    fn png(&self) -> &[u8] {
        self.png
    }
}

pub(in crate::sample::wire) struct ObservationAdapter<'a>(pub(super) &'a Observation);
impl EncodeObservation for ObservationAdapter<'_> {
    fn note(&self) -> Option<&str> {
        self.0.note()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_adapter_preserves_exact_png_bytes() {
        let png = [137, 80, 78, 71, 13, 10, 26, 10];
        let image = SampleImage {
            format: ImageFormat::Png,
            width: 1,
            height: 1,
            png_sha256: "p".to_owned(),
            rgba_sha256: "r".to_owned(),
            png_base64: base64::engine::general_purpose::STANDARD.encode(png),
        };
        let decoded = DecodedImage::new(&image).expect("valid base64 fixture");
        assert_eq!(decoded.png, png);
    }
}
