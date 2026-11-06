use super::super::decode_error::EncodeError;
use super::{HEADER_LEN, MAX_INPUT, SCRATCH, ZstdError, frame_header};
use zstd_safe::zstd_sys::ZSTD_EndDirective;

pub(in crate::sample) fn encode_container(
    message: &[u8],
    level: i32,
) -> Result<Vec<u8>, EncodeError> {
    encode_with_limit(
        message,
        level,
        usize::try_from(MAX_INPUT).unwrap_or(usize::MAX),
    )
}

fn encode_with_limit(
    message: &[u8],
    level: i32,
    file_limit: usize,
) -> Result<Vec<u8>, EncodeError> {
    let message_len = u64::try_from(message.len()).map_err(|_| EncodeError::Limit {
        resource: "decompressed message bytes",
        claimed: None,
        limit: MAX_INPUT,
    })?;
    if message_len > MAX_INPUT {
        return Err(EncodeError::Limit {
            resource: "decompressed message bytes",
            claimed: Some(message_len),
            limit: MAX_INPUT,
        });
    }
    if file_limit < HEADER_LEN || u64::try_from(file_limit).map_or(true, |n| n > MAX_INPUT) {
        return Err(EncodeError::Invariant {
            detail: "invalid container output limit",
        });
    }
    let mut output = reserved_output()?;
    let mut context = encoder_context(level, message_len)?;
    #[expect(
        clippy::large_stack_arrays,
        reason = "the encoder uses a fixed 64 KiB scratch buffer without heap allocation"
    )]
    let mut scratch = [0_u8; SCRATCH];
    for chunk in message.chunks(SCRATCH) {
        let mut input = zstd_safe::InBuffer { src: chunk, pos: 0 };
        while input.pos < input.src.len() {
            encode_step(
                &mut context,
                &mut input,
                &mut output,
                file_limit,
                &mut scratch,
                ZSTD_EndDirective::ZSTD_e_continue,
            )?;
        }
    }
    let mut empty = zstd_safe::InBuffer { src: &[], pos: 0 };
    loop {
        let remaining = encode_step(
            &mut context,
            &mut empty,
            &mut output,
            file_limit,
            &mut scratch,
            ZSTD_EndDirective::ZSTD_e_end,
        )?;
        if remaining == 0 {
            break;
        }
    }
    let payload = &output[HEADER_LEN..];
    frame_header(payload, message_len).map_err(|_| EncodeError::Invariant {
        detail: "encoder produced an invalid Zstandard frame",
    })?;
    let payload_size = payload.len();
    finish_header(output, payload_size, message_len)
}

fn reserved_output() -> Result<Vec<u8>, EncodeError> {
    let mut output = Vec::new();
    let requested = u64::try_from(HEADER_LEN).unwrap_or(u64::MAX);
    output
        .try_reserve_exact(HEADER_LEN)
        .map_err(|source| EncodeError::Allocation {
            site: "encoded container",
            requested,
            source,
        })?;
    output.resize(HEADER_LEN, 0);
    Ok(output)
}

fn encoder_context(level: i32, message_len: u64) -> Result<zstd_safe::CCtx<'static>, EncodeError> {
    let mut context = zstd_safe::CCtx::try_create().ok_or(EncodeError::ContextAllocation {
        site: "Zstandard encoder context",
    })?;
    for parameter in [
        zstd_safe::CParameter::CompressionLevel(level),
        zstd_safe::CParameter::WindowLog(25),
        zstd_safe::CParameter::ContentSizeFlag(true),
        zstd_safe::CParameter::ChecksumFlag(true),
        zstd_safe::CParameter::DictIdFlag(false),
        zstd_safe::CParameter::NbWorkers(0),
    ] {
        context
            .set_parameter(parameter)
            .map_err(encode_codec_error)?;
    }
    context
        .set_pledged_src_size(Some(message_len))
        .map_err(encode_codec_error)?;
    Ok(context)
}

fn finish_header(
    mut output: Vec<u8>,
    payload_size: usize,
    message_len: u64,
) -> Result<Vec<u8>, EncodeError> {
    let payload_len = u64::try_from(payload_size).map_err(|_| EncodeError::Invariant {
        detail: "compressed payload length is not representable",
    })?;
    output[..8].copy_from_slice(b"TUISCPB\0");
    output[8..10].copy_from_slice(&1_u16.to_le_bytes());
    output[10..12].copy_from_slice(&1_u16.to_le_bytes());
    output[12..16].copy_from_slice(&1_u32.to_le_bytes());
    output[16..24].copy_from_slice(&message_len.to_le_bytes());
    output[24..32].copy_from_slice(&payload_len.to_le_bytes());
    Ok(output)
}

fn encode_codec_error(code: usize) -> EncodeError {
    EncodeError::Codec {
        source: ZstdError { code },
    }
}

fn encode_step(
    context: &mut zstd_safe::CCtx<'_>,
    input: &mut zstd_safe::InBuffer<'_>,
    output: &mut Vec<u8>,
    file_limit: usize,
    scratch: &mut [u8; SCRATCH],
    directive: ZSTD_EndDirective,
) -> Result<usize, EncodeError> {
    let remaining = file_limit - output.len();
    let available = if remaining == 0 {
        1
    } else {
        remaining.min(SCRATCH)
    };
    let previous = input.pos;
    let mut dst = zstd_safe::OutBuffer::around(&mut scratch[..available]);
    let hint = context
        .compress_stream2(&mut dst, input, directive)
        .map_err(encode_codec_error)?;
    let produced = dst.pos();
    if produced > remaining {
        return Err(EncodeError::Limit {
            resource: "encoded sample bytes",
            claimed: None,
            limit: u64::try_from(file_limit).unwrap_or(u64::MAX),
        });
    }
    let required = output
        .len()
        .checked_add(produced)
        .ok_or(EncodeError::Limit {
            resource: "encoded sample bytes",
            claimed: None,
            limit: u64::try_from(file_limit).unwrap_or(u64::MAX),
        })?;
    reserve_output(output, required, file_limit)?;
    output.extend_from_slice(&scratch[..produced]);
    if input.pos == previous
        && produced == 0
        && !(directive == ZSTD_EndDirective::ZSTD_e_end && hint == 0)
    {
        return Err(EncodeError::Invariant {
            detail: "Zstandard encoder made no progress",
        });
    }
    Ok(hint)
}

fn reserve_output(
    output: &mut Vec<u8>,
    required: usize,
    file_limit: usize,
) -> Result<(), EncodeError> {
    if required > file_limit {
        return Err(EncodeError::Limit {
            resource: "encoded sample bytes",
            claimed: u64::try_from(required).ok(),
            limit: u64::try_from(file_limit).unwrap_or(u64::MAX),
        });
    }
    if required <= output.capacity() {
        return Ok(());
    }

    let doubled_capacity = output.capacity().saturating_mul(2);
    let target = required.max(doubled_capacity).min(file_limit);
    let additional = target.saturating_sub(output.len());
    let requested = u64::try_from(target).unwrap_or(u64::MAX);
    output
        .try_reserve_exact(additional)
        .map_err(|source| EncodeError::Allocation {
            site: "encoded container",
            requested,
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiny_container_starts_with_a_small_allocation() {
        let output = reserved_output().expect("header allocation should succeed");

        assert_eq!(output.len(), HEADER_LEN);
        assert!(output.capacity() < 1024);
    }

    #[test]
    fn encoded_container_respects_exact_output_limit() {
        let message = b"tiny sample";
        let encoded = encode_with_limit(message, 3, 256).expect("sample should fit");

        assert_eq!(encoded.len(), 56);
        assert_eq!(
            encode_with_limit(message, 3, encoded.len()).expect("exact limit should fit"),
            encoded
        );
        assert!(matches!(
            encode_with_limit(message, 3, encoded.len() - 1),
            Err(EncodeError::Limit {
                resource: "encoded sample bytes",
                ..
            })
        ));
    }
}
