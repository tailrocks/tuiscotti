// @generated canonical emitter for Metadata.
pub(super) fn emit_metadata<S: ProtoSink, E: EncodeMetadata>(
    adapter: &E,
    sink: &mut S,
) -> Result<(), EmitError> {
    let _ = (adapter, &*sink);
    let value = adapter.captured_at_ms();
    put_varint_field(sink, 1, value)?;
    let value = adapter.revision();
    put_varint_field(sink, 2, value)?;
    let value = adapter.backend();
    put_bytes_field(sink, 3, value.as_bytes())?;
    let value = adapter.session();
    put_bytes_field(sink, 4, value.as_bytes())?;
    let value = adapter.source();
    put_varint_field(
        sink,
        5,
        u64::try_from(value.to_wire()).map_err(|_| EmitError::Invariant {
            detail: "negative enum value",
        })?,
    )?;
    Ok(())
}
