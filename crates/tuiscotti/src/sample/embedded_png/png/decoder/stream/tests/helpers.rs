use super::*;

/// Writes an acTL chunk.
/// See https://wiki.mozilla.org/APNG_Specification#.60acTL.60:_The_Animation_Control_Chunk
pub(super) fn write_actl(
    w: &mut impl Write,
    animation: &crate::sample::embedded_png::png::AnimationControl,
) -> std::io::Result<()> {
    let mut data = Vec::new();
    data.write_u32::<byteorder::BigEndian>(animation.num_frames)?;
    data.write_u32::<byteorder::BigEndian>(animation.num_plays)?;
    write_chunk(w, b"acTL", &data)
}

/// Writes an fcTL chunk.
/// See https://wiki.mozilla.org/APNG_Specification#.60fcTL.60:_The_Frame_Control_Chunk
pub(super) fn write_fctl(
    w: &mut impl Write,
    frame: &crate::sample::embedded_png::png::FrameControl,
) -> std::io::Result<()> {
    let mut data = Vec::new();
    data.write_u32::<byteorder::BigEndian>(frame.sequence_number)?;
    data.write_u32::<byteorder::BigEndian>(frame.width)?;
    data.write_u32::<byteorder::BigEndian>(frame.height)?;
    data.write_u32::<byteorder::BigEndian>(frame.x_offset)?;
    data.write_u32::<byteorder::BigEndian>(frame.y_offset)?;
    data.write_u16::<byteorder::BigEndian>(frame.delay_num)?;
    data.write_u16::<byteorder::BigEndian>(frame.delay_den)?;
    data.write_u8(frame.dispose_op as u8)?;
    data.write_u8(frame.blend_op as u8)?;
    write_chunk(w, b"fcTL", &data)
}

/// Writes an fdAT chunk.
/// See https://wiki.mozilla.org/APNG_Specification#.60fdAT.60:_The_Frame_Data_Chunk
pub(super) fn write_fdat(
    w: &mut impl Write,
    sequence_number: u32,
    image_data: &[u8],
) -> std::io::Result<()> {
    let mut data = Vec::new();
    data.write_u32::<byteorder::BigEndian>(sequence_number)?;
    data.write_all(image_data)?;
    write_chunk(w, b"fdAT", &data)
}

/// Writes PNG signature and chunks that can precede an fdAT chunk that is expected
/// to have
/// - `sequence_number` set to 0
/// - image data with rgba8 pixels in a `width` by `width` image
pub(super) fn write_fdat_prefix(
    w: &mut impl Write,
    num_frames: u32,
    width: u32,
) -> std::io::Result<()> {
    write_png_sig(w)?;
    write_rgba8_ihdr_with_width(w, width)?;
    write_actl(
        w,
        &crate::sample::embedded_png::png::AnimationControl {
            num_frames,
            num_plays: 0,
        },
    )?;

    let mut fctl = crate::sample::embedded_png::png::FrameControl {
        width,
        height: width,
        ..Default::default()
    };
    write_fctl(w, &fctl)?;
    write_rgba8_idats(w, width, 0x7fffffff)?;

    fctl.sequence_number += 1;
    write_fctl(w, &fctl)?;
    Ok(())
}
/// Creates a ready-to-test [`Reader`] which decodes a PNG that contains:
/// IHDR, IDAT, IEND.
pub(super) fn create_reader_of_ihdr_idat()
-> std::result::Result<Reader<Cursor<Vec<u8>>>, Box<dyn std::error::Error>> {
    let mut png = Vec::new();
    write_noncompressed_png(&mut png, /* width = */ 16, /* idat_size = */ 1024)?;
    Ok(Decoder::new(Cursor::new(png)).read_info()?)
}

/// Creates a ready-to-test [`Reader`] which decodes an animated PNG that contains:
/// IHDR, acTL, fcTL, IDAT, fcTL, fdAT, IEND.  (i.e. IDAT is part of the animation)
pub(super) fn create_reader_of_ihdr_actl_fctl_idat_fctl_fdat()
-> std::result::Result<Reader<Cursor<Vec<u8>>>, Box<dyn std::error::Error>> {
    let width = 16;
    let mut fctl = crate::sample::embedded_png::png::FrameControl {
        width,
        height: width,
        ..Default::default()
    };

    let mut png = Vec::new();
    write_png_sig(&mut png)?;
    write_rgba8_ihdr_with_width(&mut png, width)?;
    write_actl(
        &mut png,
        &crate::sample::embedded_png::png::AnimationControl {
            num_frames: 2,
            num_plays: 0,
        },
    )?;
    fctl.sequence_number = 0;
    write_fctl(&mut png, &fctl)?;
    // Using `fctl.height + 1` means that the `IDAT` will have "left-over" data after
    // processing.  This helps to verify that `Reader.read_until_image_data` discards the
    // left-over data when resetting `UnfilteredRowsBuffer`.
    let idat_data = generate_rgba8_with_width_and_height(fctl.width, fctl.height + 1)?;
    write_chunk(&mut png, b"IDAT", &idat_data)?;

    let fdat_width = 10;
    fctl.sequence_number = 1;
    // Using different width in `IDAT` and `fDAT` frames helps to catch problems that
    // may arise when `Reader.read_until_image_data` doesn't properly reset
    // `UnfilteredRowsBuffer`.
    fctl.width = fdat_width;
    write_fctl(&mut png, &fctl)?;
    let fdat_data = generate_rgba8_with_width_and_height(fctl.width, fctl.height)?;
    write_fdat(&mut png, 2, &fdat_data)?;
    write_iend(&mut png)?;

    Ok(Decoder::new(Cursor::new(png)).read_info()?)
}

/// Creates a ready-to-test [`Reader`] which decodes an animated PNG that contains: IHDR, acTL,
/// IDAT, fcTL, fdAT, fcTL, fdAT, IEND.  (i.e. IDAT is *not* part of the animation)
pub(super) fn create_reader_of_ihdr_actl_idat_fctl_fdat_fctl_fdat()
-> std::result::Result<Reader<Cursor<Vec<u8>>>, Box<dyn std::error::Error>> {
    let width = 16;
    let frame_data = generate_rgba8_with_width_and_height(width, width)?;
    let mut fctl = crate::sample::embedded_png::png::FrameControl {
        width,
        height: width,
        ..Default::default()
    };

    let mut png = Vec::new();
    write_png_sig(&mut png)?;
    write_rgba8_ihdr_with_width(&mut png, width)?;
    write_actl(
        &mut png,
        &crate::sample::embedded_png::png::AnimationControl {
            num_frames: 2,
            num_plays: 0,
        },
    )?;
    write_chunk(&mut png, b"IDAT", &frame_data)?;
    fctl.sequence_number = 0;
    write_fctl(&mut png, &fctl)?;
    write_fdat(&mut png, 1, &frame_data)?;
    fctl.sequence_number = 2;
    write_fctl(&mut png, &fctl)?;
    write_fdat(&mut png, 3, &frame_data)?;
    write_iend(&mut png)?;

    Ok(Decoder::new(Cursor::new(png)).read_info()?)
}

pub(super) fn get_fctl_sequence_number(
    reader: &Reader<impl BufRead + Seek>,
) -> std::result::Result<u32, std::io::Error> {
    reader
        .info()
        .map_err(std::io::Error::other)?
        .frame_control
        .as_ref()
        .ok_or_else(|| std::io::Error::other("frame control is missing"))
        .map(|control| control.sequence_number)
}
