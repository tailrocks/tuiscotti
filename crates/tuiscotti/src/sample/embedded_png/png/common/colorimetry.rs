use super::*;
use crate::sample::embedded_png::png::chunk;
#[cfg(test)]
use crate::sample::embedded_png::png::encoder;
#[cfg(test)]
use std::io::Write;

/// An unsigned integer scaled version of a floating point value,
/// equivalent to an integer quotient with [fixed denominator][ScaledFloat::SCALING]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScaledFloat(u32);

impl ScaledFloat {
    pub(crate) const SCALING: f32 = 100_000.0;

    /// Gets whether the value is within the clamped range of this type.
    pub(crate) fn in_range(value: f32) -> bool {
        value >= 0.0 && (value * Self::SCALING).floor() <= u32::MAX as f32
    }

    /// Gets whether the value can be exactly converted in round-trip.
    pub(crate) fn exact(value: f32) -> bool {
        let there = Self::forward(value);
        let back = Self::reverse(there);
        !value.is_nan()
            && !back.is_nan()
            && (value.to_bits() == back.to_bits()
                || (value.to_bits() & 0x7fff_ffff == 0 && back.to_bits() & 0x7fff_ffff == 0))
    }

    fn forward(value: f32) -> u32 {
        (value.max(0.0) * Self::SCALING).floor() as u32
    }

    fn reverse(encoded: u32) -> f32 {
        encoded as f32 / Self::SCALING
    }

    /// Slightly inaccurate scaling and quantization.
    /// Clamps the value into the representable range if it is negative or too large.
    pub(crate) fn new(value: f32) -> Self {
        Self(Self::forward(value))
    }

    /// Fully accurate construction from a value scaled as per specification.
    pub(crate) fn from_scaled(val: u32) -> Self {
        Self(val)
    }

    /// Get the accurate encoded value.
    pub(crate) fn into_scaled(self) -> u32 {
        self.0
    }

    /// Get the unscaled value as a floating point.
    pub(crate) fn into_value(self) -> f32 {
        Self::reverse(self.0)
    }

    #[cfg(test)]
    pub(crate) fn encode_gama<W: Write>(self, w: &mut W) -> encoder::Result<()> {
        encoder::write_chunk(w, chunk::G_AMA, &self.into_scaled().to_be_bytes())
    }
}

/// Chromaticities of the color space primaries
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceChromaticities {
    pub(crate) white: (ScaledFloat, ScaledFloat),
    pub(crate) red: (ScaledFloat, ScaledFloat),
    pub(crate) green: (ScaledFloat, ScaledFloat),
    pub(crate) blue: (ScaledFloat, ScaledFloat),
}

impl SourceChromaticities {
    pub(crate) fn new(
        white: (f32, f32),
        red: (f32, f32),
        green: (f32, f32),
        blue: (f32, f32),
    ) -> Self {
        SourceChromaticities {
            white: (ScaledFloat::new(white.0), ScaledFloat::new(white.1)),
            red: (ScaledFloat::new(red.0), ScaledFloat::new(red.1)),
            green: (ScaledFloat::new(green.0), ScaledFloat::new(green.1)),
            blue: (ScaledFloat::new(blue.0), ScaledFloat::new(blue.1)),
        }
    }

    #[rustfmt::skip]
    pub(crate) fn to_be_bytes(self) -> [u8; 32] {
        let white_x = self.white.0.into_scaled().to_be_bytes();
        let white_y = self.white.1.into_scaled().to_be_bytes();
        let red_x   = self.red.0.into_scaled().to_be_bytes();
        let red_y   = self.red.1.into_scaled().to_be_bytes();
        let green_x = self.green.0.into_scaled().to_be_bytes();
        let green_y = self.green.1.into_scaled().to_be_bytes();
        let blue_x  = self.blue.0.into_scaled().to_be_bytes();
        let blue_y  = self.blue.1.into_scaled().to_be_bytes();
        [
            white_x[0], white_x[1], white_x[2], white_x[3],
            white_y[0], white_y[1], white_y[2], white_y[3],
            red_x[0],   red_x[1],   red_x[2],   red_x[3],
            red_y[0],   red_y[1],   red_y[2],   red_y[3],
            green_x[0], green_x[1], green_x[2], green_x[3],
            green_y[0], green_y[1], green_y[2], green_y[3],
            blue_x[0],  blue_x[1],  blue_x[2],  blue_x[3],
            blue_y[0],  blue_y[1],  blue_y[2],  blue_y[3],
        ]
    }

    #[cfg(test)]
    pub(crate) fn encode<W: Write>(self, w: &mut W) -> encoder::Result<()> {
        encoder::write_chunk(w, chunk::C_HRM, &self.to_be_bytes())
    }
}

/// The rendering intent for an sRGB image.
///
/// Presence of this data also indicates that the image conforms to the sRGB color space.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SrgbRenderingIntent {
    /// For images preferring good adaptation to the output device gamut at the expense of colorimetric accuracy, such as photographs.
    Perceptual = 0,
    /// For images requiring colour appearance matching (relative to the output device white point), such as logos.
    RelativeColorimetric = 1,
    /// For images preferring preservation of saturation at the expense of hue and lightness, such as charts and graphs.
    Saturation = 2,
    /// For images requiring preservation of absolute colorimetry, such as previews of images destined for a different output device (proofs).
    AbsoluteColorimetric = 3,
}

impl SrgbRenderingIntent {
    pub(crate) fn into_raw(self) -> u8 {
        self as u8
    }

    pub(crate) fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(SrgbRenderingIntent::Perceptual),
            1 => Some(SrgbRenderingIntent::RelativeColorimetric),
            2 => Some(SrgbRenderingIntent::Saturation),
            3 => Some(SrgbRenderingIntent::AbsoluteColorimetric),
            _ => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn encode<W: Write>(self, w: &mut W) -> encoder::Result<()> {
        encoder::write_chunk(w, chunk::S_RGB, &[self.into_raw()])
    }
}

/// Coding-independent code points (cICP) specify the color space (primaries),
/// transfer function, matrix coefficients and scaling factor of the image using
/// the code points specified in [ITU-T-H.273](https://www.itu.int/rec/T-REC-H.273).
///
/// See https://www.w3.org/TR/png-3/#cICP-chunk for more details.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CodingIndependentCodePoints {
    /// Id number of the color primaries defined in
    /// [ITU-T-H.273](https://www.itu.int/rec/T-REC-H.273) in "Table 2 -
    /// Interpretation of colour primaries (ColourPrimaries) value".
    pub(crate) color_primaries: u8,

    /// Id number of the transfer characteristics defined in
    /// [ITU-T-H.273](https://www.itu.int/rec/T-REC-H.273) in "Table 3 -
    /// Interpretation of transfer characteristics (TransferCharacteristics)
    /// value".
    pub(crate) transfer_function: u8,

    /// Id number of the matrix coefficients defined in
    /// [ITU-T-H.273](https://www.itu.int/rec/T-REC-H.273) in "Table 4 -
    /// Interpretation of matrix coefficients (MatrixCoefficients) value".
    ///
    /// This field is included to faithfully replicate the base
    /// [ITU-T-H.273](https://www.itu.int/rec/T-REC-H.273) specification, but matrix coefficients
    /// will always be set to 0, because RGB is currently the only supported color mode in PNG.
    pub(crate) matrix_coefficients: u8,

    /// Whether the image is
    /// [a full range image](https://www.w3.org/TR/png-3/#dfn-full-range-image)
    /// or
    /// [a narrow range image](https://www.w3.org/TR/png-3/#dfn-narrow-range-image).
    ///
    /// This field is included to faithfully replicate the base
    /// [ITU-T-H.273](https://www.itu.int/rec/T-REC-H.273) specification, but it has limited
    /// practical application to PNG images, because narrow-range images are [quite
    /// rare](https://github.com/w3c/png/issues/312#issuecomment-2327349614) in practice.
    pub(crate) is_video_full_range_image: bool,
}

/// Mastering Display Color Volume (mDCV) used at the point of content creation,
/// as specified in [SMPTE-ST-2086](https://ieeexplore.ieee.org/stamp/stamp.jsp?arnumber=8353899).
///
/// See https://www.w3.org/TR/png-3/#mDCV-chunk for more details.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MasteringDisplayColorVolume {
    /// Mastering display chromaticities.
    pub(crate) chromaticities: SourceChromaticities,

    /// Mastering display maximum luminance.
    ///
    /// The value is expressed in units of 0.0001 cd/m^2 - for example if this field
    /// is set to `10000000` then it indicates 1000 cd/m^2.
    pub(crate) max_luminance: u32,

    /// Mastering display minimum luminance.
    ///
    /// The value is expressed in units of 0.0001 cd/m^2 - for example if this field
    /// is set to `10000000` then it indicates 1000 cd/m^2.
    pub(crate) min_luminance: u32,
}

/// Content light level information of HDR content.
///
/// See https://www.w3.org/TR/png-3/#cLLI-chunk for more details.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ContentLightLevelInfo {
    /// Maximum Content Light Level indicates the maximum light level of any
    /// single pixel (in cd/m^2, also known as nits) of the entire playback
    /// sequence.
    ///
    /// The value is expressed in units of 0.0001 cd/m^2 - for example if this field
    /// is set to `10000000` then it indicates 1000 cd/m^2.
    ///
    /// A value of zero means that the value is unknown or not currently calculable.
    pub(crate) max_content_light_level: u32,

    /// Maximum Frame Average Light Level indicates the maximum value of the
    /// frame average light level (in cd/m^2, also known as nits) of the entire
    /// playback sequence. It is calculated by first averaging the decoded
    /// luminance values of all the pixels in each frame, and then using the
    /// value for the frame with the highest value.
    ///
    /// The value is expressed in units of 0.0001 cd/m^2 - for example if this field
    /// is set to `10000000` then it indicates 1000 cd/m^2.
    ///
    /// A value of zero means that the value is unknown or not currently calculable.
    pub(crate) max_frame_average_light_level: u32,
}
