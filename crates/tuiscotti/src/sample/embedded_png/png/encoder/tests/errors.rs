use super::*;

#[test]
fn expect_error_on_wrong_image_len() -> Result<()> {
    let width = 10;
    let height = 10;

    let output = vec![0u8; 1024];
    let writer = Cursor::new(output);
    let mut encoder = Encoder::new(writer, width as u32, height as u32);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_color(ColorType::Rgb);
    let mut png_writer = encoder.write_header()?;

    let correct_image_size = width * height * 3;
    let image = vec![0u8; correct_image_size + 1];
    let result = png_writer.write_image_data(image.as_ref());
    assert!(result.is_err());

    Ok(())
}

#[test]
fn expect_error_on_empty_image() -> Result<()> {
    let output = vec![0u8; 1024];
    let mut writer = Cursor::new(output);

    let encoder = Encoder::new(&mut writer, 0, 0);
    assert!(encoder.write_header().is_err());

    let encoder = Encoder::new(&mut writer, 100, 0);
    assert!(encoder.write_header().is_err());

    let encoder = Encoder::new(&mut writer, 0, 100);
    assert!(encoder.write_header().is_err());

    Ok(())
}

#[test]
fn expect_error_on_invalid_bit_depth_color_type_combination() -> Result<()> {
    let output = vec![0u8; 1024];
    let mut writer = Cursor::new(output);

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::One);
    encoder.set_color(ColorType::Rgb);
    assert!(encoder.write_header().is_err());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::One);
    encoder.set_color(ColorType::GrayscaleAlpha);
    assert!(encoder.write_header().is_err());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::One);
    encoder.set_color(ColorType::Rgba);
    assert!(encoder.write_header().is_err());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Two);
    encoder.set_color(ColorType::Rgb);
    assert!(encoder.write_header().is_err());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Two);
    encoder.set_color(ColorType::GrayscaleAlpha);
    assert!(encoder.write_header().is_err());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Two);
    encoder.set_color(ColorType::Rgba);
    assert!(encoder.write_header().is_err());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Four);
    encoder.set_color(ColorType::Rgb);
    assert!(encoder.write_header().is_err());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Four);
    encoder.set_color(ColorType::GrayscaleAlpha);
    assert!(encoder.write_header().is_err());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Four);
    encoder.set_color(ColorType::Rgba);
    assert!(encoder.write_header().is_err());

    let mut encoder = Encoder::new(&mut writer, 1, 1);
    encoder.set_depth(BitDepth::Sixteen);
    encoder.set_color(ColorType::Indexed);
    assert!(encoder.write_header().is_err());

    Ok(())
}

#[test]
#[cfg(all(unix, not(target_pointer_width = "32")))]
fn exper_error_on_huge_chunk() -> Result<()> {
    // Okay, so we want a proper 4 GB chunk but not actually spend the memory for reserving it.
    // Let's rely on overcommit? Otherwise we got the rather dumb option of mmap-ing /dev/zero.
    let empty = vec![0; 1usize << 31];
    let writer = Cursor::new(vec![0u8; 1024]);

    let mut encoder = Encoder::new(writer, 10, 10);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_color(ColorType::Grayscale);
    let mut png_writer = encoder.write_header()?;

    assert!(png_writer.write_chunk(chunk::FD_AT, &empty).is_err());
    Ok(())
}

#[test]
#[cfg(all(unix, not(target_pointer_width = "32")))]
fn exper_error_on_non_u32_chunk() -> Result<()> {
    // Okay, so we want a proper 4 GB chunk but not actually spend the memory for reserving it.
    // Let's rely on overcommit? Otherwise we got the rather dumb option of mmap-ing /dev/zero.
    let empty = vec![0; 1usize << 32];
    let writer = Cursor::new(vec![0u8; 1024]);

    let mut encoder = Encoder::new(writer, 10, 10);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_color(ColorType::Grayscale);
    let mut png_writer = encoder.write_header()?;

    assert!(png_writer.write_chunk(chunk::FD_AT, &empty).is_err());
    Ok(())
}
