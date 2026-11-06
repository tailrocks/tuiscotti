// @generated canonical emitter for Image.
pub(super) fn emit_image<S: ProtoSink, E: EncodeImage>(
    adapter: &E,
    sink: &mut S,
) -> Result<(), EmitError> {
    let _ = (adapter, &*sink);
    let value = adapter.format();
    put_varint_field(
        sink,
        1,
        u64::try_from(value.to_wire()).map_err(|_| EmitError::Invariant {
            detail: "negative enum value",
        })?,
    )?;
    let value = adapter.width();
    put_varint_field(sink, 2, u64::from(value))?;
    let value = adapter.height();
    put_varint_field(sink, 3, u64::from(value))?;
    let value = adapter.png_sha256();
    put_bytes_field(sink, 4, value.as_bytes())?;
    let value = adapter.rgba_sha256();
    put_bytes_field(sink, 5, value.as_bytes())?;
    let value = adapter.png();
    put_bytes_field(sink, 6, value)?;
    Ok(())
}
