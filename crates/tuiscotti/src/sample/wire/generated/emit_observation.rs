// @generated canonical emitter for Observation.
pub(super) fn emit_observation<S: ProtoSink, E: EncodeObservation>(
    adapter: &E,
    sink: &mut S,
) -> Result<(), EmitError> {
    let _ = (adapter, &*sink);
    if let Some(value) = adapter.note() {
        put_bytes_field(sink, 1, value.as_bytes())?;
    }
    Ok(())
}
