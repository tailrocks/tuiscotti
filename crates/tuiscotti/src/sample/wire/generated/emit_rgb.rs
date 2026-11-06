// @generated canonical emitter for Rgb.
pub(super) fn emit_rgb<S: ProtoSink, E: EncodeRgb>(
    adapter: &E,
    sink: &mut S,
) -> Result<(), EmitError> {
    let _ = (adapter, &*sink);
    let value = adapter.red();
    put_varint_field(sink, 1, u64::from(value))?;
    let value = adapter.green();
    put_varint_field(sink, 2, u64::from(value))?;
    let value = adapter.blue();
    put_varint_field(sink, 3, u64::from(value))?;
    Ok(())
}
