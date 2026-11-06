// @generated canonical emitter for Cursor.
pub(super) fn emit_cursor<S: ProtoSink, E: EncodeCursor>(
    adapter: &E,
    sink: &mut S,
) -> Result<(), EmitError> {
    let _ = (adapter, &*sink);
    let value = adapter.col();
    put_varint_field(sink, 1, u64::from(value))?;
    let value = adapter.row();
    put_varint_field(sink, 2, u64::from(value))?;
    let value = adapter.in_capture();
    put_varint_field(sink, 3, u64::from(value))?;
    let value = adapter.visible();
    put_varint_field(sink, 4, u64::from(value))?;
    let value = adapter.shape();
    put_varint_field(
        sink,
        5,
        u64::try_from(value.to_wire()).map_err(|_| EmitError::Invariant {
            detail: "negative enum value",
        })?,
    )?;
    let value = adapter.color();
    let mut nested_count = CountSink::new();
    emit_rgb(&value, &mut nested_count)?;
    let nested_len = nested_count.position();
    put_nested_prefix(sink, 6, nested_len)?;
    let nested_start = sink.position();
    emit_rgb(&value, sink)?;
    verify_nested_length(nested_start, sink.position(), nested_len)?;
    if let Some(value) = adapter.blink() {
        put_varint_field(sink, 7, u64::from(value))?;
    }
    Ok(())
}
