// @generated canonical emitter for Cell.
pub(super) fn emit_cell<S: ProtoSink, E: EncodeCell>(
    adapter: &E,
    sink: &mut S,
) -> Result<(), EmitError> {
    let _ = (adapter, &*sink);
    let value = adapter.text();
    put_bytes_field(sink, 1, value.as_bytes())?;
    let value = adapter.width();
    put_varint_field(sink, 2, u64::from(value))?;
    let value = adapter.fg();
    let mut nested_count = CountSink::new();
    emit_color(&value, &mut nested_count)?;
    let nested_len = nested_count.position();
    put_nested_prefix(sink, 3, nested_len)?;
    let nested_start = sink.position();
    emit_color(&value, sink)?;
    verify_nested_length(nested_start, sink.position(), nested_len)?;
    let value = adapter.bg();
    let mut nested_count = CountSink::new();
    emit_color(&value, &mut nested_count)?;
    let nested_len = nested_count.position();
    put_nested_prefix(sink, 4, nested_len)?;
    let nested_start = sink.position();
    emit_color(&value, sink)?;
    verify_nested_length(nested_start, sink.position(), nested_len)?;
    let value = adapter.bold();
    put_varint_field(sink, 5, u64::from(value))?;
    let value = adapter.dim();
    put_varint_field(sink, 6, u64::from(value))?;
    let value = adapter.italic();
    put_varint_field(sink, 7, u64::from(value))?;
    let value = adapter.inverse();
    put_varint_field(sink, 8, u64::from(value))?;
    let value = adapter.invisible();
    put_varint_field(sink, 9, u64::from(value))?;
    let value = adapter.strike();
    put_varint_field(sink, 10, u64::from(value))?;
    let value = adapter.blink();
    put_varint_field(sink, 11, u64::from(value))?;
    let value = adapter.underline();
    put_varint_field(
        sink,
        12,
        u64::try_from(value.to_wire()).map_err(|_| EmitError::Invariant {
            detail: "negative enum value",
        })?,
    )?;
    let value = adapter.underline_color();
    let mut nested_count = CountSink::new();
    emit_color(&value, &mut nested_count)?;
    let nested_len = nested_count.position();
    put_nested_prefix(sink, 13, nested_len)?;
    let nested_start = sink.position();
    emit_color(&value, sink)?;
    verify_nested_length(nested_start, sink.position(), nested_len)?;
    if let Some(value) = adapter.link_uri() {
        put_bytes_field(sink, 14, value.as_bytes())?;
    }
    if let Some(value) = adapter.link_id() {
        put_bytes_field(sink, 15, value.as_bytes())?;
    }
    Ok(())
}
