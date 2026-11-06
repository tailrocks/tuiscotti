// @generated canonical Protobuf writer traversal.
include!("writer_common.rs");
include!("emit_cell.rs");
include!("emit_color.rs");
include!("emit_cursor.rs");
include!("emit_default_color.rs");
include!("emit_font.rs");
include!("emit_image.rs");
include!("emit_metadata.rs");
include!("emit_observation.rs");
include!("emit_render_contract.rs");
include!("emit_rgb.rs");
include!("emit_sample.rs");
include!("emit_snapshot.rs");
pub(super) fn encoded_len<E: EncodeSample>(adapter: &E) -> Result<u64, EmitError> {
    let mut sink = CountSink::new();
    emit_sample(adapter, &mut sink)?;
    Ok(sink.position())
}
pub(super) fn encode_into<S: ProtoSink, E: EncodeSample>(
    adapter: &E,
    sink: &mut S,
) -> Result<(), EmitError> {
    emit_sample(adapter, sink)
}
