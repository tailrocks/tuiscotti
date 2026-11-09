use super::Sample;
use super::budget::DecodeBudget;
use super::decode_error::{BinaryFormatError, DecodeError, WirePath};

mod generated {
    include!("generated.rs");
}

pub(super) fn field_name(id: u8) -> &'static str {
    generated::field_name(id)
}

mod map;

mod write;

pub(super) fn decode_sample(bytes: &[u8]) -> Result<Sample, DecodeError> {
    decode_sample_with_input_allocation(bytes, bytes.len() as u64)
}

pub(super) fn decode_sample_with_input_allocation(
    bytes: &[u8],
    input_allocation: u64,
) -> Result<Sample, DecodeError> {
    if input_allocation < bytes.len() as u64 {
        return Err(DecodeError::Invariant {
            detail: "input allocation smaller than bytes",
        });
    }
    let mut budget = DecodeBudget::new()?;
    let input = budget.charge("encoded sample input", input_allocation)?;
    budget.set_phase(super::budget::Phase::Codec)?;
    let message = super::container::decode_container(bytes, &mut budget)?;
    let root = generated::scan_sample(&message.bytes, &mut budget)?;
    let sample = map::map_sample(root, &mut budget)?;
    sample.validate_structure().map_err(DecodeError::Domain)?;
    drop(message.bytes);
    budget.release(message.charge)?;
    budget.release(input)?;
    budget.finish_output()?;
    Ok(sample)
}

pub(super) fn decode_owned_sample(bytes: Vec<u8>) -> Result<Sample, DecodeError> {
    decode_owned_sample_with_peak(bytes).map(|(sample, _)| sample)
}

fn decode_owned_sample_with_peak(bytes: Vec<u8>) -> Result<(Sample, u64), DecodeError> {
    let mut budget = DecodeBudget::new()?;
    let input = budget.charge("encoded sample input", bytes.capacity() as u64)?;
    budget.set_phase(super::budget::Phase::Codec)?;
    let message = super::container::decode_container(&bytes, &mut budget)?;
    let root = generated::scan_sample(&message.bytes, &mut budget)?;
    let sample = map::map_sample(root, &mut budget)?;
    sample.validate_structure().map_err(DecodeError::Domain)?;
    drop(message.bytes);
    budget.release(message.charge)?;
    drop(bytes);
    budget.release(input)?;
    budget.finish_output()?;
    Ok((sample, budget.peak()))
}

pub(super) fn decode_file(path: &std::path::Path) -> Result<Sample, super::SampleError> {
    let bytes = super::io::read_bounded(path)?;
    decode_owned_sample(bytes).map_err(Into::into)
}

pub(super) fn encode_sample(sample: &Sample, level: i32) -> Result<Vec<u8>, super::SampleError> {
    let message = write::encode_sample(sample)?;
    super::container::encode_container(&message, level).map_err(Into::into)
}
