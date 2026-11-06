use super::helpers::*;
use super::*;

fn required<T>(value: Option<T>, message: &str) -> std::result::Result<T, std::io::Error> {
    value.ok_or_else(|| std::io::Error::other(message))
}

fn expected_error<T, E: std::fmt::Debug>(
    result: std::result::Result<T, E>,
    message: &str,
) -> std::result::Result<E, std::io::Error> {
    result.err().ok_or_else(|| std::io::Error::other(message))
}

#[test]
fn test_fdat_chunk_without_idat() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let png = {
        let width = 1;
        let mut png = Vec::new();
        write_png_sig(&mut png)?;
        write_rgba8_ihdr_with_width(&mut png, width)?;
        let image_data = generate_rgba8_with_width_and_height(width, width)?;
        write_actl(
            &mut png,
            &crate::sample::embedded_png::png::AnimationControl {
                num_frames: 2,
                num_plays: 1,
            },
        )?;
        let mut fctl = crate::sample::embedded_png::png::FrameControl {
            sequence_number: 0,
            width,
            height: width,
            ..Default::default()
        };
        write_fctl(&mut png, &fctl)?;
        fctl.sequence_number = 1;
        write_fctl(&mut png, &fctl)?;
        write_fdat(&mut png, 1, &image_data[..])?;
        write_iend(&mut png)?;
        png
    };
    let decoder = Decoder::new(Cursor::new(&png));
    let err = expected_error(decoder.read_info(), "fDAT without IDAT must fail")?;
    assert!(matches!(&err, DecodingError::Format(_)));
    assert_eq!(
        "Unexpected restart of ChunkType { type: fdAT, \
                         critical: false, \
                         private: true, \
                         reserved: false, \
                         safecopy: false \
             } chunk sequence",
        format!("{err}"),
    );
    Ok(())
}

#[test]
fn test_fdat_chunk_payload_length_0() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut png = Vec::new();
    write_fdat_prefix(&mut png, 2, 8)?;
    write_chunk(&mut png, b"fdAT", &[])?;

    let decoder = Decoder::new(Cursor::new(&png));
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; required(reader.output_buffer_size()?, "output buffer size missing")?];
    reader.next_frame(&mut buf)?;

    // 0-length fdAT should result in an error.
    let err = expected_error(reader.next_frame(&mut buf), "empty fdAT payload must fail")?;
    assert!(matches!(&err, DecodingError::Format(_)));
    assert_eq!("fdAT chunk shorter than 4 bytes", format!("{err}"));

    // Calling `next_frame` again should return an error.  Same error as above would be nice,
    // but it is probably unnecessary and infeasible (`DecodingError` can't derive `Clone`
    // because `std::io::Error` doesn't implement `Clone`)..  But it definitely shouldn't enter
    // an infinite loop.
    let err2 = expected_error(
        reader.next_frame(&mut buf),
        "decoder must remain in fatal state",
    )?;
    assert!(matches!(&err2, DecodingError::Parameter(_)));
    assert_eq!(
        "A fatal decoding error has been encounted earlier",
        format!("{err2}")
    );
    Ok(())
}

#[test]
fn test_fdat_chunk_payload_length_3() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut png = Vec::new();
    write_fdat_prefix(&mut png, 2, 8)?;
    write_chunk(&mut png, b"fdAT", &[1, 0, 0])?;

    let decoder = Decoder::new(Cursor::new(&png));
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; required(reader.output_buffer_size()?, "output buffer size missing")?];
    reader.next_frame(&mut buf)?;

    // 3-bytes-long fdAT should result in an error.
    let err = expected_error(reader.next_frame(&mut buf), "short fdAT payload must fail")?;
    assert!(matches!(&err, DecodingError::Format(_)));
    assert_eq!("fdAT chunk shorter than 4 bytes", format!("{err}"));
    Ok(())
}

#[test]
fn test_frame_split_across_two_fdat_chunks() -> std::result::Result<(), Box<dyn std::error::Error>>
{
    // Generate test data where the 2nd animation frame is split across 2 fdAT chunks.
    //
    // This is similar to the example given in
    // https://wiki.mozilla.org/APNG_Specification#Chunk_Sequence_Numbers:
    //
    // ```
    //    Sequence number    Chunk
    //    (none)             `acTL`
    //    0                  `fcTL` first frame
    //    (none)             `IDAT` first frame / default image
    //    1                  `fcTL` second frame
    //    2                  first `fdAT` for second frame
    //    3                  second `fdAT` for second frame
    // ```
    let png = {
        let mut png = Vec::new();
        write_fdat_prefix(&mut png, 2, 8)?;
        let image_data = generate_rgba8_with_width_and_height(8, 8)?;
        write_fdat(&mut png, 2, &image_data[..30])?;
        write_fdat(&mut png, 3, &image_data[30..])?;
        write_iend(&mut png)?;
        png
    };

    // Start decoding.
    let decoder = Decoder::new(Cursor::new(&png));
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; required(reader.output_buffer_size()?, "output buffer size missing")?];
    let animation_control = required(reader.info()?.animation_control, "acTL chunk missing")?;
    assert_eq!(animation_control.num_frames, 2);

    // Process the 1st animation frame.
    let first_frame: Vec<u8>;
    {
        reader.next_frame(&mut buf)?;
        first_frame = buf.clone();

        // Note that the doc comment of `Reader::next_frame` says that "[...]
        // can be checked afterwards by calling `info` **after** a successful call and
        // inspecting the `frame_control` data.".  (Note the **emphasis** on "after".)
        let frame_control = required(reader.info()?.frame_control, "first fcTL chunk missing")?;
        // The sequence number is taken from the `fcTL` chunk that comes before the `IDAT`
        // chunk.
        assert_eq!(frame_control.sequence_number, 0);
    }

    // Process the 2nd animation frame.
    let second_frame: Vec<u8>;
    {
        reader.next_frame(&mut buf)?;
        second_frame = buf.clone();

        // Same as above - updated `frame_control` is available *after* the `next_frame` call.
        let frame_control = required(reader.info()?.frame_control, "second fcTL chunk missing")?;
        // The sequence number is taken from the `fcTL` chunk that comes before the two `fdAT`
        // chunks.  Note that sequence numbers inside `fdAT` chunks are not publicly exposed
        // (but they are still checked when decoding to verify that they are sequential).
        assert_eq!(frame_control.sequence_number, 1);
    }

    assert_eq!(first_frame, second_frame);
    Ok(())
}

#[test]
fn test_idat_bigger_than_image_size_from_ihdr()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let png = {
        let mut png = Vec::new();
        write_png_sig(&mut png)?;
        write_rgba8_ihdr_with_width(&mut png, 8)?;

        // Here we want to test an invalid image where the `IDAT` chunk contains more data
        // (data for 8x256 image) than declared in the `IHDR` chunk (which only describes an
        // 8x8 image).
        write_chunk(
            &mut png,
            b"IDAT",
            &generate_rgba8_with_width_and_height(8, 256)?,
        )?;

        write_iend(&mut png)?;
        png
    };
    let decoder = Decoder::new(Cursor::new(&png));
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; required(reader.output_buffer_size()?, "output buffer size missing")?];

    // TODO: Should this return an error instead?  For now let's just have test assertions for
    // the current behavior.
    reader.next_frame(&mut buf)?;
    assert_eq!(3093270825, crc32fast::hash(&buf));
    Ok(())
}
