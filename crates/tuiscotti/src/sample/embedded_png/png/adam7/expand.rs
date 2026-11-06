use super::*;

pub(super) fn subbyte_values<const N: usize>(
    scanline: &[u8],
    bit_pos: [u8; N],
    mask: u8,
) -> impl Iterator<Item = u8> + '_ {
    (scanline.iter().copied()).flat_map(move |value| bit_pos.map(|n| (value >> n) & mask))
}

/// Given `row_stride`, interlace `info`, and bits-per-pixel, produce an iterator of bit positions
/// of pixels to copy from the input scanline to the image buffer.  The positions are expressed as
/// bit offsets from position (0,0) in the frame that is currently being decoded.
///
/// This should only be used with `bits_pp < 8`.
pub(super) fn expand_adam7_bits(
    row_stride_in_bytes: usize,
    info: &Adam7Info,
    bits_pp: u8,
) -> impl Iterator<Item = BitPostion> {
    debug_assert!(bits_pp == 1 || bits_pp == 2 || bits_pp == 4);
    let (line_mul, line_off, samp_mul, samp_off) = {
        let constants = info.pass_constants();
        (
            // Convert each to their respectively required type from u8.
            usize::from(constants.y_sampling),
            usize::from(constants.y_offset),
            u64::from(constants.x_sampling),
            u64::from(constants.x_offset),
        )
    };

    // the equivalent line number in progressive scan
    let prog_line = line_mul * info.line as usize + line_off;
    let byte_start = prog_line * row_stride_in_bytes;

    // In contrast to `subbyte_values` we *must* be precise with our length here.
    (0..u64::from(info.samples))
        // Bounded by u32::MAX * 8 * 4 + 16 so does not overflow `u64`.
        .map(move |i| (i * samp_mul + samp_off) * u64::from(bits_pp))
        .map(move |i| BitPostion {
            // Bounded by the buffer size which already exists.
            byte: byte_start + (i / 8) as usize,
            bit: i as u8 % 8,
        })
}

pub(super) fn expand_adam7_bytes(
    row_stride_in_bytes: usize,
    info: &Adam7Info,
    bytes_pp: u8,
) -> impl Iterator<Item = usize> {
    let (line_mul, line_off, samp_mul, samp_off) = {
        let constants = info.pass_constants();
        (
            // Convert each to their respectively required type from u8.
            usize::from(constants.y_sampling),
            usize::from(constants.y_offset),
            u64::from(constants.x_sampling),
            u64::from(constants.x_offset),
        )
    };

    // the equivalent line number in progressive scan
    let prog_line = line_mul * info.line as usize + line_off;
    let byte_start = prog_line * row_stride_in_bytes;

    (0..u64::from(info.samples))
        .map(move |i| (i * samp_mul + samp_off) * u64::from(bytes_pp))
        // Bounded by the allocated buffer size so must fit in `usize`
        .map(move |i| i as usize + byte_start)
}

/// Copies pixels from `interlaced_row` into the right location in `img`.
///
/// First bytes of `img` should belong to the top-left corner of the currently decoded frame.
///
/// `img_row_stride` specifies an offset in bytes between subsequent rows of `img`.
/// This can be the width of the current frame being decoded, but this is not required - a bigger
/// stride may be useful if the frame being decoded is a sub-region of `img`.
///
/// `interlaced_row` and `interlace_info` typically come from
/// [`Reader::next_interlaced_row`], but this is not required.  In particular, before
/// calling `expand_interlaced_row` one may need to expand the decoded row, so that its format and
/// `bits_per_pixel` matches that of `img`.  Note that in initial Adam7 passes the `interlaced_row`
/// may contain less pixels that the width of the frame being decoded (e.g. it contains only 1/8th
/// of pixels in the initial pass).
///
/// Example:
///
/// This upstream snippet is shown as text because the embedded decoder is private. The internal
/// `adam7_equivalence` and splat tests execute the same expansion behavior.
///
/// ```text
/// use png::{expand_interlaced_row, Adam7Info};
/// let info = Adam7Info::new(5, 0, 8);
/// let mut img = vec![0; 8 * 8];
/// let row = vec![1, 2, 3, 4];
/// expand_interlaced_row(&mut img, 8, &row, &info, 8);
/// assert_eq!(&img, &[
///     0, 0, 0, 0, 0, 0, 0, 0,
///     0, 0, 0, 0, 0, 0, 0, 0,
///     1, 0, 2, 0, 3, 0, 4, 0,  // <= this is where the 1st line of 5s appears
///     0, 0, 0, 0, 0, 0, 0, 0,  //    in the schematic drawing of the passes at
///     0, 0, 0, 0, 0, 0, 0, 0,  //    https://en.wikipedia.org/wiki/Adam7_algorithm
///     0, 0, 0, 0, 0, 0, 0, 0,
///     0, 0, 0, 0, 0, 0, 0, 0,
///     0, 0, 0, 0, 0, 0, 0, 0,
/// ]);
/// ```text
pub(crate) fn expand_pass(
    img: &mut [u8],
    img_row_stride: usize,
    interlaced_row: &[u8],
    interlace_info: &Adam7Info,
    bits_per_pixel: u8,
) {
    match bits_per_pixel {
        // Note: for 1, 2, 4 multiple runs through the iteration will access the same byte in `img`
        // so we can not iterate over `&mut u8` values. A better strategy would write multiple bit
        // groups in one go. This would then also not be as bounds-check heavy?
        1 => {
            const BIT_POS_1: [u8; 8] = [7, 6, 5, 4, 3, 2, 1, 0];
            let bit_indices = expand_adam7_bits(img_row_stride, interlace_info, 1);
            for (pos, px) in bit_indices.zip(subbyte_values(interlaced_row, BIT_POS_1, 0b1)) {
                let shift = 8 - bits_per_pixel - pos.bit;
                img[pos.byte] |= px << shift;
            }
        }
        2 => {
            const BIT_POS_2: [u8; 4] = [6, 4, 2, 0];
            let bit_indices = expand_adam7_bits(img_row_stride, interlace_info, 2);

            for (pos, px) in bit_indices.zip(subbyte_values(interlaced_row, BIT_POS_2, 0b11)) {
                let shift = 8 - bits_per_pixel - pos.bit;
                img[pos.byte] |= px << shift;
            }
        }
        4 => {
            const BIT_POS_4: [u8; 2] = [4, 0];
            let bit_indices = expand_adam7_bits(img_row_stride, interlace_info, 4);

            for (pos, px) in bit_indices.zip(subbyte_values(interlaced_row, BIT_POS_4, 0b1111)) {
                let shift = 8 - bits_per_pixel - pos.bit;
                img[pos.byte] |= px << shift;
            }
        }
        // While caught by the below loop, we special case this for codegen. The effects are
        // massive when the compiler uses the constant chunk size in particular for this case where
        // no more copy_from_slice is being issued by everything happens in the register alone.
        8 => {
            let byte_indices = expand_adam7_bytes(img_row_stride, interlace_info, 1);

            for (bytepos, &px) in byte_indices.zip(interlaced_row) {
                img[bytepos] = px;
            }
        }
        16 => {
            let byte_indices = expand_adam7_bytes(img_row_stride, interlace_info, 2);

            for (bytepos, px) in byte_indices.zip(interlaced_row.chunks(2)) {
                img[bytepos..][..2].copy_from_slice(px);
            }
        }
        _ => {
            debug_assert!(bits_per_pixel % 8 == 0);
            let bytes_pp = bits_per_pixel / 8;
            let byte_indices = expand_adam7_bytes(img_row_stride, interlace_info, bytes_pp);

            for (bytepos, px) in byte_indices.zip(interlaced_row.chunks(bytes_pp.into())) {
                img[bytepos..][..px.len()].copy_from_slice(px);
            }
        }
    }
}

/// Expand pass, but also ensure that after each pass the whole image has been initialized up to
/// the data available. In constrast to `expand_pass` there are no holes left in the image.
///
/// For instance, consider the first pass which is an eighth subsampling of the original image.
/// Here's a side by-side of pixel data written from each of the two algorithms:
///
/// ```texttext
/// normal:   splat:
/// 1-------  11111111
/// --------  11111111
/// --------  11111111
/// --------  11111111
/// --------  11111111
/// --------  11111111
/// --------  11111111
/// ```text
///
/// Data written in previous passes must not be modified. We 'weave' the data of passes and repeat
/// them in the neighbouring pixels until their subsampling alignment. For details, see the
/// `x_repeat` and `y_repeat` data. Thus the 4th pass would look like this:
///
/// ```texttext
/// normal:   splat:
/// --4---4-  --44--44
/// --------  --44--44
/// --------  --44--44
/// --4---4-  --44--44
/// --------  --44--44
/// --------  --44--44
/// --------  --44--44
/// ```text
///
#[cfg(test)]
pub(crate) fn expand_pass_splat(
    img: &mut [u8],
    img_row_stride: usize,
    interlaced_row: &[u8],
    interlace_info: &Adam7Info,
    bits_per_pixel: u8,
) -> std::io::Result<()> {
    fn expand_bits_to_img(
        img: &mut [u8],
        px: u8,
        mut pos: BitPostion,
        repeat: RangeTo<u8>,
        bpp: u8,
    ) -> std::io::Result<()> {
        let (mut into, mut tail) = img
            .get_mut(pos.byte..)
            .and_then(|bytes| bytes.split_first_mut())
            .ok_or_else(|| std::io::Error::other("Adam7 bit position exceeds output buffer"))?;
        let mask = (1u8 << bpp) - 1;

        for _ in 0..repeat.end {
            if pos.bit >= 8 {
                pos.byte += 1;
                pos.bit -= 8;

                (into, tail) = tail.split_first_mut().ok_or_else(|| {
                    std::io::Error::other("Adam7 bit position exceeds output buffer")
                })?;
            }

            let shift = 8 - bpp - pos.bit;
            // Preserve all other bits, but be prepared for existing bits
            let pre = (*into >> shift) & mask;
            *into ^= (pre ^ px) << shift;

            pos.bit += bpp;
        }
        Ok(())
    }

    let height = (img.len() / img_row_stride) as u32;
    let y_repeat = interlace_info.splat_line_repeat(height);
    debug_assert!(y_repeat > 0);

    match bits_per_pixel {
        // Note: for 1, 2, 4 multiple runs through the iteration will access the same byte in `img`
        // so we can not iterate over `&mut u8` values. A better strategy would write multiple bit
        // groups in one go. This would then also not be as bounds-check heavy?
        1 => {
            const BIT_POS_1: [u8; 8] = [7, 6, 5, 4, 3, 2, 1, 0];

            for offset in 0..y_repeat {
                let bit_indices = expand_adam7_bits(img_row_stride, interlace_info, 1);
                let line_offset = usize::from(offset) * img_row_stride;

                for (idx, (mut pos, px)) in bit_indices
                    .zip(subbyte_values(interlaced_row, BIT_POS_1, 0b1))
                    .enumerate()
                {
                    let x_repeat = interlace_info.splat_pixel_repeat(idx);
                    debug_assert!(x_repeat > 0);
                    pos.byte += line_offset;
                    expand_bits_to_img(img, px, pos, ..x_repeat, bits_per_pixel)?;
                }
            }
        }
        2 => {
            const BIT_POS_2: [u8; 4] = [6, 4, 2, 0];

            for offset in 0..y_repeat {
                let bit_indices = expand_adam7_bits(img_row_stride, interlace_info, 2);
                let line_offset = usize::from(offset) * img_row_stride;

                for (idx, (mut pos, px)) in bit_indices
                    .zip(subbyte_values(interlaced_row, BIT_POS_2, 0b11))
                    .enumerate()
                {
                    let x_repeat = interlace_info.splat_pixel_repeat(idx);
                    pos.byte += line_offset;
                    expand_bits_to_img(img, px, pos, ..x_repeat, bits_per_pixel)?;
                }
            }
        }
        4 => {
            const BIT_POS_4: [u8; 2] = [4, 0];

            for offset in 0..y_repeat {
                let bit_indices = expand_adam7_bits(img_row_stride, interlace_info, 4);
                let line_offset = usize::from(offset) * img_row_stride;

                for (idx, (mut pos, px)) in bit_indices
                    .zip(subbyte_values(interlaced_row, BIT_POS_4, 0b1111))
                    .enumerate()
                {
                    let x_repeat = interlace_info.splat_pixel_repeat(idx);
                    pos.byte += line_offset;
                    expand_bits_to_img(img, px, pos, ..x_repeat, bits_per_pixel)?;
                }
            }
        }
        // While caught by the below loop, we special case this for codegen. The effects are
        // massive when the compiler uses the constant chunk size in particular for this case where
        // no more copy_from_slice is being issued by everything happens in the register alone.
        8 => {
            for offset in 0..y_repeat {
                let byte_indices = expand_adam7_bytes(img_row_stride, interlace_info, 1);
                let line_offset = usize::from(offset) * img_row_stride;

                for (idx, (bytepos, px)) in byte_indices.zip(interlaced_row).enumerate() {
                    let x_repeat = usize::from(interlace_info.splat_pixel_repeat(idx));
                    debug_assert!(x_repeat > 0);
                    img[line_offset + bytepos..][..x_repeat].fill(*px);
                }
            }
        }
        _ => {
            debug_assert!(bits_per_pixel % 8 == 0);
            let bytes = bits_per_pixel / 8;
            let chunk = usize::from(bytes);

            for offset in 0..y_repeat {
                let byte_indices = expand_adam7_bytes(img_row_stride, interlace_info, bytes);
                let line_offset = usize::from(offset) * img_row_stride;

                for (idx, (bytepos, px)) in byte_indices
                    .zip(interlaced_row.chunks_exact(chunk))
                    .enumerate()
                {
                    let x_repeat = usize::from(interlace_info.splat_pixel_repeat(idx));
                    let target = &mut img[line_offset + bytepos..][..chunk * x_repeat];

                    for target in target.chunks_exact_mut(chunk) {
                        target.copy_from_slice(px);
                    }
                }
            }
        }
    }
    Ok(())
}
