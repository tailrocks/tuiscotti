use super::*;

impl<'a, W: Write> Write for StreamWriter<'a, W> {
    fn write(&mut self, mut data: &[u8]) -> io::Result<usize> {
        if let Wrapper::Unrecoverable = self.writer {
            let err = FormatErrorKind::Unrecoverable.into();
            return Err(EncodingError::Format(err).into());
        }

        if data.is_empty() {
            return Ok(0);
        }

        if self.to_write == 0 {
            match self.writer.take() {
                Wrapper::Flate2(wrt) => match wrt.finish() {
                    Ok(chunk) => self.writer = Wrapper::Chunk(chunk),
                    Err(err) => {
                        self.writer = Wrapper::Unrecoverable;
                        return Err(err);
                    }
                },
                Wrapper::FDeflate(wrt) => match wrt.finish() {
                    Ok(chunk) => self.writer = Wrapper::Chunk(chunk),
                    Err(err) => {
                        self.writer = Wrapper::Unrecoverable;
                        return Err(err);
                    }
                },
                chunk @ Wrapper::Chunk(_) => self.writer = chunk,
                Wrapper::Unrecoverable => unreachable!(),
                Wrapper::None => unreachable!(),
            };

            // Transition Wrapper::Chunk to Wrapper::Zlib.
            self.new_frame()?;
        }

        let written = data.read(&mut self.curr_buf[..self.line_len][self.index..])?;
        self.index += written;
        self.to_write -= written;

        if self.index == self.line_len {
            let filter_type = filter(
                self.filter,
                self.bpp,
                &self.prev_buf,
                &self.curr_buf,
                &mut self.filtered_buf,
            );
            // This can't fail as the other variant is used only to allow the zlib encoder to finish
            match &mut self.writer {
                Wrapper::Flate2(wrt) => {
                    wrt.write_all(&[filter_type as u8])?;
                    wrt.write_all(&self.filtered_buf)?;
                }
                Wrapper::FDeflate(wrt) => {
                    wrt.write_data(&[filter_type as u8])?;
                    wrt.write_data(&self.filtered_buf)?;
                }
                _ => unreachable!(),
            };

            mem::swap(&mut self.prev_buf, &mut self.curr_buf);
            self.index = 0;
        }

        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        match &mut self.writer {
            Wrapper::Flate2(wrt) => wrt.flush()?,
            Wrapper::Chunk(wrt) => wrt.flush()?,
            Wrapper::FDeflate(_) => (), // TODO: Add `flush()` to `crate::sample::embedded_png::fdeflate::Compressor`?
            // This handles both the case where we entered an unrecoverable state after zlib
            // decoding failure and after a panic while we had taken the chunk/zlib reader.
            Wrapper::Unrecoverable | Wrapper::None => {
                let err = FormatErrorKind::Unrecoverable.into();
                return Err(EncodingError::Format(err).into());
            }
        }

        if self.index > 0 {
            let err = FormatErrorKind::WrittenTooMuch(self.index).into();
            return Err(EncodingError::Format(err).into());
        }

        Ok(())
    }
}

impl<W: Write> Drop for StreamWriter<'_, W> {
    fn drop(&mut self) {
        drop(self.finish_mut());
    }
}
