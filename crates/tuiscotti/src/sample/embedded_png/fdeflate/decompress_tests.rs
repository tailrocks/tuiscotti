use super::*;
use std::{error::Error, fmt::Debug, io};

fn checked<T, E: Debug>(result: Result<T, E>) -> io::Result<T> {
    result.map_err(|error| io::Error::other(format!("{error:?}")))
}

use crate::sample::embedded_png::fdeflate::tables::{LENGTH_TO_LEN_EXTRA, LENGTH_TO_SYMBOL};

use super::*;
use rand::Rng;

fn roundtrip(data: &[u8]) -> Result<(), Box<dyn Error>> {
    let compressed = crate::sample::embedded_png::fdeflate::compress_to_vec(data)?;
    let decompressed = checked(decompress_to_vec(&compressed))?;
    assert_eq!(&decompressed, data);
    Ok(())
}

fn roundtrip_miniz_oxide(data: &[u8]) -> Result<(), Box<dyn Error>> {
    let compressed = miniz_oxide::deflate::compress_to_vec_zlib(data, 3);
    let decompressed = checked(decompress_to_vec(&compressed))?;
    assert_eq!(decompressed.len(), data.len());
    for (i, (a, b)) in decompressed.chunks(1).zip(data.chunks(1)).enumerate() {
        assert_eq!(a, b, "chunk {}..{}", i, i + 1);
    }
    assert_eq!(&decompressed, data);
    Ok(())
}

fn compare_decompression(data: &[u8]) -> Result<(), String> {
    let decompressed = decompress_to_vec(data).map_err(|error| format!("{error:?}"))?;
    let decompressed2 =
        miniz_oxide::inflate::decompress_to_vec_zlib(data).map_err(|error| format!("{error:?}"))?;
    for i in 0..decompressed.len().min(decompressed2.len()) {
        assert_eq!(decompressed[i], decompressed2[i], "mismatch at index {i}");
    }
    assert_eq!(decompressed.len(), decompressed2.len(), "length mismatch");
    assert_eq!(decompressed, decompressed2);
    Ok(())
}

#[test]
fn tables() {
    for (i, &bits) in LEN_SYM_TO_LEN_EXTRA.iter().enumerate() {
        let len_base = LEN_SYM_TO_LEN_BASE[i];
        for j in 0..(1 << bits) {
            if i == 27 && j == 31 {
                continue;
            }
            assert_eq!(LENGTH_TO_LEN_EXTRA[len_base + j - 3], bits, "{} {}", i, j);
            assert_eq!(
                LENGTH_TO_SYMBOL[len_base + j - 3],
                i as u16 + 257,
                "{} {}",
                i,
                j
            );
        }
    }
}

#[test]
fn fixed_tables() -> Result<(), DecompressionError> {
    let mut compression = CompressedBlock {
        litlen_table: [0; 4096],
        dist_table: [0; 512],
        secondary_table: Vec::new(),
        dist_secondary_table: Vec::new(),
        eof_code: 0,
        eof_mask: 0,
        eof_bits: 0,
    };
    Decompressor::build_tables(288, &FIXED_CODE_LENGTHS, &mut compression, &mut None)?;

    assert_eq!(compression.litlen_table[..512], FIXED_LITLEN_TABLE);
    assert_eq!(compression.dist_table[..32], FIXED_DIST_TABLE);
    Ok(())
}

#[test]
fn bounded_output_reservations_are_debited_before_each_growth() -> Result<(), Box<dyn Error>> {
    let input = vec![0x5a; 40_000];
    let compressed = crate::sample::embedded_png::fdeflate::compress_to_vec(&input)?;
    let mut requests = Vec::new();

    let output = checked(decompress_to_vec_bounded(
        &compressed,
        input.len(),
        |requested| {
            requests.push(requested);
            true
        },
    ))?;

    assert_eq!(output, input);
    assert_eq!(requests, [1024, 32 * 1024, 40_000 - 1024 - 32 * 1024]);
    Ok(())
}

#[test]
fn bounded_output_stops_when_the_budget_callback_refuses_growth() -> Result<(), Box<dyn Error>> {
    let input = vec![0x5a; 40_000];
    let compressed = crate::sample::embedded_png::fdeflate::compress_to_vec(&input)?;
    let mut requests = Vec::new();

    let result = decompress_to_vec_bounded(&compressed, input.len(), |requested| {
        requests.push(requested);
        requests.len() == 1
    });

    assert!(matches!(
        result,
        Err(BoundedDecompressionError::LimitsExceeded)
    ));
    assert_eq!(requests, [1024, 32 * 1024]);
    Ok(())
}

#[test]
fn it_works() -> Result<(), String> {
    let input = b"Hello world!";
    roundtrip(input).map_err(|error| format!("{error:?}"))?;
    let compressed = crate::sample::embedded_png::fdeflate::compress_to_vec(input)
        .map_err(|error| format!("{error:?}"))?;
    compare_decompression(&compressed)
}

#[test]
fn constant() -> Result<(), Box<dyn Error>> {
    roundtrip_miniz_oxide(&[0; 50])?;
    roundtrip_miniz_oxide(&vec![5; 2048])?;
    roundtrip_miniz_oxide(&vec![128; 2048])?;
    roundtrip_miniz_oxide(&vec![254; 2048])
}

#[test]
fn random() -> Result<(), Box<dyn Error>> {
    let mut rng = rand::rng();
    let mut data = vec![0; 50000];
    for _ in 0..10 {
        for byte in &mut data {
            *byte = rng.random::<u8>() % 5;
        }
        println!("Random data: {:?}", data);
        roundtrip_miniz_oxide(&data)?;
    }
    Ok(())
}

#[test]
fn ignore_adler32() -> Result<(), Box<dyn Error>> {
    let mut compressed = crate::sample::embedded_png::fdeflate::compress_to_vec(b"Hello world!")?;
    let last_byte = compressed.len() - 1;
    compressed[last_byte] = compressed[last_byte].wrapping_add(1);

    assert!(matches!(
        decompress_to_vec(&compressed),
        Err(DecompressionError::WrongChecksum)
    ));

    let mut decompressor = Decompressor::new();
    decompressor.ignore_adler32();
    let mut decompressed = vec![0; 1024];
    let decompressed_len = decompressor
        .read(&compressed, &mut decompressed, 0, true)
        .map_err(|error| io::Error::other(format!("{error:?}")))?
        .1;
    assert_eq!(&decompressed[..decompressed_len], b"Hello world!");
    Ok(())
}

#[test]
fn checksum_after_eof() -> Result<(), Box<dyn Error>> {
    let input = b"Hello world!";
    let compressed = crate::sample::embedded_png::fdeflate::compress_to_vec(input)?;

    let mut decompressor = Decompressor::new();
    let mut decompressed = vec![0; 1024];
    let (input_consumed, output_written) = decompressor
        .read(
            &compressed[..compressed.len() - 1],
            &mut decompressed,
            0,
            false,
        )
        .map_err(|error| io::Error::other(format!("{error:?}")))?;
    assert_eq!(output_written, input.len());
    assert_eq!(input_consumed, compressed.len() - 1);

    let (input_consumed, output_written) = decompressor
        .read(
            &compressed[input_consumed..],
            &mut decompressed[..output_written],
            output_written,
            true,
        )
        .map_err(|error| io::Error::other(format!("{error:?}")))?;
    assert!(decompressor.is_done());
    assert_eq!(input_consumed, 1);
    assert_eq!(output_written, 0);

    assert_eq!(&decompressed[..input.len()], input);
    Ok(())
}

#[test]
fn zero_length() -> Result<(), Box<dyn Error>> {
    let mut compressed = crate::sample::embedded_png::fdeflate::compress_to_vec(b"")?;

    // Splice in zero-length non-compressed blocks.
    for _ in 0..10 {
        println!("compressed len: {}", compressed.len());
        compressed.splice(2..2, [0u8, 0, 0, 0xff, 0xff].into_iter());
    }

    // Ensure that the full input is decompressed, regardless of whether
    // `end_of_input` is set.
    for end_of_input in [true, false] {
        let mut decompressor = Decompressor::new();
        let (input_consumed, output_written) = decompressor
            .read(&compressed, &mut [], 0, end_of_input)
            .map_err(|error| io::Error::other(format!("{error:?}")))?;

        assert!(decompressor.is_done());
        assert_eq!(input_consumed, compressed.len());
        assert_eq!(output_written, 0);
    }
    Ok(())
}

#[path = "decompress/tests/test_utils.rs"]
mod test_utils;
use tables::FIXED_CODE_LENGTHS;
use test_utils::{TestDecompressionError, decompress_by_chunks};

#[test]
fn inline_decoder_layout_bound() {
    assert!(std::mem::size_of::<Decompressor>() <= 20 * 1024);
}

fn verify_no_sensitivity_to_input_chunking(
    input: &[u8],
) -> Result<Vec<u8>, TestDecompressionError> {
    let r_whole = decompress_by_chunks(input, vec![input.len()], false);
    let r_bytewise = decompress_by_chunks(input, std::iter::repeat(1), false);
    assert_eq!(r_whole, r_bytewise);
    r_whole // Returning an arbitrary result, since this is equal to `r_bytewise`.
}

/// This is a regression test found by the `buf_independent` fuzzer from the `png` crate.  When
/// this test case was found, the results were unexpectedly different when 1) decompressing the
/// whole input (successful result) vs 2) decompressing byte-by-byte
/// (`Err(InvalidDistanceCode)`).
#[test]
fn test_input_chunking_sensitivity_when_handling_distance_codes()
-> Result<(), TestDecompressionError> {
    let result = verify_no_sensitivity_to_input_chunking(include_bytes!(
        "../tests/input-chunking-sensitivity-example1.zz"
    ))?;
    assert_eq!(result.len(), 281);
    assert_eq!(simd_adler32::adler32(&result.as_slice()), 751299);
    Ok(())
}

/// This is a regression test found by the `inflate_bytewise3` fuzzer from the `fdeflate`
/// crate.  When this test case was found, the results were unexpectedly different when 1)
/// decompressing the whole input (`Err(DistanceTooFarBack)`) vs 2) decompressing byte-by-byte
/// (successful result)`).
#[test]
fn test_input_chunking_sensitivity_when_no_end_of_block_symbol_example1() {
    assert_eq!(
        verify_no_sensitivity_to_input_chunking(include_bytes!(
            "../tests/input-chunking-sensitivity-example2.zz"
        )),
        Err(TestDecompressionError::ProdError(
            DecompressionError::BadLiteralLengthHuffmanTree
        ))
    );
}

/// This is a regression test found by the `inflate_bytewise3` fuzzer from the `fdeflate`
/// crate.  When this test case was found, the results were unexpectedly different when 1)
/// decompressing the whole input (`Err(InvalidDistanceCode)`) vs 2) decompressing byte-by-byte
/// (successful result)`).
#[test]
fn test_input_chunking_sensitivity_when_no_end_of_block_symbol_example2() {
    assert_eq!(
        verify_no_sensitivity_to_input_chunking(include_bytes!(
            "../tests/input-chunking-sensitivity-example3.zz"
        )),
        Err(TestDecompressionError::ProdError(
            DecompressionError::BadLiteralLengthHuffmanTree
        ))
    );
}
