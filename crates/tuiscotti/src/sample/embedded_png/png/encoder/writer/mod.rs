use super::*;

mod controls;
mod image;
mod setup;

/// PNG writer
///
/// Progresses through the image by writing images, frames, or raw individual chunks. This is
/// constructed through [`Encoder::write_header()`].
///
/// FIXME: Writing of animated chunks might be clearer if we had an _adapter_ that you would call
/// to guarantee the next image to be prefaced with a fcTL-chunk, and all other chunks would be
/// guaranteed to be `IDAT`/not affected by APNG's frame control.

pub(crate) struct Writer<W: Write> {
    /// The underlying writer.
    pub(super) w: W,
    /// The local version of the `Info` struct.
    pub(super) info: PartialInfo,
    /// Global encoding options.
    pub(super) options: Options,
    /// The total number of image frames, counting all consecutive IDAT and fdAT chunks.
    pub(super) images_written: u64,
    /// The total number of animation frames, that is equivalent to counting fcTL chunks.
    pub(super) animation_written: u32,
    /// A flag to note when the IEND chunk was already added.
    /// This is only set on code paths that drop `Self` to control the destructor.
    pub(super) iend_written: bool,
}

/// Contains the subset of attributes of [Info] needed for [Writer] to function
pub(super) struct PartialInfo {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) bit_depth: BitDepth,
    pub(super) color_type: ColorType,
    pub(super) frame_control: Option<FrameControl>,
    pub(super) animation_control: Option<AnimationControl>,
    pub(super) has_palette: bool,
}

impl PartialInfo {
    pub(super) fn new(info: &Info) -> Self {
        PartialInfo {
            width: info.width,
            height: info.height,
            bit_depth: info.bit_depth,
            color_type: info.color_type,
            frame_control: info.frame_control,
            animation_control: info.animation_control,
            has_palette: info.palette.is_some(),
        }
    }

    pub(super) fn bpp_in_prediction(&self) -> BytesPerPixel {
        BytesPerPixel::from_usize(self.bytes_per_pixel())
    }

    pub(super) fn bytes_per_pixel(&self) -> usize {
        self.color_type.bytes_per_pixel(self.bit_depth)
    }

    pub(super) fn raw_row_length(&self) -> usize {
        self.raw_row_length_from_width(self.width)
    }

    pub(super) fn raw_row_length_from_width(&self, width: u32) -> usize {
        self.color_type
            .raw_row_length_from_width(self.bit_depth, width)
    }
}

const DEFAULT_BUFFER_LENGTH: usize = 4 * 1024;

pub(crate) fn write_chunk<W: Write>(mut w: W, name: chunk::ChunkType, data: &[u8]) -> Result<()> {
    w.write_be(data.len() as u32)?;
    w.write_all(&name.0)?;
    w.write_all(data)?;
    let mut crc = Crc32::new();
    crc.update(&name.0);
    crc.update(data);
    w.write_be(crc.finalize())?;
    Ok(())
}
