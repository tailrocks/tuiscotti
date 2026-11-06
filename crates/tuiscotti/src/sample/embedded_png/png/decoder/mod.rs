mod interlace_info;
mod read_decoder;
pub(crate) mod stream;
pub(crate) mod transform;
mod unfiltering_buffer;
mod zlib;

use self::read_decoder::{ImageDataCompletionStatus, ReadDecoder};
use self::stream::{DecodeOptions, DecodingError, FormatErrorInner};
use self::transform::{TransformFn, create_transform_fn};
use self::unfiltering_buffer::UnfilteringBuffer;

use std::io::{BufRead, Seek};
use std::mem;

use crate::sample::embedded_png::png::FrameControl;
use crate::sample::embedded_png::png::adam7::Adam7Info;
use crate::sample::embedded_png::png::common::{
    BitDepth, BytesPerPixel, ColorType, Info, ParameterErrorKind, Transformations,
};
pub(crate) use zlib::{UnfilterBuf, UnfilterRegion};

pub(crate) use interlace_info::InterlaceInfo;
use interlace_info::InterlaceInfoIter;

/*
pub(crate) enum InterlaceHandling {
    /// Outputs the raw rows
    RawRows,
    /// Fill missing the pixels from the existing ones
    Rectangle,
    /// Only fill the needed pixels
    Sparkle
}
*/

/// Output info.
///
/// This describes one particular frame of the image that was written into the output buffer.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OutputInfo {
    /// The pixel width of this frame.
    pub(crate) width: u32,
    /// The pixel height of this frame.
    pub(crate) height: u32,
    /// The chosen output color type.
    pub(crate) color_type: ColorType,
    /// The chosen output bit depth.
    pub(crate) bit_depth: BitDepth,
    /// The byte count of each scan line in the image.
    pub(crate) line_size: usize,
}

impl OutputInfo {
    /// Returns the size needed to hold a decoded frame
    /// If the output buffer was larger then bytes after this count should be ignored. They may
    /// still have been changed.
    pub(crate) fn buffer_size(&self) -> usize {
        self.line_size * self.height as usize
    }
}

#[derive(Clone, Copy, Debug)]
/// Limits on the resources the `Decoder` is allowed too use
pub(crate) struct Limits {
    /// maximum number of bytes the decoder is allowed to allocate, default is 64Mib
    pub(crate) bytes: usize,
}

impl Limits {
    pub(crate) fn reserve_bytes(&mut self, bytes: usize) -> Result<(), DecodingError> {
        if self.bytes >= bytes {
            self.bytes -= bytes;
            Ok(())
        } else {
            Err(DecodingError::LimitsExceeded)
        }
    }
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            bytes: 1024 * 1024 * 64,
        }
    }
}

/// PNG Decoder
pub(crate) struct Decoder<R: BufRead + Seek> {
    read_decoder: ReadDecoder<R>,
    /// Output transformations
    transform: Transformations,
}

/// A row of data with interlace information attached.
#[derive(Clone, Copy, Debug)]
pub(crate) struct InterlacedRow<'data> {
    data: &'data [u8],
    interlace: InterlaceInfo,
}

impl<'data> InterlacedRow<'data> {
    pub(crate) fn data(&self) -> &'data [u8] {
        self.data
    }

    pub(crate) fn interlace(&self) -> &InterlaceInfo {
        &self.interlace
    }
}

/// A row of data without interlace information.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Row<'data> {
    data: &'data [u8],
}

impl<'data> Row<'data> {
    pub(crate) fn data(&self) -> &'data [u8] {
        self.data
    }
}

impl<R: BufRead + Seek> Decoder<R> {
    /// Create a new decoder configuration with default limits.
    pub(crate) fn new(r: R) -> Decoder<R> {
        Decoder::new_with_limits(r, Limits::default())
    }

    /// Create a new decoder configuration with custom limits.
    pub(crate) fn new_with_limits(r: R, limits: Limits) -> Decoder<R> {
        let mut read_decoder = ReadDecoder::new(r);
        read_decoder.set_limits(limits);

        Decoder {
            read_decoder,
            transform: Transformations::IDENTITY,
        }
    }

    /// Create a new decoder configuration with custom [`DecodeOptions`].
    pub(crate) fn new_with_options(r: R, decode_options: DecodeOptions) -> Decoder<R> {
        let mut read_decoder = ReadDecoder::with_options(r, decode_options);
        read_decoder.set_limits(Limits::default());

        Decoder {
            read_decoder,
            transform: Transformations::IDENTITY,
        }
    }

    /// Limit resource usage.
    ///
    /// This retained upstream example uses a standalone `png` crate import. The embedded decoder
    /// is private here; its limit behavior is exercised by the internal decoder allocation tests.
    ///
    /// Note that your allocations, e.g. when reading into a pre-allocated buffer, are __NOT__
    /// considered part of the limits. Nevertheless, required intermediate buffers such as for
    /// singular lines is checked against the limit.
    ///
    /// Note that this is a best-effort basis.
    ///
    /// ```text
    /// use std::fs::File;
    /// use std::io::BufReader;
    /// use png::{Decoder, Limits};
    /// // This image is 32×32, 1bit per pixel. The reader buffers one row which requires 4 bytes.
    /// let mut limits = Limits::default();
    /// limits.bytes = 3;
    /// let mut decoder = Decoder::new_with_limits(BufReader::new(File::open("tests/pngsuite/basi0g01.png").unwrap()), limits);
    /// assert!(decoder.read_info().is_err());
    ///
    /// // This image is 32x32 pixels, so the decoder will allocate less than 10Kib
    /// let mut limits = Limits::default();
    /// limits.bytes = 10*1024;
    /// let mut decoder = Decoder::new_with_limits(BufReader::new(File::open("tests/pngsuite/basi0g01.png").unwrap()), limits);
    /// assert!(decoder.read_info().is_ok());
    /// ```text
    pub(crate) fn set_limits(&mut self, limits: Limits) {
        self.read_decoder.set_limits(limits);
    }

    /// Read the PNG header and return the information contained within.
    ///
    /// Most image metadata will not be read until [`read_info`] is called, so those fields will be
    /// None or empty.
    ///
    /// [`read_info`]: Self::read_info
    pub(crate) fn read_header_info(&mut self) -> Result<&Info<'static>, DecodingError> {
        self.read_decoder.read_header_info()
    }

    /// Reads all meta data until the first IDAT chunk
    pub(crate) fn read_info(mut self) -> Result<Reader<R>, DecodingError> {
        let info = self.read_header_info()?;
        let unfiltering_buffer = UnfilteringBuffer::new(info);

        let mut reader = Reader {
            decoder: self.read_decoder,
            bpp: BytesPerPixel::One,
            subframe: SubframeInfo::not_yet_init(),
            remaining_frames: 0, // Temporary value - fixed below after reading `acTL` and `fcTL`.
            unfiltering_buffer,
            transform: self.transform,
            transform_fn: None,
            scratch_buffer: Vec::new(),
            finished: false,
        };

        // Check if the decoding buffer of a single raw line has a valid size.
        //
        // FIXME: this check and the next can be delayed until processing image data. This would
        // allow usage where only the metadata is processes, or where the image is processed
        // line-by-line even on targets that can not fit the whole image into their address space.
        // We should strive for a balance between implementation complexity (still ensure that the
        // no-overflow preconditions are met for internal calculation) and use possibilities.
        if reader.info()?.checked_raw_row_length().is_none() {
            return Err(DecodingError::LimitsExceeded);
        }

        // Check if the output buffer has a valid size.
        //
        // FIXME: see above and
        // <https://github.com/image-rs/image-png/pull/608#issuecomment-3003576956>
        if reader.output_buffer_size()?.is_none() {
            return Err(DecodingError::LimitsExceeded);
        }

        reader.read_until_image_data()?;

        reader.remaining_frames = match reader.info()?.animation_control.as_ref() {
            None => 1, // No `acTL` => only expecting `IDAT` frame.
            Some(animation) => {
                // Note: limited to (2^32 - 1) frames by the APNG spec so addition does not
                // overflow on 32-bit targets nor 64-bit targets.
                let mut num_frames = animation.num_frames;
                if reader.info()?.frame_control.is_none() {
                    // No `fcTL` before `IDAT` => `IDAT` is not part of the animation, but
                    // represents an *extra*, default frame for non-APNG-aware decoders.
                    num_frames += 1;
                }
                num_frames
            }
        };
        Ok(reader)
    }

    /// Set the allowed and performed transformations.
    ///
    /// A transformation is a pre-processing on the raw image data modifying content or encoding.
    /// Many options have an impact on memory or CPU usage during decoding.
    pub(crate) fn set_transformations(&mut self, transform: Transformations) {
        self.transform = transform;
    }

    /// Set the decoder to ignore all text chunks while parsing.
    ///
    /// The standalone upstream example is retained as text because this embedded decoder is
    /// private; text-chunk parsing is exercised by the internal metadata and image corpus tests.
    ///
    /// eg.
    /// ```text
    /// use std::fs::File;
    /// use std::io::BufReader;
    /// use png::Decoder;
    /// let mut decoder = Decoder::new(BufReader::new(File::open("tests/pngsuite/basi0g01.png").unwrap()));
    /// decoder.set_ignore_text_chunk(true);
    /// assert!(decoder.read_info().is_ok());
    /// ```text
    pub(crate) fn set_ignore_text_chunk(&mut self, ignore_text_chunk: bool) {
        self.read_decoder.set_ignore_text_chunk(ignore_text_chunk);
    }

    /// Set the decoder to ignore iccp chunks while parsing.
    ///
    /// The standalone upstream example is retained as text because this embedded decoder is
    /// private; `test_png_with_broken_iccp` exercises this behavior internally.
    ///
    /// eg.
    /// ```text
    /// use std::fs::File;
    /// use std::io::BufReader;
    /// use png::Decoder;
    /// let mut decoder = Decoder::new(BufReader::new(File::open("tests/iccp/broken_iccp.png").unwrap()));
    /// decoder.set_ignore_iccp_chunk(true);
    /// assert!(decoder.read_info().is_ok());
    /// ```text
    pub(crate) fn set_ignore_iccp_chunk(&mut self, ignore_iccp_chunk: bool) {
        self.read_decoder.set_ignore_iccp_chunk(ignore_iccp_chunk);
    }

    /// Set the decoder to ignore and not verify the Adler-32 checksum
    /// and CRC code.
    pub(crate) fn ignore_checksums(&mut self, ignore_checksums: bool) {
        self.read_decoder.ignore_checksums(ignore_checksums);
    }
}

mod reader;
pub(crate) use reader::Reader;
use reader::SubframeInfo;
