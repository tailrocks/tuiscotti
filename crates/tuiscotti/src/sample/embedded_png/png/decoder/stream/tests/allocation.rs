use super::*;

fn decoder_with_chunk(bytes: &[u8], budget: usize) -> super::super::StreamingDecoder {
    let mut decoder = super::super::StreamingDecoder::new();
    decoder.current_chunk.raw_bytes.extend_from_slice(bytes);
    decoder.limits = crate::sample::embedded_png::png::Limits { bytes: budget };
    decoder.info = Some(crate::sample::embedded_png::png::Info::with_size(1, 1));
    decoder
}

#[test]
fn plte_copy_debits_its_requested_bytes() -> std::result::Result<(), DecodingError> {
    let bytes = [12, 34, 56];
    let mut decoder = decoder_with_chunk(&bytes, bytes.len());

    decoder.parse_plte()?;

    assert_eq!(decoder.limits.bytes, 0);
    let info = decoder.info.as_ref().ok_or(DecodingError::LimitsExceeded)?;
    let palette = info.palette.as_ref().ok_or(DecodingError::LimitsExceeded)?;
    assert_eq!(palette.as_ref(), &bytes);
    Ok(())
}

#[test]
fn sbit_copy_debits_its_requested_bytes() -> std::result::Result<(), DecodingError> {
    let bytes = [5, 6, 7];
    let mut decoder = decoder_with_chunk(&bytes, bytes.len());
    let info = decoder.info.as_mut().ok_or(DecodingError::LimitsExceeded)?;
    info.color_type = crate::sample::embedded_png::png::ColorType::Rgb;
    info.bit_depth = crate::sample::embedded_png::png::BitDepth::Eight;

    decoder.parse_sbit()?;

    assert_eq!(decoder.limits.bytes, 0);
    let info = decoder.info.as_ref().ok_or(DecodingError::LimitsExceeded)?;
    let sbit = info.sbit.as_ref().ok_or(DecodingError::LimitsExceeded)?;
    assert_eq!(sbit.as_ref(), &bytes);
    Ok(())
}

#[test]
fn trns_copy_debits_source_bytes_before_truncation() -> std::result::Result<(), DecodingError> {
    let bytes = [0, 42];
    let mut decoder = decoder_with_chunk(&bytes, bytes.len());
    let info = decoder.info.as_mut().ok_or(DecodingError::LimitsExceeded)?;
    info.color_type = crate::sample::embedded_png::png::ColorType::Grayscale;
    info.bit_depth = crate::sample::embedded_png::png::BitDepth::Eight;

    decoder.parse_trns()?;

    assert_eq!(decoder.limits.bytes, 0);
    assert_eq!(
        decoder
            .info
            .as_ref()
            .ok_or(DecodingError::LimitsExceeded)?
            .trns
            .as_ref()
            .ok_or(DecodingError::LimitsExceeded)?
            .as_ref(),
        &[42],
        "the limit charges the two-byte source copy even though the stored value is one byte"
    );
    Ok(())
}

#[test]
fn exif_copy_debits_its_requested_bytes() -> std::result::Result<(), DecodingError> {
    let bytes = [1, 2, 3, 4];
    let mut decoder = decoder_with_chunk(&bytes, bytes.len());

    decoder.parse_exif()?;

    assert_eq!(decoder.limits.bytes, 0);
    assert_eq!(
        decoder
            .info
            .as_ref()
            .ok_or(DecodingError::LimitsExceeded)?
            .exif_metadata
            .as_ref()
            .ok_or(DecodingError::LimitsExceeded)?
            .as_ref(),
        &bytes
    );
    Ok(())
}

#[test]
fn bkgd_copy_debits_its_requested_bytes() -> std::result::Result<(), DecodingError> {
    let bytes = [0, 7];
    let mut decoder = decoder_with_chunk(&bytes, bytes.len());
    let info = decoder.info.as_mut().ok_or(DecodingError::LimitsExceeded)?;
    info.color_type = crate::sample::embedded_png::png::ColorType::Grayscale;

    decoder.parse_bkgd()?;

    assert_eq!(decoder.limits.bytes, 0);
    let info = decoder.info.as_ref().ok_or(DecodingError::LimitsExceeded)?;
    let bkgd = info.bkgd.as_ref().ok_or(DecodingError::LimitsExceeded)?;
    assert_eq!(bkgd.as_ref(), &bytes);
    Ok(())
}

#[test]
fn ancillary_copy_fails_when_its_request_exceeds_remaining_limit()
-> std::result::Result<(), DecodingError> {
    let bytes = [12, 34, 56];
    let mut decoder = decoder_with_chunk(&bytes, bytes.len() - 1);

    assert!(matches!(
        decoder.parse_plte(),
        Err(DecodingError::LimitsExceeded)
    ));
    assert_eq!(decoder.limits.bytes, bytes.len() - 1);
    assert!(
        decoder
            .info
            .as_ref()
            .ok_or(DecodingError::LimitsExceeded)?
            .palette
            .is_none()
    );
    Ok(())
}
