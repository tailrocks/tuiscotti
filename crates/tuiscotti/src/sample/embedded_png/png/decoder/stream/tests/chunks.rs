use super::*;

fn required<T>(value: Option<T>, message: &str) -> std::result::Result<T, std::io::Error> {
    value.ok_or_else(|| std::io::Error::other(message))
}

#[test]
fn image_source_sbit() -> std::result::Result<(), Box<dyn std::error::Error>> {
    fn trial(
        path: &str,
        expected: Option<Cow<[u8]>>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let decoder =
            crate::sample::embedded_png::png::Decoder::new(BufReader::new(File::open(path)?));
        let reader = decoder.read_info()?;
        let actual: Option<Cow<[u8]>> = reader.info()?.sbit.clone();
        assert!(actual == expected);
        Ok(())
    }

    trial("tests/sbit/g.png", Some(Cow::Owned(vec![5u8])))?;
    trial("tests/sbit/ga.png", Some(Cow::Owned(vec![5u8, 3u8])))?;
    trial(
        "tests/sbit/indexed.png",
        Some(Cow::Owned(vec![5u8, 6u8, 5u8])),
    )?;
    trial("tests/sbit/rgb.png", Some(Cow::Owned(vec![5u8, 6u8, 5u8])))?;
    trial(
        "tests/sbit/rgba.png",
        Some(Cow::Owned(vec![5u8, 6u8, 5u8, 8u8])),
    )?;
    Ok(())
}

/// Test handling of a PNG file that contains *two* iCCP chunks.
/// This is a regression test for https://github.com/image-rs/image/issues/1825.
#[test]
fn test_two_iccp_chunks() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // The test file has been taken from
    // https://github.com/image-rs/image/issues/1825#issuecomment-1321798639,
    // but the 2nd iCCP chunk has been altered manually (see the 2nd comment below for more
    // details).
    let decoder = crate::sample::embedded_png::png::Decoder::new(BufReader::new(File::open(
        "tests/bugfixes/issue#1825.png",
    )?));
    let reader = decoder.read_info()?;
    let icc_profile =
        required(reader.info()?.icc_profile.clone(), "iCCP profile missing")?.into_owned();

    // Assert that the contents of the *first* iCCP chunk are returned.
    //
    // Note that the 2nd chunk in the test file has been manually altered to have a different
    // content (`b"test iccp contents"`) which would have a different CRC (797351983).
    assert_eq!(4070462061, crc32fast::hash(&icc_profile));
    Ok(())
}

#[test]
fn test_iccp_roundtrip() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dummy_icc = b"I'm a profile";

    let mut info = crate::sample::embedded_png::png::Info::with_size(1, 1);
    info.icc_profile = Some(dummy_icc.into());
    let mut encoded_image = Vec::new();
    let enc = crate::sample::embedded_png::png::Encoder::with_info(&mut encoded_image, info)?;
    let mut enc = enc.write_header()?;
    enc.write_image_data(&[0])?;
    enc.finish()?;

    let dec = crate::sample::embedded_png::png::Decoder::new(Cursor::new(&encoded_image));
    let dec = dec.read_info()?;
    assert_eq!(
        dummy_icc,
        &**required(dec.info()?.icc_profile.as_ref(), "iCCP profile missing")?
    );
    Ok(())
}

#[test]
fn test_phys_roundtrip() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut info = crate::sample::embedded_png::png::Info::with_size(1, 1);
    info.pixel_dims = Some(crate::sample::embedded_png::png::PixelDimensions {
        xppu: 12,
        yppu: 34,
        unit: Unit::Meter,
    });
    let mut encoded_image = Vec::new();
    let enc = crate::sample::embedded_png::png::Encoder::with_info(&mut encoded_image, info)?;
    let mut enc = enc.write_header()?;
    enc.write_image_data(&[0])?;
    enc.finish()?;

    let dec = crate::sample::embedded_png::png::Decoder::new(Cursor::new(&encoded_image));
    let dec = dec.read_info()?;
    let phys = required(dec.info()?.pixel_dims.as_ref(), "pHYs dimensions missing")?;
    assert_eq!(phys.xppu, 12);
    assert_eq!(phys.yppu, 34);
    assert_eq!(phys.unit, Unit::Meter);
    Ok(())
}

#[test]
fn test_srgb_roundtrip() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut info = crate::sample::embedded_png::png::Info::with_size(1, 1);
    info.srgb = Some(SrgbRenderingIntent::Saturation);
    let mut encoded_image = Vec::new();
    let enc = crate::sample::embedded_png::png::Encoder::with_info(&mut encoded_image, info)?;
    let mut enc = enc.write_header()?;
    enc.write_image_data(&[0])?;
    enc.finish()?;

    let dec = crate::sample::embedded_png::png::Decoder::new(Cursor::new(&encoded_image));
    let dec = dec.read_info()?;
    assert_eq!(
        required(dec.info()?.srgb, "sRGB intent missing")?,
        SrgbRenderingIntent::Saturation
    );
    Ok(())
}

#[test]
fn test_png_with_broken_iccp() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let decoder = crate::sample::embedded_png::png::Decoder::new(BufReader::new(File::open(
        "tests/iccp/broken_iccp.png",
    )?));
    assert!(decoder.read_info().is_ok());
    let mut decoder = crate::sample::embedded_png::png::Decoder::new(BufReader::new(File::open(
        "tests/iccp/broken_iccp.png",
    )?));
    decoder.set_ignore_iccp_chunk(true);
    assert!(decoder.read_info().is_ok());
    Ok(())
}

/// Test handling of `cICP`, `mDCV`, and `cLLI` chunks.
#[test]
fn test_cicp_mdcv_and_clli_chunks() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut decoder = crate::sample::embedded_png::png::Decoder::new(BufReader::new(File::open(
        "tests/bugfixes/cicp_pq.png",
    )?));
    decoder.ignore_checksums(true);
    let reader = decoder.read_info()?;
    let info = reader.info()?;

    let cicp = required(info.coding_independent_code_points, "cICP chunk missing")?;
    assert_eq!(cicp.color_primaries, 9);
    assert_eq!(cicp.transfer_function, 16);
    assert_eq!(cicp.matrix_coefficients, 0);
    assert!(cicp.is_video_full_range_image);

    let mdcv = required(info.mastering_display_color_volume, "mDCV chunk missing")?;
    assert_relative_eq!(mdcv.chromaticities.red.0.into_value(), 0.680);
    assert_relative_eq!(mdcv.chromaticities.red.1.into_value(), 0.320);
    assert_relative_eq!(mdcv.chromaticities.green.0.into_value(), 0.265);
    assert_relative_eq!(mdcv.chromaticities.green.1.into_value(), 0.690);
    assert_relative_eq!(mdcv.chromaticities.blue.0.into_value(), 0.150);
    assert_relative_eq!(mdcv.chromaticities.blue.1.into_value(), 0.060);
    assert_relative_eq!(mdcv.chromaticities.white.0.into_value(), 0.3127);
    assert_relative_eq!(mdcv.chromaticities.white.1.into_value(), 0.3290);
    assert_relative_eq!(mdcv.min_luminance as f32 / 10_000.0, 0.01);
    assert_relative_eq!(mdcv.max_luminance as f32 / 10_000.0, 5000.0);

    let clli = required(info.content_light_level, "cLLI chunk missing")?;
    assert_relative_eq!(clli.max_content_light_level as f32 / 10_000.0, 4000.0);
    assert_relative_eq!(clli.max_frame_average_light_level as f32 / 10_000.0, 2627.0);
    Ok(())
}

/// Test handling of `eXIf` chunk.
#[test]
fn test_exif_chunk() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let decoder = crate::sample::embedded_png::png::Decoder::new(BufReader::new(File::open(
        "tests/bugfixes/F-exif-chunk-early.png",
    )?));
    let reader = decoder.read_info()?;
    let info = reader.info()?;
    let exif = required(info.exif_metadata.as_ref(), "eXIf chunk missing")?.as_ref();
    assert_eq!(exif.len(), 90);
    Ok(())
}

/// Tests what happens then [`Reader.finish`] is called twice.
#[test]
fn test_finishing_twice() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut png = Vec::new();
    write_noncompressed_png(&mut png, 16, 1024)?;
    let decoder = Decoder::new(Cursor::new(&png));
    let mut reader = decoder.read_info()?;

    // First call to `finish` - expecting success.
    reader.finish()?;

    // Second call to `finish` - expecting an error.
    let err = reader.finish().expect_err("second finish must fail");
    assert!(matches!(&err, DecodingError::Parameter(_)));
    assert_eq!("End of image has been reached", format!("{err}"));
    Ok(())
}
