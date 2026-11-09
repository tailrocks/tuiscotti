// @generated canonical emitter for Color.
pub(super) fn emit_color<S: ProtoSink, E: EncodeColor>(
    adapter: &E,
    sink: &mut S,
) -> Result<(), EmitError> {
    let _ = (adapter, &*sink);
    let mut oneof_value = 0u32;
    if let Some(value) = adapter.default_color() {
        oneof_value = oneof_value.checked_add(1).ok_or(EmitError::Invariant {
            detail: "oneof count overflow",
        })?;
        let mut nested_count = CountSink::new();
        emit_default_color(&value, &mut nested_count)?;
        let nested_len = nested_count.position();
        put_nested_prefix(sink, 1, nested_len)?;
        let nested_start = sink.position();
        emit_default_color(&value, sink)?;
        verify_nested_length(nested_start, sink.position(), nested_len)?;
    }
    if let Some(value) = adapter.indexed() {
        oneof_value = oneof_value.checked_add(1).ok_or(EmitError::Invariant {
            detail: "oneof count overflow",
        })?;
        put_varint_field(sink, 2, u64::from(value))?;
    }
    if let Some(value) = adapter.rgb() {
        oneof_value = oneof_value.checked_add(1).ok_or(EmitError::Invariant {
            detail: "oneof count overflow",
        })?;
        let mut nested_count = CountSink::new();
        emit_rgb(&value, &mut nested_count)?;
        let nested_len = nested_count.position();
        put_nested_prefix(sink, 3, nested_len)?;
        let nested_start = sink.position();
        emit_rgb(&value, sink)?;
        verify_nested_length(nested_start, sink.position(), nested_len)?;
    }
    if oneof_value != 1 {
        return Err(EmitError::Invariant {
            detail: "oneof must select exactly one branch",
        });
    }
    Ok(())
}
