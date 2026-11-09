//! Transforming a decompressed, unfiltered row into the final output.

mod palette;

use crate::sample::embedded_png::png::{BitDepth, ColorType, DecodingError, Info, Transformations};

use super::stream::FormatErrorInner;

/// Type of a function that can transform a decompressed, unfiltered row (the
/// 1st argument) into the final pixels (the 2nd argument), optionally using
/// image metadata (e.g. PLTE data can be accessed using the 3rd argument).
///
/// The closed variants keep the transform state inline and avoid a fallible
/// heap allocation for each decoder.
pub(crate) enum TransformFn {
    Plain(fn(&[u8], &mut [u8], &Info)),
    Packed(fn(&[u8], &mut [u8], &Info) -> Result<(), DecodingError>),
    PaletteRgb8([[u8; 4]; 256]),
    PaletteRgbPacked([[u8; 4]; 256]),
    PaletteRgba([[u8; 4]; 256]),
}

const _: () = assert!(std::mem::size_of::<TransformFn>() <= 1040);

impl TransformFn {
    pub(crate) fn apply(&self, input: &[u8], output: &mut [u8], info: &Info) {
        drop(self.apply_checked(input, output, info));
    }

    pub(crate) fn try_apply(
        &self,
        input: &[u8],
        output: &mut [u8],
        info: &Info,
    ) -> Result<(), DecodingError> {
        self.apply_checked(input, output, info)
    }

    fn apply_checked(
        &self,
        input: &[u8],
        output: &mut [u8],
        info: &Info,
    ) -> Result<(), DecodingError> {
        match self {
            Self::Plain(transform) => transform(input, output, info),
            Self::Packed(transform) => return transform(input, output, info),
            Self::PaletteRgb8(palette) => palette::expand_8bit_into_rgb8(input, output, palette),
            Self::PaletteRgbPacked(palette) => {
                palette::expand_into_rgb8(input, output, info, palette)
            }
            Self::PaletteRgba(palette) => {
                palette::expand_paletted_into_rgba8(input, output, info, palette)
            }
        }
        Ok(())
    }
}

/// Returns a transformation function that should be applied to image rows based
/// on 1) decoded image metadata (`info`) and 2) the transformations requested
/// by the crate client (`transform`).
pub(crate) fn create_transform_fn(
    info: &Info,
    transform: Transformations,
) -> Result<TransformFn, DecodingError> {
    let color_type = info.color_type;
    let bit_depth = info.bit_depth as u8;
    let trns = info.trns.is_some() || transform.contains(Transformations::ALPHA);
    let expand =
        transform.contains(Transformations::EXPAND) || transform.contains(Transformations::ALPHA);
    let strip16 = bit_depth == 16 && transform.contains(Transformations::STRIP_16);
    match color_type {
        ColorType::Indexed if expand => {
            if info.palette.is_none() {
                Err(DecodingError::Format(
                    FormatErrorInner::PaletteRequired.into(),
                ))
            } else if let BitDepth::Sixteen = info.bit_depth {
                // This should have been caught earlier but let's check again. Can't hurt.
                Err(DecodingError::Format(
                    FormatErrorInner::InvalidColorBitDepth {
                        color_type: ColorType::Indexed,
                        bit_depth: BitDepth::Sixteen,
                    }
                    .into(),
                ))
            } else {
                if trns {
                    palette::create_expansion_into_rgba8(info)
                } else {
                    palette::create_expansion_into_rgb8(info)
                }
            }
        }
        ColorType::Grayscale | ColorType::GrayscaleAlpha if bit_depth < 8 && expand => {
            Ok(TransformFn::Packed(if trns {
                expand_gray_u8_with_trns as fn(&[u8], &mut [u8], &Info) -> Result<(), DecodingError>
            } else {
                expand_gray_u8 as fn(&[u8], &mut [u8], &Info) -> Result<(), DecodingError>
            }))
        }
        ColorType::Grayscale | ColorType::Rgb if expand && trns => {
            Ok(TransformFn::Plain(if bit_depth == 8 {
                expand_trns_line as fn(&[u8], &mut [u8], &Info)
            } else if strip16 {
                expand_trns_and_strip_line16 as fn(&[u8], &mut [u8], &Info)
            } else {
                assert_eq!(bit_depth, 16);
                expand_trns_line16 as fn(&[u8], &mut [u8], &Info)
            }))
        }
        ColorType::Grayscale | ColorType::GrayscaleAlpha | ColorType::Rgb | ColorType::Rgba
            if strip16 =>
        {
            Ok(TransformFn::Plain(transform_row_strip16))
        }
        _ => Ok(TransformFn::Plain(copy_row)),
    }
}

fn copy_row(row: &[u8], output_buffer: &mut [u8], _: &Info) {
    output_buffer.copy_from_slice(row);
}

fn transform_row_strip16(row: &[u8], output_buffer: &mut [u8], _: &Info) {
    for i in 0..row.len() / 2 {
        output_buffer[i] = row[2 * i];
    }
}

#[inline(always)]
fn unpack_bits<F>(input: &[u8], output: &mut [u8], channels: usize, bit_depth: u8, func: F)
where
    F: Fn(u8, &mut [u8]),
{
    // Only [1, 2, 4, 8] are valid bit depths
    if !matches!(bit_depth, 1 | 2 | 4 | 8) {
        return;
    }
    // Check that `input` is capable of producing a buffer as long as `output`:
    // number of shift lookups per bit depth * channels * input length
    if (8 / bit_depth as usize * channels).saturating_mul(input.len()) < output.len() {
        return;
    }

    let mut buf_chunks = output.chunks_exact_mut(channels);
    let mut iter = input.iter();

    // `shift` iterates through the corresponding bit depth sequence:
    // 1 => &[7, 6, 5, 4, 3, 2, 1, 0],
    // 2 => &[6, 4, 2, 0],
    // 4 => &[4, 0],
    // 8 => &[0],
    //
    // `(0..8).step_by(bit_depth.into()).rev()` doesn't always optimize well so
    // shifts are calculated instead. (2023-08, Rust 1.71)

    if bit_depth == 8 {
        for (&curr, chunk) in iter.zip(&mut buf_chunks) {
            func(curr, chunk);
        }
    } else {
        let mask = ((1u16 << bit_depth) - 1) as u8;

        // These variables are initialized in the loop
        let mut shift = -1;
        let mut curr = 0;

        for chunk in buf_chunks {
            if shift < 0 {
                shift = 8 - bit_depth as i32;
                let Some(&value) = iter.next() else {
                    return;
                };
                curr = value;
            }

            let pixel = (curr >> shift) & mask;
            func(pixel, chunk);

            shift -= bit_depth as i32;
        }
    }
}

fn validate_unpack_bits(
    input: &[u8],
    output: &[u8],
    channels: usize,
    bit_depth: u8,
) -> Result<(), DecodingError> {
    if !matches!(bit_depth, 1 | 2 | 4 | 8)
        || (8 / bit_depth as usize * channels).saturating_mul(input.len()) < output.len()
    {
        return Err(DecodingError::InvalidState);
    }
    Ok(())
}

fn expand_trns_line(input: &[u8], output: &mut [u8], info: &Info) {
    let channels = info.color_type.samples();
    let trns = info.trns.as_deref();
    for (input, output) in input
        .chunks_exact(channels)
        .zip(output.chunks_exact_mut(channels + 1))
    {
        output[..channels].copy_from_slice(input);
        output[channels] = if Some(input) == trns { 0 } else { 0xFF };
    }
}

fn expand_trns_line16(input: &[u8], output: &mut [u8], info: &Info) {
    let channels = info.color_type.samples();
    let trns = info.trns.as_deref();
    for (input, output) in input
        .chunks_exact(channels * 2)
        .zip(output.chunks_exact_mut(channels * 2 + 2))
    {
        output[..channels * 2].copy_from_slice(input);
        if Some(input) == trns {
            output[channels * 2] = 0;
            output[channels * 2 + 1] = 0
        } else {
            output[channels * 2] = 0xFF;
            output[channels * 2 + 1] = 0xFF
        };
    }
}

fn expand_trns_and_strip_line16(input: &[u8], output: &mut [u8], info: &Info) {
    let channels = info.color_type.samples();
    let trns = info.trns.as_deref();
    for (input, output) in input
        .chunks_exact(channels * 2)
        .zip(output.chunks_exact_mut(channels + 1))
    {
        for i in 0..channels {
            output[i] = input[i * 2];
        }
        output[channels] = if Some(input) == trns { 0 } else { 0xFF };
    }
}

fn expand_gray_u8(row: &[u8], buffer: &mut [u8], info: &Info) -> Result<(), DecodingError> {
    validate_unpack_bits(row, buffer, 1, info.bit_depth as u8)?;
    let scaling_factor = (255) / ((1u16 << info.bit_depth as u8) - 1) as u8;
    unpack_bits(row, buffer, 1, info.bit_depth as u8, |val, chunk| {
        chunk[0] = val * scaling_factor
    });
    Ok(())
}

fn expand_gray_u8_with_trns(
    row: &[u8],
    buffer: &mut [u8],
    info: &Info,
) -> Result<(), DecodingError> {
    validate_unpack_bits(row, buffer, 2, info.bit_depth as u8)?;
    let scaling_factor = (255) / ((1u16 << info.bit_depth as u8) - 1) as u8;
    let trns = info.trns.as_deref();
    unpack_bits(row, buffer, 2, info.bit_depth as u8, |pixel, chunk| {
        chunk[1] = if let Some(trns) = trns {
            if pixel == trns[0] { 0 } else { 0xFF }
        } else {
            0xFF
        };
        chunk[0] = pixel * scaling_factor
    });
    Ok(())
}
