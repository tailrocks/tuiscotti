use super::*;
use std::error::Error;

#[derive(Default)]
struct Ledger {
    fail: Option<&'static str>,
    live: u64,
    peak: u64,
    events: Vec<(&'static str, u64, u64)>,
}
impl PngBudget for Ledger {
    type Token = u64;
    fn charge(&mut self, site: &'static str, bytes: u64) -> Result<u64, SampleError> {
        self.events.push((site, bytes, self.live));
        if self.fail == Some(site) {
            return Err(SampleError::ResourceLimit {
                resource: site,
                claimed: Some(bytes),
                limit: 0,
                source: None,
            });
        }
        self.live += bytes;
        self.peak = self.peak.max(self.live);
        Ok(bytes)
    }
    fn release(&mut self, token: u64) -> Result<(), SampleError> {
        self.live -= token;
        Ok(())
    }
}
#[test]
fn actual_png_boundary_charges_and_injected_limits() -> Result<(), Box<dyn Error>> {
    let bytes = include_bytes!("../../../tests/pngsuite/basn6a08.png");
    let mut ledger = Ledger::default();
    let pixels = decode_png(bytes, &mut ledger)?;
    eprintln!(
        "PNG input={} dimensions={}x{} output={} peak ledger={} events={:?}",
        bytes.len(),
        pixels.width,
        pixels.height,
        pixels.rgba.len(),
        ledger.peak,
        ledger.events
    );
    assert_eq!(ledger.events.len(), 3);
    assert_eq!(ledger.events[2].2, ledger.events[0].1 + ledger.events[1].1);
    assert_eq!(ledger.live, pixels.rgba.len() as u64);
    for site in [
        "PNG header and metadata",
        "PNG rows and transform",
        "PNG RGBA output",
    ] {
        let mut failed = Ledger {
            fail: Some(site),
            ..Ledger::default()
        };
        let Err(error) = decode_png(bytes, &mut failed) else {
            return Err(format!("{site} did not fail").into());
        };
        assert!(matches!(&error, SampleError::ResourceLimit { resource, .. } if *resource == site));
        assert_eq!(crate::approve::sample_code(&error), "RESOURCE_LIMIT");
        eprintln!("injected {site}: events={:?}", failed.events);
    }
    Ok(())
}

fn chunk(kind: [u8; 4], data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut result = Vec::new();
    result.extend_from_slice(&u32::try_from(data.len())?.to_be_bytes());
    result.extend_from_slice(&kind);
    result.extend_from_slice(data);
    let mut crc = crc32fast::Hasher::new();
    crc.update(&kind);
    crc.update(data);
    result.extend_from_slice(&crc.finalize().to_be_bytes());
    Ok(result)
}

#[test]
fn metadata_is_bounded_before_decoding_and_normal_text_preserves_pixels()
-> Result<(), Box<dyn Error>> {
    let source = include_bytes!("../../../tests/pngsuite/basn6a08.png");
    let mut normal = source[..33].to_vec();
    normal.extend_from_slice(&chunk(*b"tEXt", b"Author\0Someone")?);
    normal.extend_from_slice(&chunk(*b"iTXt", b"Title\0\0\0en\0Title\0Hello")?);
    normal.extend_from_slice(&chunk(
        *b"zTXt",
        b"Note\0\0\x78\x9c\xf3\x48\xcd\xc9\xc9\x07\x00\x05\x8c\x01\xf5",
    )?);
    normal.extend_from_slice(&source[33..]);
    let original = decode_png(source, &mut Ledger::default())?;
    let decoded = decode_png(&normal, &mut Ledger::default())?;
    assert_eq!(
        (decoded.width, decoded.height, decoded.rgba),
        (original.width, original.height, original.rgba)
    );

    let text = chunk(*b"tEXt", b"k\0")?;
    let mut malicious = source[..33].to_vec();
    for _ in 0..250_000 {
        malicious.extend_from_slice(&text);
    }
    malicious.extend_from_slice(&source[33..]);
    let mut ledger = Ledger::default();
    let Err(error) = decode_png(&malicious, &mut ledger) else {
        return Err("excessive metadata was accepted".into());
    };
    assert!(matches!(
        &error,
        SampleError::ResourceLimit {
            resource: "PNG metadata allocation bytes",
            ..
        }
    ));
    assert_eq!(crate::approve::sample_code(&error), "RESOURCE_LIMIT");
    assert_eq!(ledger.events.len(), 1);
    assert!(ledger.peak < 1024 * 1024);
    Ok(())
}

#[test]
fn metadata_scan_excludes_image_payload_and_trailing_data() -> Result<(), Box<dyn Error>> {
    let mut image = b"\x89PNG\r\n\x1a\n".to_vec();
    image.extend_from_slice(&chunk(*b"IDAT", &vec![0; 1024 * 1024])?);
    image.extend_from_slice(&chunk(*b"fdAT", &vec![0; 1024 * 1024])?);
    image.extend_from_slice(&chunk(*b"IEND", &[])?);
    let before = metadata_input_bytes(&image)?;
    assert_eq!(before, 8 + 3 * 12);
    image.extend_from_slice(&vec![0; 1024 * 1024]);
    assert_eq!(metadata_input_bytes(&image)?, before);
    Ok(())
}

#[test]
fn truncated_metadata_framing_is_counted_and_rejected() -> Result<(), Box<dyn Error>> {
    let source = include_bytes!("../../../tests/pngsuite/basn6a08.png");
    let text = chunk(*b"tEXt", b"key\0value")?;
    for end in 1..text.len() {
        let mut truncated = source[..33].to_vec();
        truncated.extend_from_slice(&text[..end]);
        assert_eq!(metadata_input_bytes(&truncated)?, truncated.len() as u64);
        let Err(error) = decode_png(&truncated, &mut Ledger::default()) else {
            return Err(format!("truncated chunk at byte {end} was accepted").into());
        };
        assert_eq!(
            crate::approve::sample_code(&error),
            "INVALID_IMAGE",
            "byte {end}"
        );
    }
    Ok(())
}

#[test]
fn header_errors_precede_excessive_metadata() -> Result<(), Box<dyn Error>> {
    let source = include_bytes!("../../../tests/pngsuite/basn6a08.png");
    let mut ihdr = source[16..29].to_vec();
    ihdr[..4].copy_from_slice(&(crate::sample::MAX_IMAGE_DIM + 1).to_be_bytes());
    let mut input = source[..8].to_vec();
    input.extend_from_slice(&chunk(*b"IHDR", &ihdr)?);
    let text = chunk(*b"tEXt", b"k\0")?;
    for _ in 0..250_000 {
        input.extend_from_slice(&text);
    }
    assert!(metadata_envelope(metadata_input_bytes(&input)?)? > MAX_METADATA_ENVELOPE);
    let Err(error) = decode_png(&input, &mut Ledger::default()) else {
        return Err("oversized dimensions were accepted".into());
    };
    assert!(matches!(error, SampleError::DimensionTooLarge { .. }));

    input[0] = 0;
    let Err(error) = decode_png(&input, &mut Ledger::default()) else {
        return Err("invalid header was accepted".into());
    };
    assert_eq!(crate::approve::sample_code(&error), "INVALID_IMAGE");
    Ok(())
}
