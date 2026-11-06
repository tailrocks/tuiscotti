use super::*;
use std::error::Error;

#[test]
fn roundtrip1() -> std::result::Result<(), Box<dyn Error>> {
    roundtrip_inner()
}

#[test]
fn roundtrip2() -> std::result::Result<(), Box<dyn Error>> {
    roundtrip_inner()
}

#[test]
fn image_palette() -> std::result::Result<(), Box<dyn std::error::Error>> {
    for &bit_depth in &[1u8, 2, 4, 8] {
        // Do a reference decoding, choose a fitting palette image from pngsuite
        let path = format!("tests/pngsuite/basn3p0{}.png", bit_depth);
        let decoder = Decoder::new(BufReader::new(File::open(&path)?));
        let mut reader = decoder.read_info()?;

        let mut decoded_pixels = vec![
            0;
            reader.output_buffer_size()?.ok_or_else(|| {
                std::io::Error::other("decoded image size is unavailable")
            })?
        ];
        let info = reader.info()?;
        assert_eq!(
            info.width as usize * info.height as usize * usize::from(bit_depth),
            decoded_pixels.len() * 8
        );
        let info = reader.next_frame(&mut decoded_pixels)?;
        let indexed_data = decoded_pixels;

        let palette = reader
            .info()?
            .palette
            .as_ref()
            .ok_or_else(|| std::io::Error::other("indexed PNG is missing its palette"))?;
        let mut out = Vec::new();
        {
            let mut encoder = Encoder::new(&mut out, info.width, info.height);
            encoder.set_depth(
                BitDepth::from_u8(bit_depth)
                    .ok_or_else(|| std::io::Error::other("invalid PNG bit depth"))?,
            );
            encoder.set_color(ColorType::Indexed);
            encoder.set_palette(palette.as_ref());

            let mut writer = encoder.write_header()?;
            writer.write_image_data(&indexed_data)?;
        }

        // Decode re-encoded image
        let decoder = Decoder::new(Cursor::new(&*out));
        let mut reader = decoder.read_info()?;
        let mut redecoded = vec![
            0;
            reader.output_buffer_size()?.ok_or_else(|| {
                std::io::Error::other("decoded image size is unavailable")
            })?
        ];
        reader.next_frame(&mut redecoded)?;
        // check if the encoded image is ok:
        assert_eq!(indexed_data, redecoded);
    }
    Ok(())
}

#[test]
fn can_write_header_with_valid_bit_depth_color_type_combination() -> Result<()> {
    let output = vec![0u8; 1024];
    let mut writer = Cursor::new(output);

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::One);
    encoder.set_color(ColorType::Grayscale);
    assert!(encoder.write_header().is_ok());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::One);
    encoder.set_color(ColorType::Indexed);
    assert!(encoder.write_header().is_ok());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Two);
    encoder.set_color(ColorType::Grayscale);
    assert!(encoder.write_header().is_ok());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Two);
    encoder.set_color(ColorType::Indexed);
    assert!(encoder.write_header().is_ok());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Four);
    encoder.set_color(ColorType::Grayscale);
    assert!(encoder.write_header().is_ok());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Four);
    encoder.set_color(ColorType::Indexed);
    assert!(encoder.write_header().is_ok());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_color(ColorType::Grayscale);
    assert!(encoder.write_header().is_ok());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_color(ColorType::Rgb);
    assert!(encoder.write_header().is_ok());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_color(ColorType::Indexed);
    assert!(encoder.write_header().is_ok());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_color(ColorType::GrayscaleAlpha);
    assert!(encoder.write_header().is_ok());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_color(ColorType::Rgba);
    assert!(encoder.write_header().is_ok());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Sixteen);
    encoder.set_color(ColorType::Grayscale);
    assert!(encoder.write_header().is_ok());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Sixteen);
    encoder.set_color(ColorType::Rgb);
    assert!(encoder.write_header().is_ok());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Sixteen);
    encoder.set_color(ColorType::GrayscaleAlpha);
    assert!(encoder.write_header().is_ok());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Sixteen);
    encoder.set_color(ColorType::Rgba);
    assert!(encoder.write_header().is_ok());

    Ok(())
}

#[test]
fn all_filters_roundtrip() -> io::Result<()> {
    let pixel: Vec<_> = (0..48).collect();

    let roundtrip = |filter: Filter| -> io::Result<()> {
        let mut buffer = vec![];
        let mut encoder = Encoder::new(&mut buffer, 4, 4);
        encoder.set_depth(BitDepth::Eight);
        encoder.set_color(ColorType::Rgb);
        encoder.set_filter(filter);
        encoder.write_header()?.write_image_data(&pixel)?;

        let decoder = crate::sample::embedded_png::png::Decoder::new(Cursor::new(buffer));
        let mut reader = decoder.read_info()?;
        let info = reader.info()?;
        assert_eq!(info.width, 4);
        assert_eq!(info.height, 4);
        let mut dest = vec![0; pixel.len()];
        reader.next_frame(&mut dest)?;
        assert_eq!(dest, pixel, "Deviation with filter type {:?}", filter);

        Ok(())
    };

    roundtrip(Filter::NoFilter)?;
    roundtrip(Filter::Sub)?;
    roundtrip(Filter::Up)?;
    roundtrip(Filter::Avg)?;
    roundtrip(Filter::Paeth)?;

    Ok(())
}

#[test]
fn some_gamma_roundtrip() -> io::Result<()> {
    let pixel: Vec<_> = (0..48).collect();

    let roundtrip = |gamma: Option<ScaledFloat>| -> io::Result<()> {
        let mut buffer = vec![];
        let mut encoder = Encoder::new(&mut buffer, 4, 4);
        encoder.set_depth(BitDepth::Eight);
        encoder.set_color(ColorType::Rgb);
        encoder.set_filter(Filter::Avg);
        if let Some(gamma) = gamma {
            encoder.set_source_gamma(gamma);
        }
        encoder.write_header()?.write_image_data(&pixel)?;

        let decoder = crate::sample::embedded_png::png::Decoder::new(Cursor::new(buffer));
        let mut reader = decoder.read_info()?;
        assert_eq!(
            reader.info()?.gamma(),
            gamma,
            "Deviation with gamma {:?}",
            gamma
        );
        let mut dest = vec![0; pixel.len()];
        let info = reader.next_frame(&mut dest)?;
        assert_eq!(info.width, 4);
        assert_eq!(info.height, 4);

        Ok(())
    };

    roundtrip(None)?;
    roundtrip(Some(ScaledFloat::new(0.35)))?;
    roundtrip(Some(ScaledFloat::new(0.45)))?;
    roundtrip(Some(ScaledFloat::new(0.55)))?;
    roundtrip(Some(ScaledFloat::new(0.7)))?;
    roundtrip(Some(ScaledFloat::new(1.0)))?;
    roundtrip(Some(ScaledFloat::new(2.5)))?;

    Ok(())
}

#[test]
fn write_image_chunks_beyond_first() -> Result<()> {
    let width = 10;
    let height = 10;

    let output = vec![0u8; 1024];
    let writer = Cursor::new(output);

    // Not an animation but we should still be able to write multiple images
    // See issue: <https://github.com/image-rs/image-png/issues/301>
    // This is technically all valid png so there is no issue with correctness.
    let mut encoder = Encoder::new(writer, width, height);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_color(ColorType::Grayscale);
    let mut png_writer = encoder.write_header()?;

    for _ in 0..3 {
        let correct_image_size = (width * height) as usize;
        let image = vec![0u8; correct_image_size];
        png_writer.write_image_data(image.as_ref())?;
    }

    Ok(())
}

#[test]
fn image_validate_sequence_without_animation() -> Result<()> {
    let width = 10;
    let height = 10;

    let output = vec![0u8; 1024];
    let writer = Cursor::new(output);

    let mut encoder = Encoder::new(writer, width, height);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_color(ColorType::Grayscale);
    encoder.validate_sequence(true);
    let mut png_writer = encoder.write_header()?;

    let correct_image_size = (width * height) as usize;
    let image = vec![0u8; correct_image_size];
    png_writer.write_image_data(image.as_ref())?;

    assert!(png_writer.write_image_data(image.as_ref()).is_err());
    Ok(())
}

#[test]
fn image_validate_animation() -> Result<()> {
    let width = 10;
    let height = 10;

    let output = vec![0u8; 1024];
    let writer = Cursor::new(output);
    let correct_image_size = (width * height) as usize;
    let image = vec![0u8; correct_image_size];

    let mut encoder = Encoder::new(writer, width, height);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_color(ColorType::Grayscale);
    encoder.set_animated(1, 0)?;
    encoder.validate_sequence(true);
    let mut png_writer = encoder.write_header()?;

    png_writer.write_image_data(image.as_ref())?;

    Ok(())
}

#[test]
fn image_validate_animation2() -> Result<()> {
    let width = 10;
    let height = 10;

    let output = vec![0u8; 1024];
    let writer = Cursor::new(output);
    let correct_image_size = (width * height) as usize;
    let image = vec![0u8; correct_image_size];

    let mut encoder = Encoder::new(writer, width, height);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_color(ColorType::Grayscale);
    encoder.set_animated(2, 0)?;
    encoder.validate_sequence(true);
    let mut png_writer = encoder.write_header()?;

    png_writer.write_image_data(image.as_ref())?;
    png_writer.write_image_data(image.as_ref())?;
    png_writer.finish()?;

    Ok(())
}

#[test]
fn image_validate_animation_sep_def_image() -> Result<()> {
    let width = 10;
    let height = 10;

    let output = vec![0u8; 1024];
    let writer = Cursor::new(output);
    let correct_image_size = (width * height) as usize;
    let image = vec![0u8; correct_image_size];

    let mut encoder = Encoder::new(writer, width, height);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_color(ColorType::Grayscale);
    encoder.set_animated(1, 0)?;
    encoder.set_sep_def_img(true)?;
    encoder.validate_sequence(true);
    let mut png_writer = encoder.write_header()?;

    png_writer.write_image_data(image.as_ref())?;
    png_writer.write_image_data(image.as_ref())?;
    png_writer.finish()?;

    Ok(())
}

#[test]
fn image_validate_missing_image() -> Result<()> {
    let width = 10;
    let height = 10;

    let output = vec![0u8; 1024];
    let writer = Cursor::new(output);

    let mut encoder = Encoder::new(writer, width, height);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_color(ColorType::Grayscale);
    encoder.validate_sequence(true);
    let png_writer = encoder.write_header()?;

    assert!(png_writer.finish().is_err());
    Ok(())
}

#[test]
fn image_validate_missing_animated_frame() -> Result<()> {
    let width = 10;
    let height = 10;

    let output = vec![0u8; 1024];
    let writer = Cursor::new(output);
    let correct_image_size = (width * height) as usize;
    let image = vec![0u8; correct_image_size];

    let mut encoder = Encoder::new(writer, width, height);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_color(ColorType::Grayscale);
    encoder.set_animated(2, 0)?;
    encoder.validate_sequence(true);
    let mut png_writer = encoder.write_header()?;

    png_writer.write_image_data(image.as_ref())?;
    assert!(png_writer.finish().is_err());

    Ok(())
}
