use super::budget::{Charge, DecodeBudget, Phase};
use super::decode_error::{BinaryFormatError, DecodeError, ZstdError};

const HEADER_LEN: usize = 32;
const MAX_INPUT: u64 = 64 * 1024 * 1024;
const MAX_WINDOW: u64 = 32 * 1024 * 1024;
const CONTEXT_LIMIT: u64 = 1024 * 1024 * 1024;
const SCRATCH: usize = 65_536;
const CODEC_ENVELOPE: u64 = CONTEXT_LIMIT + 16 + 33_947_712 + 16 + 65_536;

#[derive(Debug)]
pub(super) struct DecodedMessage {
    pub(super) bytes: Vec<u8>,
    pub(super) charge: Charge,
}

#[derive(Debug)]
struct Header<'a> {
    payload: &'a [u8],
    expanded: u64,
}

fn header_error(offset: usize, detail: &'static str) -> DecodeError {
    DecodeError::Format(BinaryFormatError::Header { offset, detail })
}
fn frame_error(offset: usize, detail: &'static str) -> DecodeError {
    DecodeError::Format(BinaryFormatError::Frame {
        offset: HEADER_LEN + offset,
        detail,
    })
}
fn codec_error(code: usize) -> DecodeError {
    DecodeError::Format(BinaryFormatError::Codec {
        source: Box::new(ZstdError { code }),
    })
}

fn parse_header(input: &[u8]) -> Result<Header<'_>, DecodeError> {
    let len = u64::try_from(input.len())
        .map_err(|_| DecodeError::resource("encoded sample bytes", None, MAX_INPUT))?;
    if len > MAX_INPUT {
        return Err(DecodeError::resource(
            "encoded sample bytes",
            Some(len),
            MAX_INPUT,
        ));
    }
    if input.len() < HEADER_LEN {
        return Err(header_error(input.len(), "truncated container header"));
    }
    if &input[..8] != b"TUISCPB\0" {
        return Err(header_error(0, "invalid container magic"));
    }
    let version = u16::from_le_bytes([input[8], input[9]]);
    if version != 1 {
        return Err(DecodeError::UnsupportedContainerVersion(version));
    }
    let codec = u16::from_le_bytes([input[10], input[11]]);
    if codec != 1 {
        return Err(DecodeError::UnsupportedCodec(codec));
    }
    let version = u32::from_le_bytes([input[12], input[13], input[14], input[15]]);
    if version != 1 {
        return Err(DecodeError::UnsupportedMessageVersion(version));
    }
    let expanded = u64::from_le_bytes(
        input[16..24]
            .try_into()
            .map_err(|_| header_error(16, "truncated expanded length"))?,
    );
    let compressed = u64::from_le_bytes(
        input[24..32]
            .try_into()
            .map_err(|_| header_error(24, "truncated compressed length"))?,
    );
    if expanded > MAX_INPUT {
        return Err(DecodeError::resource(
            "decompressed message bytes",
            Some(expanded),
            MAX_INPUT,
        ));
    }
    let max_payload = MAX_INPUT - HEADER_LEN as u64;
    if compressed > max_payload {
        return Err(DecodeError::resource(
            "compressed payload bytes",
            Some(compressed),
            max_payload,
        ));
    }
    let expected = compressed
        .checked_add(HEADER_LEN as u64)
        .ok_or_else(|| DecodeError::resource("encoded sample bytes", None, MAX_INPUT))?;
    if len != expected {
        return Err(header_error(24, "compressed length does not match input"));
    }
    Ok(Header {
        payload: &input[HEADER_LEN..],
        expanded,
    })
}

pub(super) fn frame_header(payload: &[u8], expanded: u64) -> Result<(), DecodeError> {
    if payload.len() < 5 {
        return Err(frame_error(payload.len(), "truncated frame header"));
    }
    if payload[..4] != [0x28, 0xb5, 0x2f, 0xfd] {
        return Err(frame_error(0, "invalid Zstandard frame magic"));
    }
    let descriptor = payload[4];
    if descriptor & 8 != 0 {
        return Err(frame_error(4, "reserved frame descriptor bit"));
    }
    if descriptor & 4 == 0 {
        return Err(frame_error(4, "frame checksum is required"));
    }
    let single = descriptor & 0x20 != 0;
    let dict_width = [0_usize, 1, 2, 4][usize::from(descriptor & 3)];
    let content_width = match descriptor >> 6 {
        0 if single => 1,
        0 => 0,
        1 => 2,
        2 => 4,
        _ => 8,
    };
    let mut pos = 5;
    let window = if single {
        None
    } else {
        let Some(&wd) = payload.get(pos) else {
            return Err(frame_error(pos, "truncated window descriptor"));
        };
        pos += 1;
        let base = 1_u64
            .checked_shl(10 + u32::from(wd >> 3))
            .ok_or_else(|| DecodeError::resource("Zstandard window bytes", None, MAX_WINDOW))?;
        Some(
            base.checked_add((base / 8) * u64::from(wd & 7))
                .ok_or_else(|| DecodeError::resource("Zstandard window bytes", None, MAX_WINDOW))?,
        )
    };
    let dictionary = read_le(payload, &mut pos, dict_width)?;
    if dictionary != 0 {
        return Err(frame_error(pos - dict_width, "dictionary is not supported"));
    }
    let content = if content_width == 0 {
        None
    } else {
        let mut value = read_le(payload, &mut pos, content_width)?;
        if content_width == 2 {
            value += 256;
        }
        Some(value)
    };
    let actual_window = if single {
        content.ok_or_else(|| frame_error(pos, "missing frame content size"))?
    } else {
        window.unwrap_or(0)
    };
    validate_frame_content(
        payload,
        pos,
        content_width,
        content,
        actual_window,
        expanded,
    )
}

fn validate_frame_content(
    payload: &[u8],
    pos: usize,
    content_width: usize,
    content: Option<u64>,
    actual_window: u64,
    expanded: u64,
) -> Result<(), DecodeError> {
    if actual_window > MAX_WINDOW {
        return Err(DecodeError::resource(
            "Zstandard window bytes",
            Some(actual_window),
            MAX_WINDOW,
        ));
    }
    if let Some(content) = content
        && content != expanded
    {
        return Err(frame_error(
            pos - content_width,
            "frame content size disagrees with container",
        ));
    }
    let library_content = zstd_safe::get_frame_content_size(payload)
        .map_err(|_| frame_error(0, "invalid Zstandard frame header"))?;
    if library_content != content {
        return Err(frame_error(4, "frame content size parser disagreement"));
    }
    let compressed = zstd_safe::find_frame_compressed_size(payload).map_err(codec_error)?;
    if compressed != payload.len() {
        return Err(frame_error(compressed, "trailing data or multiple frames"));
    }
    Ok(())
}

fn read_le(payload: &[u8], pos: &mut usize, width: usize) -> Result<u64, DecodeError> {
    let end = pos
        .checked_add(width)
        .ok_or_else(|| frame_error(*pos, "frame header length overflow"))?;
    let bytes = payload
        .get(*pos..end)
        .ok_or_else(|| frame_error(*pos, "truncated frame header"))?;
    let mut value = 0_u64;
    for (shift, &byte) in bytes.iter().enumerate() {
        value |= u64::from(byte) << (shift * 8);
    }
    *pos = end;
    Ok(value)
}

pub(super) fn decode_container(
    input: &[u8],
    budget: &mut DecodeBudget,
) -> Result<DecodedMessage, DecodeError> {
    let header = parse_header(input)?;
    budget.reserve_message_error_envelope(header.expanded)?;
    frame_header(header.payload, header.expanded)?;
    let len = usize::try_from(header.expanded).map_err(|_| {
        DecodeError::resource(
            "addressable message bytes",
            Some(header.expanded),
            usize::MAX as u64,
        )
    })?;
    let charge = budget.charge("decompressed message", header.expanded)?;
    let codec_charge = budget.charge("Zstandard context and workspace", CODEC_ENVELOPE)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(len)
        .map_err(|source| DecodeError::Allocation {
            site: "decompressed message",
            requested: header.expanded,
            source,
        })?;
    let mut context = decoder_context()?;
    let mut input = zstd_safe::InBuffer {
        src: header.payload,
        pos: 0,
    };
    #[expect(
        clippy::large_stack_arrays,
        reason = "the decoder uses a fixed 64 KiB scratch buffer without heap allocation"
    )]
    let mut scratch = [0_u8; SCRATCH];
    loop {
        let remaining = len - bytes.len();
        let out_len = if remaining == 0 {
            1
        } else {
            remaining.min(SCRATCH)
        };
        let previous = input.pos;
        let mut output = zstd_safe::OutBuffer::around(&mut scratch[..out_len]);
        let result = context
            .decompress_stream(&mut output, &mut input)
            .map_err(codec_error)?;
        let produced = output.pos();
        if produced > remaining {
            return Err(frame_error(
                input.pos,
                "decompressed output exceeds declared length",
            ));
        }
        bytes.extend_from_slice(&scratch[..produced]);
        if result == 0 {
            if input.pos != input.src.len() || bytes.len() != len {
                return Err(frame_error(
                    input.pos,
                    "compressed or decompressed length mismatch",
                ));
            }
            break;
        }
        if input.pos == previous && produced == 0 {
            return Err(frame_error(
                input.pos,
                "incomplete frame or decoder made no progress",
            ));
        }
    }
    drop(context);
    budget.release(codec_charge)?;
    budget.set_phase(Phase::Scan)?;
    Ok(DecodedMessage { bytes, charge })
}

fn decoder_context() -> Result<zstd_safe::DCtx<'static>, DecodeError> {
    let mut context = zstd_safe::DCtx::try_create().ok_or(DecodeError::ContextAllocation {
        site: "Zstandard decoder context",
    })?;
    let context_size = u64::try_from(context.sizeof())
        .map_err(|_| DecodeError::resource("Zstandard context bytes", None, CONTEXT_LIMIT))?;
    if context_size > CONTEXT_LIMIT {
        return Err(DecodeError::resource(
            "Zstandard context bytes",
            Some(context_size),
            CONTEXT_LIMIT,
        ));
    }
    context.init().map_err(codec_error)?;
    context
        .set_parameter(zstd_safe::DParameter::WindowLogMax(25))
        .map_err(codec_error)?;
    Ok(context)
}

mod encode;
pub(super) use encode::encode_container;
