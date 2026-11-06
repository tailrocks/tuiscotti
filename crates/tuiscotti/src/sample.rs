#![doc = include_str!("../docs/sample.md")]
#[doc = "Sample schema identifier. Only this value parses."]
pub const SAMPLE_SCHEMA: &str = "tuiscotti.sample.v1";

#[doc = "Schema version of the downstream render contract in samples."]
pub const CONTRACT_VERSION: u32 = 1;

#[doc = "Encoded sample file limit: 64 MiB, enforced with bounded reads."]
pub const MAX_SAMPLE_FILE_BYTES: u64 = 64 * 1024 * 1024;
#[doc = "Cells in one sample."]
pub const MAX_SAMPLE_CELLS: usize = 1_000_000;
#[doc = "UTF-8 bytes in one cell string."]
pub const MAX_CELL_TEXT_BYTES: usize = 4_096;
#[doc = "Decoded RGBA byte limit: 256 MiB, validated before allocation."]
pub const MAX_RGBA_BYTES: u64 = 256 * 1024 * 1024;
#[doc = "One image dimension limit: 32,768 pixels."]
pub const MAX_IMAGE_DIM: u32 = 32_768;
#[doc = "Test ID length limit: 240 ASCII bytes."]
pub const MAX_TEST_ID_BYTES: usize = 240;
#[doc = "One test ID segment limit: 64 ASCII bytes."]
pub const MAX_TEST_ID_SEGMENT_BYTES: usize = 64;

mod budget;
mod container;
mod decode_error;
mod digest;
pub(crate) mod embedded_png;
mod error;
mod hash;
mod image;
mod io;
pub(crate) mod png_decode;
mod record;
mod types;
mod validation;
mod wire;

pub use decode_error::{BinaryFormatError, WirePath, ZstdError};
pub use error::{PngBitDepth, PngColorType, PngDecoderError, SampleError};
pub(crate) use image::decode_png_bounded;
pub(crate) use image::decode_png_with_budget;
pub(crate) use io::{read_bounded, read_bounded_optional};
pub use record::Sample;
pub use types::{
    ContractFont, CursorPhase, Gate, ImageFormat, Observation, RenderContract, SampleImage,
    SampleSchema,
};
pub(crate) use validation::validate_test_id_grammar;
