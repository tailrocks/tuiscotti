// @generated canonical emitter for Font.
pub(super) fn emit_font<S: ProtoSink, E: EncodeFont>(
    adapter: &E,
    sink: &mut S,
) -> Result<(), EmitError> {
    let _ = (adapter, &*sink);
    let value = adapter.digest();
    put_bytes_field(sink, 1, value.as_bytes())?;
    let value = adapter.face_index();
    put_varint_field(sink, 2, u64::from(value))?;
    let value = adapter.bold();
    put_varint_field(sink, 3, u64::from(value))?;
    let value = adapter.italic();
    put_varint_field(sink, 4, u64::from(value))?;
    let value = adapter.fallback_position();
    put_varint_field(sink, 5, value)?;
    Ok(())
}
