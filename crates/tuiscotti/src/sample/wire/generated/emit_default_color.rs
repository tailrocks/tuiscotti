// @generated canonical emitter for DefaultColor.
pub(super) fn emit_default_color<S: ProtoSink, E: EncodeDefaultColor>(
    adapter: &E,
    sink: &mut S,
) -> Result<(), EmitError> {
    let _ = (adapter, &*sink);
    sink.put(&[])?;
    Ok(())
}
