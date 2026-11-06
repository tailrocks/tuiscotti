use super::*;
use super::*;
use crate::sample::embedded_png::png::DeflateCompression;
use std::io;

use super::*;
use crate::sample::embedded_png::png::Decoder;

use io::BufReader;
use rand::{Rng, rng};
use std::cmp;
use std::fs::File;
use std::io::Cursor;

fn roundtrip_inner() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // More loops = more random testing, but also more test wait time
    for _ in 0..5 {
        for path in glob::glob("tests/pngsuite/*.png")? {
            let path = path?;
            let filename = path
                .file_name()
                .ok_or_else(|| io::Error::other("PNG fixture has no filename"))?;
            if filename
                .to_str()
                .ok_or_else(|| io::Error::other("PNG fixture filename is not UTF-8"))?
                .starts_with('x')
            {
                // x* files are expected to fail to decode
                continue;
            }
            eprintln!("{}", path.display());
            // Decode image
            let decoder = Decoder::new(BufReader::new(File::open(path)?));
            let mut reader = decoder.read_info()?;
            let mut buf =
                vec![
                    0;
                    reader
                        .output_buffer_size()?
                        .ok_or_else(|| io::Error::other("decoded image size is unavailable"))?
                ];
            let info = reader.next_frame(&mut buf)?;
            use DeflateCompression::*;
            for compression in [NoCompression, FdeflateUltraFast, Level(4)] {
                // Encode decoded image
                let mut out = Vec::new();
                {
                    let mut wrapper = RandomChunkWriter {
                        rng: rng(),
                        w: &mut out,
                    };

                    let mut encoder = Encoder::new(&mut wrapper, info.width, info.height);
                    encoder.set_color(info.color_type);
                    encoder.set_depth(info.bit_depth);
                    encoder.set_deflate_compression(compression);
                    if let Some(palette) = &reader.info()?.palette {
                        encoder.set_palette(palette.clone());
                    }
                    let mut encoder = encoder.write_header()?;
                    encoder.write_image_data(&buf)?;
                }
                // Decode encoded decoded image
                let decoder = Decoder::new(Cursor::new(&*out));
                let mut reader = decoder.read_info()?;
                let mut buf2 =
                    vec![
                        0;
                        reader
                            .output_buffer_size()?
                            .ok_or_else(|| io::Error::other("decoded image size is unavailable"))?
                    ];
                reader.next_frame(&mut buf2)?;
                // check if the encoded image is ok:
                assert_eq!(buf, buf2);
            }
        }
    }
    Ok(())
}

fn roundtrip_stream_inner() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // More loops = more random testing, but also more test wait time
    for _ in 0..5 {
        for path in glob::glob("tests/pngsuite/*.png")? {
            let path = path?;
            let filename = path
                .file_name()
                .ok_or_else(|| io::Error::other("PNG fixture has no filename"))?;
            if filename
                .to_str()
                .ok_or_else(|| io::Error::other("PNG fixture filename is not UTF-8"))?
                .starts_with('x')
            {
                // x* files are expected to fail to decode
                continue;
            }
            // Decode image
            let decoder = Decoder::new(BufReader::new(File::open(path)?));
            let mut reader = decoder.read_info()?;
            let mut buf =
                vec![
                    0;
                    reader
                        .output_buffer_size()?
                        .ok_or_else(|| io::Error::other("decoded image size is unavailable"))?
                ];
            let info = reader.next_frame(&mut buf)?;
            use DeflateCompression::*;
            for compression in [NoCompression, FdeflateUltraFast, Level(4)] {
                // Encode decoded image
                let mut out = Vec::new();
                {
                    let mut wrapper = RandomChunkWriter {
                        rng: rng(),
                        w: &mut out,
                    };

                    let mut encoder = Encoder::new(&mut wrapper, info.width, info.height);
                    encoder.set_color(info.color_type);
                    encoder.set_depth(info.bit_depth);
                    encoder.set_deflate_compression(compression);
                    if let Some(palette) = &reader.info()?.palette {
                        encoder.set_palette(palette.clone());
                    }
                    let mut encoder = encoder.write_header()?;
                    let mut stream_writer = encoder.stream_writer()?;

                    let mut outer_wrapper = RandomChunkWriter {
                        rng: rng(),
                        w: &mut stream_writer,
                    };

                    outer_wrapper.write_all(&buf)?;
                }
                // Decode encoded decoded image
                let decoder = Decoder::new(Cursor::new(&*out));
                let mut reader = decoder.read_info()?;
                let mut buf2 =
                    vec![
                        0;
                        reader
                            .output_buffer_size()?
                            .ok_or_else(|| io::Error::other("decoded image size is unavailable"))?
                    ];
                reader.next_frame(&mut buf2)?;
                // check if the encoded image is ok:
                assert_eq!(buf, buf2);
            }
        }
    }
    Ok(())
}

fn test_stream_flushing(
    compression: Compression,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let output = vec![0u8; 1024];
    let mut cursor = Cursor::new(output);

    let mut encoder = Encoder::new(&mut cursor, 8, 8);
    encoder.set_color(ColorType::Rgba);
    encoder.set_compression(compression);
    let mut writer = encoder.write_header()?;
    let mut stream = writer.stream_writer()?;

    for _ in 0..8 {
        let written = stream.write(&[1; 32])?;
        assert_eq!(written, 32);
        stream.flush()?;
    }
    stream.finish()?;
    drop(writer);

    {
        cursor.set_position(0);
        let mut decoder = Decoder::new(cursor).read_info()?;
        let mut buffer = [0u8; 256];
        decoder.next_frame(&mut buffer[..])?;
        assert_eq!(buffer, [1; 256]);
    }

    Ok(())
}

/// A Writer that only writes a few bytes at a time
struct RandomChunkWriter<R: Rng, W: Write> {
    rng: R,
    w: W,
}

impl<R: Rng, W: Write> Write for RandomChunkWriter<R, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // choose a random length to write
        let len = cmp::min(self.rng.random_range(1..50), buf.len());

        self.w.write(&buf[0..len])
    }

    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}

#[path = "encoder/tests/errors.rs"]
mod errors;
#[path = "encoder/tests/roundtrip.rs"]
mod roundtrip;
#[path = "encoder/tests/stream.rs"]
mod stream;
