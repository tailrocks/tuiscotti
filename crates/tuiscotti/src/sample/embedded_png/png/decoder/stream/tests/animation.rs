use super::helpers::*;
use super::*;

fn required<T>(value: Option<T>, message: &str) -> std::result::Result<T, std::io::Error> {
    value.ok_or_else(|| std::io::Error::other(message))
}

#[test]
fn test_next_frame_polling_after_end_non_animated()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut reader = create_reader_of_ihdr_idat()?;
    let mut buf = vec![0; required(reader.output_buffer_size()?, "output buffer size missing")?];
    reader
        .next_frame(&mut buf)
        .expect("Expecting no error for IDAT frame");

    let err = reader
        .next_frame(&mut buf)
        .expect_err("Main test - expecting error");
    assert!(
        matches!(&err, DecodingError::Parameter(_)),
        "Unexpected kind of error: {:?}",
        &err,
    );
    Ok(())
}

/// Tests that [`Reader.next_frame_info`] will report a `PolledAfterEndOfImage` error when
/// called when decoding a PNG that only contains a single frame.
#[test]
fn test_next_frame_info_polling_after_end_non_animated()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut reader = create_reader_of_ihdr_idat()?;

    let err = reader
        .next_frame_info()
        .expect_err("Main test - expecting error");
    assert!(
        matches!(&err, DecodingError::Parameter(_)),
        "Unexpected kind of error: {:?}",
        &err,
    );
    Ok(())
}

/// Tests that [`Reader.next_frame`] will report a `PolledAfterEndOfImage` error when called
/// after already decoding a single frame in an animated PNG where IDAT is part of the
/// animation.
#[test]
fn test_next_frame_polling_after_end_idat_part_of_animation()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut reader = create_reader_of_ihdr_actl_fctl_idat_fctl_fdat()?;
    let mut buf = vec![0; required(reader.output_buffer_size()?, "output buffer size missing")?];

    assert_eq!(get_fctl_sequence_number(&reader)?, 0);
    reader
        .next_frame(&mut buf)
        .expect("Expecting no error for IDAT frame");

    // `next_frame` doesn't advance to the next `fcTL`.
    assert_eq!(get_fctl_sequence_number(&reader)?, 0);

    reader
        .next_frame(&mut buf)
        .expect("Expecting no error for fdAT frame");
    assert_eq!(get_fctl_sequence_number(&reader)?, 1);

    let err = reader
        .next_frame(&mut buf)
        .expect_err("Main test - expecting error");
    assert!(
        matches!(&err, DecodingError::Parameter(_)),
        "Unexpected kind of error: {:?}",
        &err,
    );
    Ok(())
}

/// Tests that [`Reader.next_frame`] will report a `PolledAfterEndOfImage` error when called
/// after already decoding a single frame in an animated PNG where IDAT is *not* part of the
/// animation.
#[test]
fn test_next_frame_polling_after_end_idat_not_part_of_animation()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut reader = create_reader_of_ihdr_actl_idat_fctl_fdat_fctl_fdat()?;
    let mut buf = vec![0; required(reader.output_buffer_size()?, "output buffer size missing")?];

    assert!(reader.info()?.frame_control.is_none());
    reader
        .next_frame(&mut buf)
        .expect("Expecting no error for IDAT frame");

    // `next_frame` doesn't advance to the next `fcTL`.
    assert!(reader.info()?.frame_control.is_none());

    reader
        .next_frame(&mut buf)
        .expect("Expecting no error for 1st fdAT frame");
    assert_eq!(get_fctl_sequence_number(&reader)?, 0);

    reader
        .next_frame(&mut buf)
        .expect("Expecting no error for 2nd fdAT frame");
    assert_eq!(get_fctl_sequence_number(&reader)?, 2);

    let err = reader
        .next_frame(&mut buf)
        .expect_err("Main test - expecting error");
    assert!(
        matches!(&err, DecodingError::Parameter(_)),
        "Unexpected kind of error: {:?}",
        &err,
    );
    Ok(())
}

/// Tests that after decoding a whole frame via [`Reader.next_row`] the call to
/// [`Reader.next_frame`] will decode the **next** frame.
#[test]
fn test_row_by_row_then_next_frame() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut reader = create_reader_of_ihdr_actl_fctl_idat_fctl_fdat()?;
    let mut buf = vec![0; required(reader.output_buffer_size()?, "output buffer size missing")?];

    assert_eq!(get_fctl_sequence_number(&reader)?, 0);
    while let Some(_) = reader.next_row()? {}
    assert_eq!(get_fctl_sequence_number(&reader)?, 0);

    buf.fill(0x0f);
    reader
        .next_frame(&mut buf)
        .expect("Expecting no error from next_frame call");

    // Verify if we have read the next `fcTL` chunk + repopulated `buf`:
    assert_eq!(get_fctl_sequence_number(&reader)?, 1);
    assert!(buf.iter().any(|byte| *byte != 0x0f));
    Ok(())
}

/// Tests that after decoding a whole frame via [`Reader.next_row`] it is possible
/// to use [`Reader.next_row`] to decode the next frame (by using the `next_frame_info` API to
/// advance to the next frame when `next_row` returns `None`).
#[test]
fn test_row_by_row_of_two_frames() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut reader = create_reader_of_ihdr_actl_fctl_idat_fctl_fdat()?;

    let mut rows_of_frame1 = 0;
    assert_eq!(get_fctl_sequence_number(&reader)?, 0);
    while let Some(_) = reader.next_row()? {
        rows_of_frame1 += 1;
    }
    assert_eq!(rows_of_frame1, 16);
    assert_eq!(get_fctl_sequence_number(&reader)?, 0);

    let mut rows_of_frame2 = 0;
    assert_eq!(reader.next_frame_info()?.sequence_number, 1);
    assert_eq!(get_fctl_sequence_number(&reader)?, 1);
    while let Some(_) = reader.next_row()? {
        rows_of_frame2 += 1;
    }
    assert_eq!(rows_of_frame2, 16);
    assert_eq!(get_fctl_sequence_number(&reader)?, 1);

    let err = reader
        .next_frame_info()
        .expect_err("No more frames - expecting error");
    assert!(
        matches!(&err, DecodingError::Parameter(_)),
        "Unexpected kind of error: {:?}",
        &err,
    );
    Ok(())
}

/// This test is similar to `test_next_frame_polling_after_end_idat_part_of_animation`, but it
/// uses `next_frame_info` calls to read to the next `fcTL` earlier - before the next call to
/// `next_frame` (knowing `fcTL` before calling `next_frame` may be helpful to determine the
/// size of the output buffer and/or to prepare the buffer based on the `DisposeOp` of the
/// previous frames).
#[test]
fn test_next_frame_info_after_next_frame() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut reader = create_reader_of_ihdr_actl_fctl_idat_fctl_fdat()?;
    let mut buf = vec![0; required(reader.output_buffer_size()?, "output buffer size missing")?];

    assert_eq!(get_fctl_sequence_number(&reader)?, 0);
    reader
        .next_frame(&mut buf)
        .expect("Expecting no error for IDAT frame");

    // `next_frame` doesn't advance to the next `fcTL`.
    assert_eq!(get_fctl_sequence_number(&reader)?, 0);

    // But `next_frame_info` can be used to go to the next `fcTL`.
    assert_eq!(reader.next_frame_info()?.sequence_number, 1);
    assert_eq!(get_fctl_sequence_number(&reader)?, 1);

    reader
        .next_frame(&mut buf)
        .expect("Expecting no error for fdAT frame");
    assert_eq!(get_fctl_sequence_number(&reader)?, 1);

    let err = reader
        .next_frame_info()
        .expect_err("Main test - expecting error");
    assert!(
        matches!(&err, DecodingError::Parameter(_)),
        "Unexpected kind of error: {:?}",
        &err,
    );
    Ok(())
}

/// This test is similar to `test_next_frame_polling_after_end_idat_not_part_of_animation`, but
/// it uses `next_frame_info` to skip the `IDAT` frame entirely + to move between frames.
#[test]
fn test_next_frame_info_to_skip_first_frame() -> std::result::Result<(), Box<dyn std::error::Error>>
{
    let mut reader = create_reader_of_ihdr_actl_idat_fctl_fdat_fctl_fdat()?;
    let mut buf = vec![0; required(reader.output_buffer_size()?, "output buffer size missing")?];

    // First (IDAT) frame doesn't have frame control info, which means
    // that it is not part of the animation.
    assert!(reader.info()?.frame_control.is_none());

    // `next_frame_info` can be used to skip the IDAT frame (without first having to separately
    // discard the image data - e.g. by also calling `next_frame` first).
    assert_eq!(reader.next_frame_info()?.sequence_number, 0);
    assert_eq!(get_fctl_sequence_number(&reader)?, 0);
    reader
        .next_frame(&mut buf)
        .expect("Expecting no error for 1st fdAT frame");
    assert_eq!(get_fctl_sequence_number(&reader)?, 0);

    // Get the `fcTL` for the 2nd frame.
    assert_eq!(reader.next_frame_info()?.sequence_number, 2);
    reader
        .next_frame(&mut buf)
        .expect("Expecting no error for 2nd fdAT frame");
    assert_eq!(get_fctl_sequence_number(&reader)?, 2);

    let err = reader
        .next_frame_info()
        .expect_err("Main test - expecting error");
    assert!(
        matches!(&err, DecodingError::Parameter(_)),
        "Unexpected kind of error: {:?}",
        &err,
    );
    Ok(())
}
