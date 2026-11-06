use std::{fs::File, io::BufReader};

use crate::sample::embedded_png::png::{DecodeOptions, Decoder, DecodingError};

#[test]
fn issue_430() -> std::io::Result<()> {
    let file = File::open("tests/bugfixes/issue#430.png")?;

    let mut decode_options = DecodeOptions::default();
    decode_options.set_skip_ancillary_crc_failures(false);

    let decoder = Decoder::new_with_options(BufReader::new(file), decode_options).read_info();

    assert!(
        matches!(decoder, Err(DecodingError::Format(_))),
        "Decoding of iCCP chunk with invalid CRC should have failed with 'skip_ancillary_crc' disabled."
    );
    Ok(())
}
