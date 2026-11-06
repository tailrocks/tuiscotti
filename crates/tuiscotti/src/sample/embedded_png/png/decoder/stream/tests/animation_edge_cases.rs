use super::helpers::*;
use super::*;

fn required<T>(value: Option<T>, message: &str) -> std::result::Result<T, std::io::Error> {
    value.ok_or_else(|| std::io::Error::other(message))
}

#[test]
fn test_incorrect_trns_chunk_is_ignored() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let png = {
        let mut png = Vec::new();
        write_png_sig(&mut png)?;
        write_rgba8_ihdr_with_width(&mut png, 8)?;
        write_chunk(&mut png, b"tRNS", &[12, 34, 56])?;
        write_chunk(
            &mut png,
            b"IDAT",
            &generate_rgba8_with_width_and_height(8, 8)?,
        )?;
        write_iend(&mut png)?;
        png
    };
    let decoder = Decoder::new(Cursor::new(&png));
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; required(reader.output_buffer_size()?, "output buffer size missing")?];
    assert!(reader.info()?.trns.is_none());
    reader.next_frame(&mut buf)?;
    assert_eq!(3093270825, crc32fast::hash(&buf));
    assert!(reader.info()?.trns.is_none());
    Ok(())
}

/// This is a regression test for https://crbug.com/422421347
#[test]
fn test_actl_num_frames_zero() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let width = 16;
    let frame_data = generate_rgba8_with_width_and_height(width, width)?;

    let mut png = Vec::new();
    write_png_sig(&mut png)?;
    write_rgba8_ihdr_with_width(&mut png, width)?;
    write_actl(
        &mut png,
        &crate::sample::embedded_png::png::AnimationControl {
            num_frames: 0, // <= spec violation needed by this test
            num_plays: 0,
        },
    )?;
    // Presence of an `fcTL` chunk will prevent incrementing
    // `num_frames` when calculating `remaining_frames` in
    // `Decoder::read_info`.  So the test writes an `fcTL` chunk
    // to end up with `remaining_frames == 0` if `parse_actl` allows
    // `num_frames == 0`.
    write_fctl(
        &mut png,
        &crate::sample::embedded_png::png::FrameControl {
            width,
            height: width,
            ..Default::default()
        },
    )?;
    write_chunk(&mut png, b"IDAT", &frame_data)?;
    write_iend(&mut png)?;

    let mut reader = Decoder::new(Cursor::new(png)).read_info()?;

    // Using `next_interlaced_row` in the test, because it doesn't check
    // `Reader::remaining_frames` (unlike `next_frame`), because it assumes that either
    // `read_info` or `next_frame` leave `Reader` in a valid state.
    //
    // The test passes if these `next_interlaced_row` calls don't hit any `assert!` failures.
    while let Some(_row) = reader.next_interlaced_row()? {}
    Ok(())
}

#[test]
fn test_small_fctl() -> std::result::Result<(), Box<dyn std::error::Error>> {
    const FCTL_SIZE: u32 = 30;
    const IHDR_SIZE: u32 = 50;
    let mut png = Vec::new();
    write_png_sig(&mut png)?;
    write_rgba8_ihdr_with_width(&mut png, IHDR_SIZE)?;
    write_actl(
        &mut png,
        &crate::sample::embedded_png::png::AnimationControl {
            num_frames: 1,
            num_plays: 1,
        },
    )?;
    write_fctl(
        &mut png,
        &crate::sample::embedded_png::png::FrameControl {
            width: FCTL_SIZE,
            height: FCTL_SIZE,
            x_offset: 10,
            y_offset: 10,
            sequence_number: 0,
            ..Default::default()
        },
    )?;
    write_chunk(
        &mut png,
        b"IDAT",
        &generate_rgba8_with_width_and_height(IHDR_SIZE, IHDR_SIZE)?,
    )?;
    write_iend(&mut png)?;

    let reader = Decoder::new(Cursor::new(png)).read_info();
    let err = reader
        .err()
        .ok_or_else(|| std::io::Error::other("out-of-bounds frame control must fail"))?;
    assert!(matches!(&err, DecodingError::Format(_)));
    assert_eq!("Sub frame is out-of-bounds.", format!("{err}"));
    Ok(())
}

#[test]
fn test_invalid_text_chunk() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // The spec requires a NUL character (separating keyword from text) within the first 80
    // bytes of the chunk.  Here there is no NUL character in the first 100 bytes, so this
    // chunk is invalid and should trigger an error in `parse_text`.
    let invalid_text_chunk = vec![b'A'; 100];

    const SIZE: u32 = 20;
    let mut png = Vec::new();
    write_png_sig(&mut png)?;
    write_rgba8_ihdr_with_width(&mut png, SIZE)?;
    write_chunk(&mut png, b"tEXt", invalid_text_chunk.as_slice())?;
    write_chunk(
        &mut png,
        b"IDAT",
        &generate_rgba8_with_width_and_height(SIZE, SIZE)?,
    )?;
    write_iend(&mut png)?;

    let reader = Decoder::new(Cursor::new(png)).read_info()?;
    let info = reader.info()?;
    assert_eq!(info.width, SIZE);
    assert_eq!(info.uncompressed_latin1_text.len(), 0);
    Ok(())
}

/// This is a regression test for https://crbug.com/451710590.
#[test]
fn test_duplicate_actl_chunk() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let width = 16;
    let frame_data = generate_rgba8_with_width_and_height(width, width)?;

    let mut png = Vec::new();
    write_png_sig(&mut png)?;
    write_rgba8_ihdr_with_width(&mut png, width)?;
    write_actl(
        &mut png,
        &crate::sample::embedded_png::png::AnimationControl {
            num_frames: 2,
            num_plays: 123,
        },
    )?;
    write_actl(
        &mut png,
        &crate::sample::embedded_png::png::AnimationControl {
            num_frames: 1, // <- should be ignored
            num_plays: 456,
        },
    )?;
    write_chunk(&mut png, b"IDAT", &frame_data)?;
    write_iend(&mut png)?;

    let reader = Decoder::new(Cursor::new(png)).read_info()?;
    let actl = required(
        reader.info()?.animation_control.as_ref(),
        "animation control missing",
    )?;
    assert_eq!(actl.num_frames, 2);
    assert_eq!(actl.num_plays, 123);
    Ok(())
}
