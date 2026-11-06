use super::*;

pub(super) enum ChunkOutput<'a, W: Write> {
    Borrowed(&'a mut Writer<W>),
    Owned(Writer<W>),
}

// opted for deref for practical reasons
impl<'a, W: Write> Deref for ChunkOutput<'a, W> {
    type Target = Writer<W>;

    fn deref(&self) -> &Self::Target {
        match self {
            ChunkOutput::Borrowed(writer) => writer,
            ChunkOutput::Owned(writer) => writer,
        }
    }
}

impl<'a, W: Write> DerefMut for ChunkOutput<'a, W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            ChunkOutput::Borrowed(writer) => writer,
            ChunkOutput::Owned(writer) => writer,
        }
    }
}

/// This writer is used between the actual writer and the
/// ZlibEncoder and has the job of packaging the compressed
/// data into a PNG chunk, based on the image metadata
///
/// Currently the way it works is that the specified buffer
/// will hold one chunk at the time and buffer the incoming
/// data until `flush` is called or the maximum chunk size
/// is reached.
///
/// The maximum chunk is the smallest between the selected buffer size
/// and `u32::MAX >> 1` (`0x7fffffff` or `2147483647` dec)
///
/// When a chunk has to be flushed the length (that is now known)
/// and the CRC will be written at the correct locations in the chunk.
pub(super) struct ChunkWriter<'a, W: Write> {
    pub(super) writer: ChunkOutput<'a, W>,
    buffer: Vec<u8>,
    /// keeps track of where the last byte was written
    index: usize,
    curr_chunk: ChunkType,
}

impl<'a, W: Write> ChunkWriter<'a, W> {
    pub(super) fn new(writer: ChunkOutput<'a, W>, buf_len: usize) -> ChunkWriter<'a, W> {
        // currently buf_len will determine the size of each chunk
        // the len is capped to the maximum size every chunk can hold
        // (this wont ever overflow an u32)
        //
        // TODO (maybe): find a way to hold two chunks at a time if `usize`
        //               is 64 bits.
        const CAP: usize = u32::MAX as usize >> 1;
        let curr_chunk = if writer.images_written == 0 {
            chunk::IDAT
        } else {
            chunk::FD_AT
        };
        ChunkWriter {
            writer,
            buffer: vec![0; CAP.min(buf_len)],
            index: 0,
            curr_chunk,
        }
    }

    /// Returns the size of each scanline for the next frame
    /// paired with the size of the whole frame
    ///
    /// This is used by the `StreamWriter` to know when the scanline ends
    /// so it can filter compress it and also to know when to start
    /// the next one
    pub(super) fn next_frame_info(&self) -> (usize, usize) {
        let wrt = self.writer.deref();

        let width: usize;
        let height: usize;
        if let Some(fctl) = wrt.info.frame_control {
            width = fctl.width as usize;
            height = fctl.height as usize;
        } else {
            width = wrt.info.width as usize;
            height = wrt.info.height as usize;
        }

        let in_len = wrt.info.raw_row_length_from_width(width as u32) - 1;
        let data_size = in_len * height;

        (in_len, data_size)
    }

    /// NOTE: this bypasses the internal buffer so the flush method should be called before this
    ///       in the case there is some data left in the buffer when this is called, it will panic
    pub(super) fn write_header(&mut self) -> Result<()> {
        assert_eq!(self.index, 0, "Called when not flushed");
        let wrt = self.writer.deref_mut();

        self.curr_chunk = if wrt.images_written == 0 {
            chunk::IDAT
        } else {
            chunk::FD_AT
        };

        match wrt.info.frame_control {
            Some(_) if wrt.should_skip_frame_control_on_default_image() => {}
            Some(ref mut fctl) => {
                fctl.encode(&mut wrt.w)?;
                fctl.sequence_number += 1;
            }
            _ => {}
        }

        Ok(())
    }

    /// Set the [`FrameControl`] for the following frame
    ///
    /// It will ignore the `sequence_number` of the parameter
    /// as it is updated internally.
    pub(super) fn set_fctl(&mut self, f: FrameControl) {
        if let Some(ref mut fctl) = self.writer.info.frame_control {
            // Ignore the sequence number
            *fctl = FrameControl {
                sequence_number: fctl.sequence_number,
                ..f
            };
        } else {
            panic!("This function must be called on an animated PNG")
        }
    }

    /// Flushes the current chunk
    fn flush_inner(&mut self) -> io::Result<()> {
        if self.index > 0 {
            // flush the chunk and reset everything
            write_chunk(
                &mut self.writer.w,
                self.curr_chunk,
                &self.buffer[..self.index],
            )?;

            self.index = 0;
        }
        Ok(())
    }
}

impl<'a, W: Write> Write for ChunkWriter<'a, W> {
    fn write(&mut self, mut data: &[u8]) -> io::Result<usize> {
        if data.is_empty() {
            return Ok(0);
        }

        // index == 0 means a chunk has been flushed out
        if self.index == 0 {
            let wrt = self.writer.deref_mut();

            // Prepare the next animated frame, if any.
            let no_fctl = wrt.should_skip_frame_control_on_default_image();
            if !no_fctl && let Some(fctl) = wrt.info.frame_control.as_mut() {
                self.buffer[0..4].copy_from_slice(&fctl.sequence_number.to_be_bytes());
                fctl.sequence_number += 1;
                self.index = 4;
            }
        }

        // Cap the buffer length to the maximum number of bytes that can't still
        // be added to the current chunk
        let written = data.len().min(self.buffer.len() - self.index);
        data = &data[..written];

        self.buffer[self.index..][..written].copy_from_slice(data);
        self.index += written;

        // if the maximum data for this chunk as been reached it needs to be flushed
        if self.index == self.buffer.len() {
            self.flush_inner()?;
        }

        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_inner()
    }
}

impl<W: Write> Drop for ChunkWriter<'_, W> {
    fn drop(&mut self) {
        drop(self.flush());
    }
}

// TODO: find a better name
//
/// This enum is used to be allow the `StreamWriter` to keep
/// its inner `ChunkWriter` without wrapping it inside a
/// `ZlibEncoder`. This is used in the case that between the
/// change of state that happens when the last write of a frame
/// is performed an error occurs, which obviously has to be returned.
/// This creates the problem of where to store the writer before
/// exiting the function, and this is where `Wrapper` comes in.
///
/// Unfortunately the `ZlibWriter` can't be used because on the
/// write following the error, `finish` would be called and that
/// would write some data even if 0 bytes where compressed.
///
/// If the `finish` function fails then there is nothing much to
/// do as the `ChunkWriter` would get lost so the `Unrecoverable`
/// variant is used to signal that.
pub(super) enum Wrapper<'a, W: Write> {
    Chunk(ChunkWriter<'a, W>),
    Flate2(ZlibEncoder<ChunkWriter<'a, W>>),
    FDeflate(crate::sample::embedded_png::fdeflate::Compressor<ChunkWriter<'a, W>>),
    Unrecoverable,
    /// This is used in-between, should never be matched
    None,
}

impl<'a, W: Write> Wrapper<'a, W> {
    pub(super) fn from_level(
        writer: ChunkWriter<'a, W>,
        compression: DeflateCompression,
    ) -> io::Result<Self> {
        Ok(match compression {
            DeflateCompression::NoCompression => {
                Wrapper::Flate2(ZlibEncoder::new(writer, flate2::Compression::none()))
            }
            DeflateCompression::FdeflateUltraFast => Wrapper::FDeflate(
                crate::sample::embedded_png::fdeflate::Compressor::new(writer)?,
            ),
            DeflateCompression::Level(level) => Wrapper::Flate2(ZlibEncoder::new(
                writer,
                flate2::Compression::new(u32::from(level)),
            )),
        })
    }

    /// Like `Option::take` this returns the `Wrapper` contained
    /// in `self` and replaces it with `Wrapper::None`
    pub(super) fn take(&mut self) -> Wrapper<'a, W> {
        let mut swap = Wrapper::None;
        mem::swap(self, &mut swap);
        swap
    }
}
