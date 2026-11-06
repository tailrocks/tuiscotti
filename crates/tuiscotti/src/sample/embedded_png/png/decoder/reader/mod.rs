use super::*;

mod frames;
mod output;
mod rows;

/// PNG reader (mostly high-level interface)
///
/// Provides a high level that iterates over lines or whole images.
pub(crate) struct Reader<R: BufRead + Seek> {
    pub(super) decoder: ReadDecoder<R>,
    pub(super) bpp: BytesPerPixel,
    pub(super) subframe: SubframeInfo,
    /// How many frames remain to be decoded.  Decremented after each `IDAT` or `fdAT` sequence.
    pub(super) remaining_frames: u32,
    /// Buffer with not-yet-`unfilter`-ed image rows
    pub(super) unfiltering_buffer: UnfilteringBuffer,
    /// Output transformations
    pub(super) transform: Transformations,
    /// Function that can transform decompressed, unfiltered rows into final output.
    /// See the `transform.rs` module for more details.
    pub(super) transform_fn: Option<TransformFn>,
    /// This buffer is only used so that `next_row` and `next_interlaced_row` can return reference
    /// to a byte slice. In a future version of this library, this buffer will be removed and
    /// `next_row` and `next_interlaced_row` will write directly into a user provided output buffer.
    pub(super) scratch_buffer: Vec<u8>,
    /// Whether `ImageEnd` was already reached by `fn finish`.
    pub(super) finished: bool,
}

/// The subframe specific information.
///
/// In APNG the frames are constructed by combining previous frame and a new subframe (through a
/// combination of `dispose_op` and `overlay_op`). These sub frames specify individual dimension
/// information and reuse the global interlace options. This struct encapsulates the state of where
/// in a particular IDAT-frame or subframe we are.
pub(super) struct SubframeInfo {
    width: u32,
    height: u32,
    rowlen: usize,
    current_interlace_info: Option<InterlaceInfo>,
    interlace_info_iter: InterlaceInfoIter,
    consumed_and_flushed: bool,
}

impl SubframeInfo {
    pub(super) fn not_yet_init() -> Self {
        SubframeInfo {
            width: 0,
            height: 0,
            rowlen: 0,
            current_interlace_info: None,
            interlace_info_iter: InterlaceInfoIter::empty(),
            consumed_and_flushed: false,
        }
    }

    pub(super) fn new(info: &Info) -> Self {
        // The apng fctnl overrides width and height.
        // All other data is set by the main info struct.
        let (width, height) = if let Some(fc) = info.frame_control {
            (fc.width, fc.height)
        } else {
            (info.width, info.height)
        };

        let mut interlace_info_iter = InterlaceInfoIter::new(width, height, info.interlaced);
        let current_interlace_info = interlace_info_iter.next();
        SubframeInfo {
            width,
            height,
            rowlen: info.raw_row_length_from_width(width),
            current_interlace_info,
            interlace_info_iter,
            consumed_and_flushed: false,
        }
    }
}
