use super::embedded_png::{fdeflate, png};
use std::io::Cursor;

use super::SampleError;
use super::budget::{Charge, DecodeBudget};
use super::validation::check_image_dims;

const _: () = assert!(std::mem::size_of::<png::text_metadata::TEXtChunk>() <= 128);
const _: () = assert!(std::mem::size_of::<png::text_metadata::ZTXtChunk>() <= 128);
const _: () = assert!(std::mem::size_of::<png::text_metadata::ITXtChunk>() <= 128);
const _: () = assert!(std::mem::size_of::<fdeflate::Decompressor>() <= 20 * 1024);
const _: () = assert!(std::mem::size_of::<png::DecodingError>() <= 4096);

pub(crate) trait PngBudget {
    type Token;
    fn charge(&mut self, site: &'static str, bytes: u64) -> Result<Self::Token, SampleError>;
    fn release(&mut self, token: Self::Token) -> Result<(), SampleError>;
}

impl PngBudget for DecodeBudget {
    type Token = Charge;
    fn charge(&mut self, site: &'static str, bytes: u64) -> Result<Charge, SampleError> {
        DecodeBudget::charge(self, site, bytes).map_err(SampleError::from)
    }
    fn release(&mut self, token: Charge) -> Result<(), SampleError> {
        DecodeBudget::release(self, token).map_err(SampleError::from)
    }
}

pub(crate) struct PngPixels<T> {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgba: Vec<u8>,
    pub(crate) charge: T,
}

fn arithmetic_error(resource: &'static str) -> SampleError {
    SampleError::ResourceLimit {
        resource,
        claimed: None,
        limit: u64::MAX,
        source: None,
    }
}

fn bounded(value: u128, resource: &'static str) -> Result<u64, SampleError> {
    u64::try_from(value).map_err(|_| arithmetic_error(resource))
}

fn metadata_envelope(p: u64) -> Result<u64, SampleError> {
    let p = u128::from(p);
    let n = p / 12;
    let s = std::mem::size_of::<png::text_metadata::TEXtChunk>()
        .max(std::mem::size_of::<png::text_metadata::ZTXtChunk>())
        .max(std::mem::size_of::<png::text_metadata::ITXtChunk>()) as u128;
    let value = 3 * p.max(128) + 9 * p + 16 * n + 8 + (3 * n + 12) * s;
    bounded(value, "PNG metadata allocation bytes")
}

fn header_envelope(p: u64) -> Result<u64, SampleError> {
    let deflate = 6_u128 * (2288 + 1920)
        + 2 * (4096 + 512) * 4
        + std::mem::size_of::<fdeflate::Decompressor>() as u128;
    bounded(
        u128::from(metadata_envelope(p)?) + deflate,
        "PNG header allocation bytes",
    )
}

fn row_envelope(width: u32, height: u32) -> Result<u64, SampleError> {
    let w = u128::from(width);
    let h = u128::from(height);
    let r = 8 * w + 1;
    let t = 7 * h * r;
    let rows = 3 * (t + 8192).max(131_072) + 6 * r.max(8) + 2 * (4 * w).max(8);
    bounded(rows + 1024, "PNG row allocation bytes")
}

fn counter_limit(p: u64, width: u32, height: u32) -> Result<usize, SampleError> {
    let value = header_envelope(p)?
        .checked_add(row_envelope(width, height)?)
        .ok_or_else(|| arithmetic_error("PNG decoder allocation bytes"))?;
    Ok(addressable_counter(value, usize::MAX))
}

fn addressable_counter(value: u64, addressable: usize) -> usize {
    usize::try_from(value).map_or(addressable, |value| value.min(addressable))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::io;

    #[test]
    fn allocation_errors_keep_source_and_resource_classification() -> Result<(), Box<dyn Error>> {
        use std::error::Error as _;
        fn reserve_failure() -> io::Result<std::collections::TryReserveError> {
            Vec::<u8>::new()
                .try_reserve_exact(usize::MAX)
                .err()
                .ok_or_else(|| {
                    io::Error::other("reserving usize::MAX bytes unexpectedly succeeded")
                })
        }
        let png = png::DecodingError::Allocation {
            site: "test PNG allocation",
            requested: usize::MAX,
            source: reserve_failure()?,
        };
        let sample = png_error(png, 128);
        assert!(matches!(
            sample,
            SampleError::Allocation {
                site: "test PNG allocation",
                ..
            }
        ));
        assert!(sample.source().is_some());
        assert_eq!(crate::approve::sample_code(&sample), "RESOURCE_LIMIT");
        let encoded = SampleError::EncodeAllocation {
            source: reserve_failure()?,
        };
        assert!(encoded.source().is_some());
        assert_eq!(crate::approve::sample_code(&encoded), "RESOURCE_LIMIT");
        let limit = png_error(png::DecodingError::LimitsExceeded, 128);
        assert!(limit.source().is_some());
        assert_eq!(crate::approve::sample_code(&limit), "RESOURCE_LIMIT");
        Ok(())
    }

    #[test]
    fn png_counter_covers_the_header_and_row_request_envelopes() -> Result<(), Box<dyn Error>> {
        for p in [0, 1, 64 * 1024 * 1024] {
            for (width, height) in [
                (1, 1),
                (crate::sample::MAX_IMAGE_DIM, crate::sample::MAX_IMAGE_DIM),
            ] {
                let expected = header_envelope(p)?
                    .checked_add(row_envelope(width, height)?)
                    .ok_or("combined PNG allocation envelope overflowed")?;
                assert_eq!(
                    counter_limit(p, width, height)?,
                    usize::try_from(expected).unwrap_or(usize::MAX),
                    "P={p}, dimensions={width}x{height}"
                );
            }
        }
        assert!(counter_limit(u64::MAX, u32::MAX, u32::MAX).is_err());
        let wide = header_envelope(1)?
            .checked_add(row_envelope(8_200, 4_100)?)
            .ok_or("wide PNG allocation envelope overflowed")?;
        assert!(wide > u64::from(u32::MAX));
        super::super::validation::check_image_dims(8_200, 4_100)?;
        let rgba_bytes = u64::from(8_200_u32)
            .checked_mul(u64::from(4_100_u32))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or("RGBA byte count overflowed")?;
        assert!(rgba_bytes < super::super::MAX_RGBA_BYTES);
        assert_eq!(
            counter_limit(1, 8_200, 4_100)?,
            usize::try_from(wide).unwrap_or(usize::MAX),
            "the proof envelope saturates at the target address space while valid RGBA stays admitted"
        );
        assert_eq!(
            addressable_counter(wide, u32::MAX as usize),
            u32::MAX as usize
        );
        assert_eq!(
            addressable_counter(wide, usize::MAX),
            usize::try_from(wide).unwrap_or(usize::MAX)
        );
        Ok(())
    }
}
fn png_error(source: png::DecodingError, counter: usize) -> SampleError {
    match source {
        png::DecodingError::Allocation {
            site,
            requested,
            source,
        } => SampleError::Allocation {
            site,
            requested: u64::try_from(requested).unwrap_or(u64::MAX),
            source,
        },
        png::DecodingError::LimitsExceeded => SampleError::PngResourceLimit {
            limit: u64::try_from(counter).unwrap_or(u64::MAX),
            source: super::PngDecoderError::new(source),
        },
        source => SampleError::PngCodec {
            source: super::PngDecoderError::new(source),
        },
    }
}

pub(crate) fn decode_png<B: PngBudget>(
    bytes: &[u8],
    budget: &mut B,
) -> Result<PngPixels<B::Token>, SampleError> {
    const INITIAL_HEADER_COUNTER: usize = 128;
    let p = u64::try_from(bytes.len()).map_err(|_| arithmetic_error("PNG input bytes"))?;
    let header_charge = budget.charge("PNG header and metadata", header_envelope(p)?)?;
    // `read_header_info` only grows the raw chunk vector for the 13-byte IHDR, whose first
    // reservation is exactly the embedded decoder's 128-byte chunk-buffer request.
    let initial_counter = INITIAL_HEADER_COUNTER;
    let mut decoder = png::Decoder::new_with_limits(
        Cursor::new(bytes),
        png::Limits {
            bytes: initial_counter,
        },
    );
    decoder.set_ignore_iccp_chunk(true);
    decoder.set_transformations(png::Transformations::EXPAND);
    let info = decoder
        .read_header_info()
        .map_err(|source| png_error(source, initial_counter))?;
    let (width, height) = (info.width, info.height);
    check_image_dims(width, height)?;
    let counter = counter_limit(p, width, height)?;
    decoder.set_limits(png::Limits { bytes: counter });
    let row_charge = budget.charge("PNG rows and transform", row_envelope(width, height)?)?;
    let mut reader = decoder
        .read_info()
        .map_err(|source| png_error(source, counter))?;
    let (color, depth) = reader
        .output_color_type()
        .map_err(|source| png_error(source, counter))?;
    if (color, depth) != (png::ColorType::Rgba, png::BitDepth::Eight) {
        return Err(SampleError::PngColor {
            color: color.into(),
            depth: depth.into(),
        });
    }
    let rgba_bytes = u64::from(width) * u64::from(height) * 4;
    let len = usize::try_from(rgba_bytes).map_err(|_| SampleError::ResourceLimit {
        resource: "addressable RGBA bytes",
        claimed: Some(rgba_bytes),
        limit: u64::try_from(usize::MAX).unwrap_or(u64::MAX),
        source: None,
    })?;
    let charge = budget.charge("PNG RGBA output", rgba_bytes)?;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(len)
        .map_err(|source| SampleError::Allocation {
            site: "PNG RGBA output",
            requested: rgba_bytes,
            source,
        })?;
    rgba.resize(len, 0);
    reader
        .next_frame(&mut rgba)
        .map_err(|source| png_error(source, counter))?;
    drop(reader);
    budget.release(row_charge)?;
    budget.release(header_charge)?;
    Ok(PngPixels {
        width,
        height,
        rgba,
        charge,
    })
}

#[derive(Debug, Default)]
pub(crate) struct DynamicBudget {
    total: u64,
    peak: u64,
    limit: u64,
}

#[derive(Debug)]
pub(crate) struct DynamicCharge(u64);

impl PngBudget for DynamicBudget {
    type Token = DynamicCharge;

    fn charge(&mut self, _site: &'static str, bytes: u64) -> Result<Self::Token, SampleError> {
        let total = self
            .total
            .checked_add(bytes)
            .ok_or_else(|| arithmetic_error("PNG operation allocation bytes"))?;
        if total > self.limit {
            return Err(SampleError::ResourceLimit {
                resource: "PNG operation allocation bytes",
                claimed: Some(total),
                limit: self.limit,
                source: None,
            });
        }
        self.total = total;
        self.peak = self.peak.max(total);
        Ok(DynamicCharge(bytes))
    }

    fn release(&mut self, token: Self::Token) -> Result<(), SampleError> {
        self.total = self
            .total
            .checked_sub(token.0)
            .ok_or(SampleError::DecodeInvariant {
                detail: "dynamic PNG budget charge underflow",
            })?;
        Ok(())
    }
}

impl DynamicBudget {
    pub(crate) fn bounded() -> Result<Self, SampleError> {
        let budget = DecodeBudget::new().map_err(SampleError::from)?;
        Ok(Self {
            limit: budget.limit(),
            ..Self::default()
        })
    }

    #[cfg(test)]
    pub(crate) fn peak(&self) -> u64 {
        self.peak
    }
}

pub(crate) fn decode_base64<B: PngBudget>(
    input: &str,
    budget: &mut B,
) -> Result<(Vec<u8>, B::Token), SampleError> {
    use base64::Engine as _;
    let length = input
        .len()
        .checked_add(3)
        .and_then(|n| n.checked_div(4))
        .and_then(|n| n.checked_mul(3))
        .ok_or_else(|| arithmetic_error("base64 decoded allocation bytes"))?;
    let requested =
        u64::try_from(length).map_err(|_| arithmetic_error("base64 decoded allocation bytes"))?;
    let charge = budget.charge("base64 PNG input", requested)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|source| SampleError::Allocation {
            site: "base64 PNG input",
            requested,
            source,
        })?;
    bytes.resize(length, 0);
    let written = base64::engine::general_purpose::STANDARD
        .decode_slice(input, &mut bytes)
        .map_err(|source| match source {
            base64::DecodeSliceError::DecodeError(source) => SampleError::Base64Invalid { source },
            base64::DecodeSliceError::OutputSliceTooSmall => SampleError::DecodeInvariant {
                detail: "base64 decoded size estimate too small",
            },
        })?;
    bytes.truncate(written);
    Ok((bytes, charge))
}

pub(crate) fn copy_text(
    value: &str,
    budget: &mut DynamicBudget,
    site: &'static str,
) -> Result<String, SampleError> {
    let requested = u64::try_from(value.len()).map_err(|_| arithmetic_error(site))?;
    let _charge = budget.charge(site, requested)?;
    let mut result = String::new();
    result
        .try_reserve_exact(value.len())
        .map_err(|source| SampleError::Allocation {
            site,
            requested,
            source,
        })?;
    result.push_str(value);
    Ok(result)
}

#[cfg(test)]
#[path = "png_decode/boundary_probe.rs"]
mod boundary_probe;
