// @generated canonical emitter for Sample.
pub(super) fn emit_sample<S: ProtoSink, E: EncodeSample>(
    adapter: &E,
    sink: &mut S,
) -> Result<(), EmitError> {
    let _ = (adapter, &*sink);
    let value = adapter.schema();
    put_bytes_field(sink, 1, value.as_bytes())?;
    let value = adapter.test_id();
    put_bytes_field(sink, 2, value.as_bytes())?;
    let value = adapter.gate();
    put_varint_field(
        sink,
        3,
        u64::try_from(value.to_wire()).map_err(|_| EmitError::Invariant {
            detail: "negative enum value",
        })?,
    )?;
    let value = adapter.snapshot();
    let mut nested_count = CountSink::new();
    emit_snapshot(&value, &mut nested_count)?;
    let nested_len = nested_count.position();
    put_nested_prefix(sink, 4, nested_len)?;
    let nested_start = sink.position();
    emit_snapshot(&value, sink)?;
    verify_nested_length(nested_start, sink.position(), nested_len)?;
    if let Some(value) = adapter.render_contract() {
        let mut nested_count = CountSink::new();
        emit_render_contract(&value, &mut nested_count)?;
        let nested_len = nested_count.position();
        put_nested_prefix(sink, 5, nested_len)?;
        let nested_start = sink.position();
        emit_render_contract(&value, sink)?;
        verify_nested_length(nested_start, sink.position(), nested_len)?;
    }
    if let Some(value) = adapter.image() {
        let mut nested_count = CountSink::new();
        emit_image(&value, &mut nested_count)?;
        let nested_len = nested_count.position();
        put_nested_prefix(sink, 6, nested_len)?;
        let nested_start = sink.position();
        emit_image(&value, sink)?;
        verify_nested_length(nested_start, sink.position(), nested_len)?;
    }
    let value = adapter.observation();
    let mut nested_count = CountSink::new();
    emit_observation(&value, &mut nested_count)?;
    let nested_len = nested_count.position();
    put_nested_prefix(sink, 7, nested_len)?;
    let nested_start = sink.position();
    emit_observation(&value, sink)?;
    verify_nested_length(nested_start, sink.position(), nested_len)?;
    Ok(())
}
