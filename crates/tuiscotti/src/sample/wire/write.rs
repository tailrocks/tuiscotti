use super::super::{Sample, SampleError};

mod adapter;
mod sink;
#[cfg(test)]
mod tests;

use adapter::SampleAdapter;
use sink::{WriterSink, map_emit_error};

/// Encode a validated domain sample as its canonical protobuf message.
///
/// The generated traversal owns descriptor order and shortest protobuf encodings.
pub(super) fn encode_sample(sample: &Sample) -> Result<Vec<u8>, SampleError> {
    sample.validate_structure()?;
    let adapter = SampleAdapter::new(sample)?;
    let length = super::generated::encoded_len(&adapter).map_err(map_emit_error)?;
    let mut sink = WriterSink::new(length)?;
    super::generated::encode_into(&adapter, &mut sink).map_err(map_emit_error)?;
    sink.finish()
}
