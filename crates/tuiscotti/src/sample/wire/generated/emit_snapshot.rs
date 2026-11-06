// @generated canonical emitter for Snapshot.
pub(super) fn emit_snapshot<S: ProtoSink, E: EncodeSnapshot>(
    adapter: &E,
    sink: &mut S,
) -> Result<(), EmitError> {
    let _ = (adapter, &*sink);
    let value = adapter.schema();
    put_varint_field(sink, 1, u64::from(value))?;
    let value = adapter.cols();
    put_varint_field(sink, 2, u64::from(value))?;
    let value = adapter.rows();
    put_varint_field(sink, 3, u64::from(value))?;
    emit_snapshot_scope(adapter, sink)?;
    let value = adapter.viewport_origin();
    put_varint_field(sink, 5, value)?;
    let values = adapter.cells();
    for value in values {
        let mut nested_count = CountSink::new();
        emit_cell(&value, &mut nested_count)?;
        let nested_len = nested_count.position();
        put_nested_prefix(sink, 6, nested_len)?;
        let nested_start = sink.position();
        emit_cell(&value, sink)?;
        verify_nested_length(nested_start, sink.position(), nested_len)?;
    }
    let values = adapter.palette();
    for value in values {
        let mut nested_count = CountSink::new();
        emit_rgb(&value, &mut nested_count)?;
        let nested_len = nested_count.position();
        put_nested_prefix(sink, 7, nested_len)?;
        let nested_start = sink.position();
        emit_rgb(&value, sink)?;
        verify_nested_length(nested_start, sink.position(), nested_len)?;
    }
    let value = adapter.default_foreground();
    let mut nested_count = CountSink::new();
    emit_rgb(&value, &mut nested_count)?;
    let nested_len = nested_count.position();
    put_nested_prefix(sink, 8, nested_len)?;
    let nested_start = sink.position();
    emit_rgb(&value, sink)?;
    verify_nested_length(nested_start, sink.position(), nested_len)?;
    let value = adapter.default_background();
    let mut nested_count = CountSink::new();
    emit_rgb(&value, &mut nested_count)?;
    let nested_len = nested_count.position();
    put_nested_prefix(sink, 9, nested_len)?;
    let nested_start = sink.position();
    emit_rgb(&value, sink)?;
    verify_nested_length(nested_start, sink.position(), nested_len)?;
    let value = adapter.default_cursor();
    let mut nested_count = CountSink::new();
    emit_rgb(&value, &mut nested_count)?;
    let nested_len = nested_count.position();
    put_nested_prefix(sink, 10, nested_len)?;
    let nested_start = sink.position();
    emit_rgb(&value, sink)?;
    verify_nested_length(nested_start, sink.position(), nested_len)?;
    let value = adapter.cursor();
    let mut nested_count = CountSink::new();
    emit_cursor(&value, &mut nested_count)?;
    let nested_len = nested_count.position();
    put_nested_prefix(sink, 11, nested_len)?;
    let nested_start = sink.position();
    emit_cursor(&value, sink)?;
    verify_nested_length(nested_start, sink.position(), nested_len)?;
    if let Some(value) = adapter.title() {
        put_bytes_field(sink, 12, value.as_bytes())?;
    }
    let value = adapter.metadata();
    let mut nested_count = CountSink::new();
    emit_metadata(&value, &mut nested_count)?;
    let nested_len = nested_count.position();
    put_nested_prefix(sink, 13, nested_len)?;
    let nested_start = sink.position();
    emit_metadata(&value, sink)?;
    verify_nested_length(nested_start, sink.position(), nested_len)?;
    Ok(())
}
fn emit_snapshot_scope<S: ProtoSink, E: EncodeSnapshot>(
    adapter: &E,
    sink: &mut S,
) -> Result<(), EmitError> {
    let value = adapter.scope();
    put_varint_field(
        sink,
        4,
        u64::try_from(value.to_wire()).map_err(|_| EmitError::Invariant {
            detail: "negative enum value",
        })?,
    )?;
    Ok(())
}
