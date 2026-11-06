use super::*;
use std::error::Error;

#[test]
fn roundtrip_stream1() -> std::result::Result<(), Box<dyn Error>> {
    roundtrip_stream_inner()
}

#[test]
fn roundtrip_stream2() -> std::result::Result<(), Box<dyn Error>> {
    roundtrip_stream_inner()
}

#[test]
fn issue_307_stream_validation() -> std::result::Result<(), Box<dyn Error>> {
    let output = vec![0u8; 1024];
    let mut cursor = Cursor::new(output);

    let encoder = Encoder::new(&mut cursor, 1, 1); // Create a 1-pixel image
    let mut writer = encoder.write_header()?;
    let mut stream = writer.stream_writer()?;

    let written = stream.write(&[1, 2, 3, 4])?;
    assert_eq!(written, 1);
    stream.finish()?;
    drop(writer);

    {
        cursor.set_position(0);
        let mut decoder = Decoder::new(cursor).read_info()?;
        let mut buffer = [0u8; 1];
        decoder.next_frame(&mut buffer[..])?;
        assert_eq!(buffer, [1]);
    }

    Ok(())
}

#[test]
fn stream_filtering() -> std::result::Result<(), Box<dyn Error>> {
    let output = vec![0u8; 1024];
    let mut cursor = Cursor::new(output);

    let mut encoder = Encoder::new(&mut cursor, 8, 8);
    encoder.set_color(ColorType::Rgba);
    encoder.set_filter(Filter::Paeth);
    let mut writer = encoder.write_header()?;
    let mut stream = writer.stream_writer()?;

    for _ in 0..8 {
        let written = stream.write(&[1; 32])?;
        assert_eq!(written, 32);
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

#[test]
fn stream_flushing_with_high_compression() -> std::result::Result<(), Box<dyn Error>> {
    test_stream_flushing(Compression::High)
}

#[test]
fn stream_flushing_with_balanced_compression() -> std::result::Result<(), Box<dyn Error>> {
    test_stream_flushing(Compression::Balanced)
}

#[test]
fn stream_flushing_with_fast_compression() -> std::result::Result<(), Box<dyn Error>> {
    test_stream_flushing(Compression::Fast)
}

#[test]
fn stream_flushing_with_fastest_compression() -> std::result::Result<(), Box<dyn Error>> {
    test_stream_flushing(Compression::Fastest)
}

#[test]
fn stream_flushing_with_no_compression() -> std::result::Result<(), Box<dyn Error>> {
    test_stream_flushing(Compression::NoCompression)
}

#[test]
fn finish_drops_inner_writer() -> Result<()> {
    struct NoWriter<'flag>(&'flag mut bool);

    impl Write for NoWriter<'_> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    impl Drop for NoWriter<'_> {
        fn drop(&mut self) {
            *self.0 = true;
        }
    }

    let mut flag = false;

    {
        let mut encoder = Encoder::new(NoWriter(&mut flag), 10, 10);
        encoder.set_depth(BitDepth::Eight);
        encoder.set_color(ColorType::Grayscale);

        let mut writer = encoder.write_header()?;
        writer.write_image_data(&[0; 100])?;
        writer.finish()?;
    }

    assert!(flag, "PNG finished but writer was not dropped");
    Ok(())
}
