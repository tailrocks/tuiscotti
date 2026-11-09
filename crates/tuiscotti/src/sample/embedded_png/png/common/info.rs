use super::*;
#[cfg(test)]
use crate::sample::embedded_png::png::encoder;
use crate::sample::embedded_png::png::text_metadata::{ITXtChunk, TEXtChunk, ZTXtChunk};
use std::borrow::Cow;
#[cfg(test)]
use std::io::Write;

/// PNG info struct
#[derive(Clone, Debug)]
#[non_exhaustive]
pub(crate) struct Info<'a> {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) bit_depth: BitDepth,
    /// How colors are stored in the image.
    pub(crate) color_type: ColorType,
    pub(crate) interlaced: bool,
    /// The image's `sBIT` chunk, if present; contains significant bits of the sample.
    pub(crate) sbit: Option<Cow<'a, [u8]>>,
    /// The image's `tRNS` chunk, if present; contains the alpha channel of the image's palette, 1 byte per entry.
    pub(crate) trns: Option<Cow<'a, [u8]>>,
    pub(crate) pixel_dims: Option<PixelDimensions>,
    /// The image's `PLTE` chunk, if present; contains the RGB channels (in that order) of the image's palettes, 3 bytes per entry (1 per channel).
    pub(crate) palette: Option<Cow<'a, [u8]>>,
    /// The contents of the image's gAMA chunk, if present.
    /// Prefer `source_gamma` to also get the derived replacement gamma from sRGB chunks.
    pub(crate) gama_chunk: Option<ScaledFloat>,
    /// The contents of the image's `cHRM` chunk, if present.
    /// Prefer `source_chromaticities` to also get the derived replacements from sRGB chunks.
    pub(crate) chrm_chunk: Option<SourceChromaticities>,
    /// The contents of the image's `bKGD` chunk, if present.
    pub(crate) bkgd: Option<Cow<'a, [u8]>>,

    pub(crate) frame_control: Option<FrameControl>,
    pub(crate) animation_control: Option<AnimationControl>,
    /// Gamma of the source system.
    /// Set by both `gAMA` as well as to a replacement by `sRGB` chunk.
    pub(crate) source_gamma: Option<ScaledFloat>,
    /// Chromaticities of the source system.
    /// Set by both `cHRM` as well as to a replacement by `sRGB` chunk.
    pub(crate) source_chromaticities: Option<SourceChromaticities>,
    /// The rendering intent of an SRGB image.
    ///
    /// Presence of this value also indicates that the image conforms to the SRGB color space.
    pub(crate) srgb: Option<SrgbRenderingIntent>,
    /// The ICC profile for the image.
    pub(crate) icc_profile: Option<Cow<'a, [u8]>>,
    /// The coding-independent code points for video signal type identification of the image.
    pub(crate) coding_independent_code_points: Option<CodingIndependentCodePoints>,
    /// The mastering display color volume for the image.
    pub(crate) mastering_display_color_volume: Option<MasteringDisplayColorVolume>,
    /// The content light information for the image.
    pub(crate) content_light_level: Option<ContentLightLevelInfo>,
    /// The EXIF metadata for the image.
    pub(crate) exif_metadata: Option<Cow<'a, [u8]>>,
    /// tEXt field
    pub(crate) uncompressed_latin1_text: Vec<TEXtChunk>,
    /// zTXt field
    pub(crate) compressed_latin1_text: Vec<ZTXtChunk>,
    /// iTXt field
    pub(crate) utf8_text: Vec<ITXtChunk>,
}

impl Default for Info<'_> {
    fn default() -> Info<'static> {
        Info {
            width: 0,
            height: 0,
            bit_depth: BitDepth::Eight,
            color_type: ColorType::Grayscale,
            interlaced: false,
            palette: None,
            sbit: None,
            trns: None,
            gama_chunk: None,
            chrm_chunk: None,
            bkgd: None,
            pixel_dims: None,
            frame_control: None,
            animation_control: None,
            source_gamma: None,
            source_chromaticities: None,
            srgb: None,
            icc_profile: None,
            coding_independent_code_points: None,
            mastering_display_color_volume: None,
            content_light_level: None,
            exif_metadata: None,
            uncompressed_latin1_text: Vec::new(),
            compressed_latin1_text: Vec::new(),
            utf8_text: Vec::new(),
        }
    }
}

impl Info<'_> {
    /// A utility constructor for a default info with width and height.
    pub(crate) fn with_size(width: u32, height: u32) -> Self {
        Info {
            width,
            height,
            ..Default::default()
        }
    }

    /// Size of the image, width then height.
    pub(crate) fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns true if the image is an APNG image.
    pub(crate) fn is_animated(&self) -> bool {
        self.frame_control.is_some() && self.animation_control.is_some()
    }

    /// Returns the frame control information of the image.
    pub(crate) fn animation_control(&self) -> Option<&AnimationControl> {
        self.animation_control.as_ref()
    }

    /// Returns the frame control information of the current frame
    pub(crate) fn frame_control(&self) -> Option<&FrameControl> {
        self.frame_control.as_ref()
    }

    /// Returns the number of bits per pixel.
    pub(crate) fn bits_per_pixel(&self) -> usize {
        self.color_type.bits_per_pixel(self.bit_depth)
    }

    /// Returns the number of bytes per pixel.
    pub(crate) fn bytes_per_pixel(&self) -> usize {
        // If adjusting this for expansion or other transformation passes, remember to keep the old
        // implementation for bpp_in_prediction, which is internal to the png specification.
        self.color_type.bytes_per_pixel(self.bit_depth)
    }

    /// Return the number of bytes for this pixel used in prediction.
    ///
    /// Some filters use prediction, over the raw bytes of a scanline. Where a previous pixel is
    /// require for such forms the specification instead references previous bytes. That is, for
    /// a gray pixel of bit depth 2, the pixel used in prediction is actually 4 pixels prior. This
    /// has the consequence that the number of possible values is rather small. To make this fact
    /// more obvious in the type system and the optimizer we use an explicit enum here.
    pub(crate) fn bpp_in_prediction(&self) -> BytesPerPixel {
        BytesPerPixel::from_usize(self.bytes_per_pixel())
    }

    /// Returns the number of bytes needed for one deinterlaced image.
    pub(crate) fn raw_bytes(&self) -> usize {
        self.height as usize * self.raw_row_length()
    }

    /// Returns the number of bytes needed for one deinterlaced row.
    pub(crate) fn raw_row_length(&self) -> usize {
        self.raw_row_length_from_width(self.width)
    }

    pub(crate) fn checked_raw_row_length(&self) -> Option<usize> {
        self.color_type
            .checked_raw_row_length(self.bit_depth, self.width)
    }

    /// Returns the number of bytes needed for one deinterlaced row of width `width`.
    pub(crate) fn raw_row_length_from_width(&self, width: u32) -> usize {
        self.color_type
            .raw_row_length_from_width(self.bit_depth, width)
    }

    /// Gamma dependent on sRGB chunk
    pub(crate) fn gamma(&self) -> Option<ScaledFloat> {
        if self.srgb.is_some() {
            Some(crate::sample::embedded_png::png::srgb::substitute_gamma())
        } else {
            self.gama_chunk
        }
    }

    /// Chromaticities dependent on sRGB chunk
    pub(crate) fn chromaticities(&self) -> Option<SourceChromaticities> {
        if self.srgb.is_some() {
            Some(crate::sample::embedded_png::png::srgb::substitute_chromaticities())
        } else {
            self.chrm_chunk
        }
    }

    /// Mark the image data as conforming to the SRGB color space with the specified rendering intent.
    ///
    /// Any ICC profiles will be ignored.
    ///
    /// Source gamma and chromaticities will be written only if they're set to fallback
    /// values specified in [11.3.2.5](https://www.w3.org/TR/png-3/#sRGB-gAMA-cHRM).
    pub(crate) fn set_source_srgb(&mut self, rendering_intent: SrgbRenderingIntent) {
        self.srgb = Some(rendering_intent);
        self.icc_profile = None;
    }
}
