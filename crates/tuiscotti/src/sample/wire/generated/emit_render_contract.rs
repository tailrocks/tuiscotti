// @generated canonical emitter for RenderContract.
pub(super) fn emit_render_contract<S: ProtoSink, E: EncodeRenderContract>(
    adapter: &E,
    sink: &mut S,
) -> Result<(), EmitError> {
    let _ = (adapter, &*sink);
    let value = adapter.profile();
    put_bytes_field(sink, 1, value.as_bytes())?;
    let value = adapter.contract_version();
    put_varint_field(sink, 2, u64::from(value))?;
    let value = adapter.renderer_version();
    put_bytes_field(sink, 3, value.as_bytes())?;
    let value = adapter.raster_policy();
    put_bytes_field(sink, 4, value.as_bytes())?;
    let values = adapter.fonts();
    for value in values {
        let mut nested_count = CountSink::new();
        emit_font(&value, &mut nested_count)?;
        let nested_len = nested_count.position();
        put_nested_prefix(sink, 5, nested_len)?;
        let nested_start = sink.position();
        emit_font(&value, sink)?;
        verify_nested_length(nested_start, sink.position(), nested_len)?;
    }
    let value = adapter.cell_width();
    put_fixed32_field(sink, 6, value.to_bits())?;
    let value = adapter.cell_height();
    put_fixed32_field(sink, 7, value.to_bits())?;
    let value = adapter.font_size();
    put_fixed32_field(sink, 8, value.to_bits())?;
    let value = adapter.baseline();
    put_fixed32_field(sink, 9, value.to_bits())?;
    let value = adapter.scale();
    put_fixed64_field(sink, 10, value.to_bits())?;
    let value = adapter.title_bar();
    put_varint_field(sink, 11, u64::from(value))?;
    let value = adapter.palette_digest();
    put_bytes_field(sink, 12, value.as_bytes())?;
    let value = adapter.cursor_phase();
    put_varint_field(
        sink,
        13,
        u64::try_from(value.to_wire()).map_err(|_| EmitError::Invariant {
            detail: "negative enum value",
        })?,
    )?;
    if let Some(value) = adapter.blink() {
        put_varint_field(sink, 14, u64::from(value))?;
    }
    let value = adapter.platform_key();
    put_bytes_field(sink, 15, value.as_bytes())?;
    Ok(())
}
