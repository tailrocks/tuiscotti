use super::*;

fn required<T>(value: Option<T>, message: &str) -> std::result::Result<T, std::io::Error> {
    value.ok_or_else(|| std::io::Error::other(message))
}

#[test]
fn test_only_idat_chunk_in_input_stream() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let png = {
        let mut png = Vec::new();
        write_png_sig(&mut png)?;
        write_chunk(&mut png, b"IDAT", &[])?;
        png
    };
    let decoder = Decoder::new(Cursor::new(&png));
    let err = decoder
        .read_info()
        .err()
        .ok_or_else(|| std::io::Error::other("IDAT before IHDR must fail"))?;
    assert!(matches!(&err, DecodingError::Format(_)));
    assert_eq!(
        "ChunkType { type: IDAT, \
                         critical: true, \
                         private: false, \
                         reserved: false, \
                         safecopy: false \
             } chunk appeared before IHDR chunk",
        format!("{err}"),
    );
    Ok(())
}

/// `StreamingInput` can be used by tests to simulate a streaming input
/// (e.g. a slow http response, where all bytes are not immediately available).
#[derive(Clone)]
struct StreamingInput {
    full_input: Vec<u8>,
    state: Rc<RefCell<StreamingInputState>>,
}

struct StreamingInputState {
    current_pos: usize,
    available_len: usize,
}

impl StreamingInput {
    fn new(full_input: Vec<u8>) -> Self {
        Self {
            full_input,
            state: Rc::new(RefCell::new(StreamingInputState {
                current_pos: 0,
                available_len: 0,
            })),
        }
    }

    fn with_noncompressed_png(width: u32, idat_size: usize) -> std::io::Result<Self> {
        let mut png = Vec::new();
        write_noncompressed_png(&mut png, width, idat_size)?;
        Ok(Self::new(png))
    }

    fn expose_next_byte(&self) {
        let mut state = self.state.borrow_mut();
        assert!(state.available_len < self.full_input.len());
        state.available_len += 1;
    }

    fn stream_input_until_reader_is_available(
        &self,
    ) -> std::result::Result<Reader<StreamingInput>, Box<dyn std::error::Error>> {
        let reader = loop {
            self.state.borrow_mut().current_pos = 0;
            match Decoder::new(self.clone()).read_info() {
                Ok(reader) => {
                    break reader;
                }
                Err(DecodingError::IoError(e)) if e.kind() == ErrorKind::UnexpectedEof => {
                    self.expose_next_byte();
                }
                Err(error) => {
                    return Err(std::io::Error::other(format!(
                        "unexpected stream read error: {error}"
                    ))
                    .into());
                }
            }
        };
        Ok(reader)
    }

    fn decode_full_input<F, R>(&self, f: F) -> std::result::Result<R, Box<dyn std::error::Error>>
    where
        F: FnOnce(Reader<Cursor<&[u8]>>) -> R,
    {
        let decoder = Decoder::new(Cursor::new(&*self.full_input));
        Ok(f(decoder.read_info()?))
    }
}

impl Read for StreamingInput {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut state = self.state.borrow_mut();
        let mut available_bytes = &self.full_input[state.current_pos..state.available_len];
        let number_of_read_bytes = available_bytes.read(buf)?;
        state.current_pos += number_of_read_bytes;
        assert!(state.current_pos <= state.available_len);
        Ok(number_of_read_bytes)
    }
}
impl BufRead for StreamingInput {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        let state = self.state.borrow();
        Ok(&self.full_input[state.current_pos..state.available_len])
    }

    fn consume(&mut self, amt: usize) {
        let mut state = self.state.borrow_mut();
        state.current_pos += amt;
        assert!(state.current_pos <= state.available_len);
    }
}
impl Seek for StreamingInput {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let mut state = self.state.borrow_mut();
        state.current_pos = match pos {
            std::io::SeekFrom::Start(n) => n as usize,
            std::io::SeekFrom::End(n) => (self.full_input.len() as i64 + n) as usize,
            std::io::SeekFrom::Current(n) => (state.current_pos as i64 + n) as usize,
        } as usize;
        Ok(state.current_pos as u64)
    }
    fn stream_position(&mut self) -> std::io::Result<u64> {
        Ok(self.state.borrow().current_pos as u64)
    }
}

/// Test resuming/retrying `Reader.next_frame` after `UnexpectedEof`.
#[test]
fn test_streaming_input_and_decoding_via_next_frame()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    const WIDTH: u32 = 16;
    const IDAT_SIZE: usize = 512;
    let streaming_input = StreamingInput::with_noncompressed_png(WIDTH, IDAT_SIZE)?;

    let (whole_output_info, decoded_from_whole_input) =
        streaming_input.decode_full_input(|mut r| -> Result<_, Box<dyn std::error::Error>> {
            let size = required(r.output_buffer_size()?, "output buffer size missing")?;
            let mut buf = vec![0; size];
            let output_info = r.next_frame(&mut buf)?;
            Ok((output_info, buf))
        })??;

    let mut png_reader = streaming_input.stream_input_until_reader_is_available()?;
    let mut decoded_from_streaming_input = vec![
        0;
        required(
            png_reader.output_buffer_size()?,
            "output buffer size missing"
        )?
    ];
    let streaming_output_info = loop {
        match png_reader.next_frame(decoded_from_streaming_input.as_mut_slice()) {
            Ok(output_info) => break output_info,
            Err(DecodingError::IoError(e)) if e.kind() == ErrorKind::UnexpectedEof => {
                streaming_input.expose_next_byte()
            }
            error => {
                return Err(std::io::Error::other(format!(
                    "unexpected streaming decode error: {error:?}"
                ))
                .into());
            }
        }
    };
    assert_eq!(whole_output_info, streaming_output_info);
    assert_eq!(
        crc32fast::hash(&decoded_from_whole_input),
        crc32fast::hash(&decoded_from_streaming_input)
    );
    Ok(())
}

/// Test resuming/retrying `Reader.next_row` after `UnexpectedEof`.
#[test]
fn test_streaming_input_and_decoding_via_next_row()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    const WIDTH: u32 = 16;
    const IDAT_SIZE: usize = 512;
    let streaming_input = StreamingInput::with_noncompressed_png(WIDTH, IDAT_SIZE)?;

    let decoded_from_whole_input =
        streaming_input.decode_full_input(|mut r| -> Result<_, Box<dyn std::error::Error>> {
            let size = required(r.output_buffer_size()?, "output buffer size missing")?;
            let mut buf = vec![0; size];
            r.next_frame(&mut buf)?;
            Ok(buf)
        })??;

    let mut png_reader = streaming_input.stream_input_until_reader_is_available()?;
    let mut decoded_from_streaming_input = Vec::new();
    loop {
        match png_reader.next_row() {
            Ok(None) => break,
            Ok(Some(row)) => decoded_from_streaming_input.extend_from_slice(row.data()),
            Err(DecodingError::IoError(e)) if e.kind() == ErrorKind::UnexpectedEof => {
                streaming_input.expose_next_byte()
            }
            error => {
                return Err(std::io::Error::other(format!(
                    "unexpected streaming decode error: {error:?}"
                ))
                .into());
            }
        }
    }
    assert_eq!(
        crc32fast::hash(&decoded_from_whole_input),
        crc32fast::hash(&decoded_from_streaming_input)
    );
    Ok(())
}

/// Test resuming/retrying `Decoder.read_header_info` after `UnexpectedEof`.
#[test]
fn test_streaming_input_and_reading_header_info()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    const WIDTH: u32 = 16;
    const IDAT_SIZE: usize = 512;
    let streaming_input = StreamingInput::with_noncompressed_png(WIDTH, IDAT_SIZE)?;

    let info_from_whole_input = streaming_input.decode_full_input(|r| r.info().cloned())??;

    let mut decoder = Decoder::new(streaming_input.clone());
    let info_from_streaming_input = loop {
        match decoder.read_header_info() {
            Ok(info) => break info.clone(),
            Err(DecodingError::IoError(e)) if e.kind() == ErrorKind::UnexpectedEof => {
                streaming_input.expose_next_byte()
            }
            error => {
                return Err(std::io::Error::other(format!(
                    "unexpected streaming decode error: {error:?}"
                ))
                .into());
            }
        }
    };

    assert_eq!(info_from_whole_input.width, info_from_streaming_input.width);
    assert_eq!(
        info_from_whole_input.height,
        info_from_streaming_input.height
    );
    assert_eq!(
        info_from_whole_input.bit_depth,
        info_from_streaming_input.bit_depth
    );
    assert_eq!(
        info_from_whole_input.color_type,
        info_from_streaming_input.color_type
    );
    assert_eq!(
        info_from_whole_input.interlaced,
        info_from_streaming_input.interlaced
    );
    Ok(())
}
