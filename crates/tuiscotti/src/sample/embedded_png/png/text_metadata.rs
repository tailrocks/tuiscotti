//! # Text chunks (tEXt/zTXt/iTXt) structs and functions
//!
//! The [PNG spec](https://www.w3.org/TR/2003/REC-PNG-20031110/#11textinfo) optionally allows for
//! embedded text chunks in the file. They may appear either before or after the image data
//! chunks. There are three kinds of text chunks.
//!  -   `tEXt`: This has a `keyword` and `text` field, and is ISO 8859-1 encoded.
//!  -   `zTXt`: This is semantically the same as `tEXt`, i.e. it has the same fields and
//!      encoding, but the `text` field is compressed before being written into the PNG file.
//!  -   `iTXt`: This chunk allows for its `text` field to be any valid UTF-8, and supports
//!      compression of the text field as well.
//!
//!  The `ISO 8859-1` encoding technically doesn't allow any control characters
//!  to be used, but in practice these values are encountered anyway. This can
//!  either be the extended `ISO-8859-1` encoding with control characters or the
//!  `Windows-1252` encoding. This crate assumes the `ISO-8859-1` encoding is
//!  used.
//!
//!  The standalone upstream snippets are retained as text because this embedded decoder is
//!  private and cannot be imported as an external `png` crate. Text parsing and writing remain
//!  covered by internal metadata, image corpus, and encoder round-trip tests.
//!
//!  ## Reading text chunks
//!
//!  As a PNG is decoded, any text chunk encountered is appended the
//!  [`Info`](`crate::sample::embedded_png::png::common::Info`) struct, in the `uncompressed_latin1_text`,
//!  `compressed_latin1_text`, and the `utf8_text` fields depending on whether the encountered
//!  chunk is `tEXt`, `zTXt`, or `iTXt`.
//!
//!  ```text
//!  use std::fs::File;
//!  use std::io::BufReader;
//!  use std::iter::FromIterator;
//!  use std::path::PathBuf;
//!
//!  // Opening a png file that has a zTXt chunk
//!  let decoder = png::Decoder::new(
//!      BufReader::new(File::open("tests/text_chunk_examples/ztxt_example.png").unwrap())
//!  );
//!  let mut reader = decoder.read_info().unwrap();
//!  // If the text chunk is before the image data frames, `reader.info()` already contains the text.
//!  for text_chunk in &reader.info()?.unwrap().compressed_latin1_text {
//!      println!("{:?}", text_chunk.keyword); // Prints the keyword
//!      println!("{:#?}", text_chunk); // Prints out the text chunk.
//!      // To get the uncompressed text, use the `get_text` method.
//!      println!("{}", text_chunk.get_text().unwrap());
//!  }
//!  ```text
//!
//!  ## Writing text chunks
//!
//!  There are two ways to write text chunks: the first is to add the appropriate text structs directly to the encoder header before the header is written to file.
//!  To add a text chunk at any point in the stream, use the `write_text_chunk` method.
//!
//!  ```text
//!  # use png::text_metadata::{ITXtChunk, ZTXtChunk};
//!  # use std::env;
//!  # use std::fs::File;
//!  # use std::io::BufWriter;
//!  # use std::iter::FromIterator;
//!  # use std::path::PathBuf;
//!  # let file = File::create(PathBuf::from_iter(["target", "text_chunk.png"])).unwrap();
//!  # let ref mut w = BufWriter::new(file);
//!  let mut encoder = png::Encoder::new(w, 2, 1); // Width is 2 pixels and height is 1.
//!  encoder.set_color(png::ColorType::Rgba);
//!  encoder.set_depth(png::BitDepth::Eight);
//!  // Adding text chunks to the header
//!  encoder
//!     .add_text_chunk(
//!         "Testing tEXt".to_string(),
//!         "This is a tEXt chunk that will appear before the IDAT chunks.".to_string(),
//!     )
//!     .unwrap();
//!  encoder
//!      .add_ztxt_chunk(
//!          "Testing zTXt".to_string(),
//!          "This is a zTXt chunk that is compressed in the png file.".to_string(),
//!      )
//!      .unwrap();
//!  encoder
//!      .add_itxt_chunk(
//!          "Testing iTXt".to_string(),
//!          "iTXt chunks support all of UTF8. Example: हिंदी.".to_string(),
//!      )
//!      .unwrap();
//!
//!  let mut writer = encoder.write_header().unwrap();
//!
//!  let data = [255, 0, 0, 255, 0, 0, 0, 255]; // An array containing a RGBA sequence. First pixel is red and second pixel is black.
//!  writer.write_image_data(&data).unwrap(); // Save
//!
//!  // We can add a tEXt/zTXt/iTXt at any point before the encoder is dropped from scope. These chunks will be at the end of the png file.
//!  let tail_ztxt_chunk = ZTXtChunk::new("Comment".to_string(), "A zTXt chunk after the image data.".to_string());
//!  writer.write_text_chunk(&tail_ztxt_chunk).unwrap();
//!
//!  // The fields of the text chunk are public, so they can be mutated before being written to the file.
//!  let mut tail_itxt_chunk = ITXtChunk::new("Author".to_string(), "सायंतन खान".to_string());
//!  tail_itxt_chunk.compressed = true;
//!  tail_itxt_chunk.language_tag = "hi".to_string();
//!  tail_itxt_chunk.translated_keyword = "लेखक".to_string();
//!  writer.write_text_chunk(&tail_itxt_chunk).unwrap();
//!  ```text

#![warn(missing_docs)]

use crate::sample::embedded_png::fdeflate::BoundedDecompressionError;
use crate::sample::embedded_png::png::decoder::Limits;
use crate::sample::embedded_png::png::{DecodingError, chunk};
#[cfg(test)]
use crate::sample::embedded_png::png::{EncodingError, encoder};
#[cfg(test)]
use flate2::Compression;
#[cfg(test)]
use flate2::write::ZlibEncoder;
use std::convert::TryFrom;
#[cfg(test)]
use std::io::Write;

/// Default decompression limit for compressed text chunks.
pub(crate) const DECOMPRESSION_LIMIT: usize = 2097152; // 2 MiB

fn unmetered_text_limits() -> Limits {
    Limits { bytes: usize::MAX }
}

/// Text encoding errors that is wrapped by the standard EncodingError type
#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum TextEncodingError {
    /// Unrepresentable characters in string
    Unrepresentable,
    /// Keyword longer than 79 bytes or empty
    InvalidKeywordSize,
    /// Error encountered while compressing text
    CompressionError,
}

/// Text decoding error that is wrapped by the standard DecodingError type
#[derive(Debug, Clone, Copy)]
pub(crate) enum TextDecodingError {
    /// Unrepresentable characters in string
    Unrepresentable,
    /// Keyword longer than 79 bytes or empty
    InvalidKeywordSize,
    /// Missing null separator
    MissingNullSeparator,
    /// Compressed text cannot be uncompressed
    InflationError,
    /// Needs more space to decompress
    OutOfDecompressionSpace,
    /// Using an unspecified value for the compression method
    InvalidCompressionMethod,
    /// Using a byte that is not 0 or 255 as compression flag in iTXt chunk
    InvalidCompressionFlag,
    /// Missing the compression flag
    MissingCompressionFlag,
}

/// A generalized text chunk trait
#[cfg(test)]
pub(crate) trait EncodableTextChunk {
    /// Encode text chunk as `Vec<u8>` to a `Write`
    fn encode<W: Write>(&self, w: &mut W) -> Result<(), EncodingError>;
}

/// Struct representing a tEXt chunk
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TEXtChunk {
    /// Keyword field of the tEXt chunk. Needs to be between 1-79 bytes when encoded as Latin-1.
    pub(crate) keyword: String,
    /// Text field of tEXt chunk. Can be at most 2GB.
    pub(crate) text: String,
}

fn decode_iso_8859_1(
    text: &[u8],
    site: &'static str,
    limits: &mut Limits,
) -> Result<String, DecodingError> {
    let extra = text.iter().filter(|&&byte| byte >= 0x80).count();
    let requested = text
        .len()
        .checked_add(extra)
        .ok_or(DecodingError::LimitsExceeded)?;
    limits.reserve_bytes(requested)?;
    let mut decoded = String::new();
    decoded
        .try_reserve_exact(requested)
        .map_err(|source| DecodingError::allocation(site, requested, source))?;
    for &byte in text {
        decoded.push(byte as char);
    }
    Ok(decoded)
}

fn copy_text_bytes(
    text: &[u8],
    site: &'static str,
    limits: &mut Limits,
) -> Result<Vec<u8>, DecodingError> {
    limits.reserve_bytes(text.len())?;
    let mut copy = Vec::new();
    copy.try_reserve_exact(text.len())
        .map_err(|source| DecodingError::allocation(site, text.len(), source))?;
    copy.extend_from_slice(text);
    Ok(copy)
}

fn copy_text_string(
    text: &str,
    site: &'static str,
    limits: &mut Limits,
) -> Result<String, DecodingError> {
    limits.reserve_bytes(text.len())?;
    let mut copy = String::new();
    copy.try_reserve_exact(text.len())
        .map_err(|source| DecodingError::allocation(site, text.len(), source))?;
    copy.push_str(text);
    Ok(copy)
}

#[cfg(test)]
pub(crate) fn encode_iso_8859_1(text: &str) -> Result<Vec<u8>, TextEncodingError> {
    encode_iso_8859_1_iter(text).collect()
}

#[cfg(test)]
fn encode_iso_8859_1_into(buf: &mut Vec<u8>, text: &str) -> Result<(), TextEncodingError> {
    for b in encode_iso_8859_1_iter(text) {
        buf.push(b?);
    }
    Ok(())
}

#[cfg(test)]
fn encode_iso_8859_1_iter(text: &str) -> impl Iterator<Item = Result<u8, TextEncodingError>> + '_ {
    text.chars()
        .map(|c| u8::try_from(c as u32).map_err(|_| TextEncodingError::Unrepresentable))
}

fn decode_ascii(text: &[u8]) -> Result<&str, TextDecodingError> {
    if text.is_ascii() {
        // `from_utf8` cannot panic because we're already checked that `text` is ASCII-7.
        // And this is the only safe way to get ASCII-7 string from `&[u8]`.
        std::str::from_utf8(text).map_err(|_| TextDecodingError::Unrepresentable)
    } else {
        Err(TextDecodingError::Unrepresentable)
    }
}

mod compressed;
mod international;
mod text;

pub(crate) use compressed::ZTXtChunk;
pub(crate) use international::ITXtChunk;
