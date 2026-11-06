include!("scanner_core.rs");
include!("scanner_engine.rs");
include!("scanner_validate.rs");
include!("scanner_finish.rs");
fn fields_for(kind: &str) -> Option<&'static [FieldSpec]> {
    match kind {
        "Cell" => Some(FIELDS_CELL),
        "Color" => Some(FIELDS_COLOR),
        "Cursor" => Some(FIELDS_CURSOR),
        "DefaultColor" => Some(FIELDS_DEFAULTCOLOR),
        "Font" => Some(FIELDS_FONT),
        "Image" => Some(FIELDS_IMAGE),
        "Metadata" => Some(FIELDS_METADATA),
        "Observation" => Some(FIELDS_OBSERVATION),
        "RenderContract" => Some(FIELDS_RENDERCONTRACT),
        "Rgb" => Some(FIELDS_RGB),
        "Sample" => Some(FIELDS_SAMPLE),
        "Snapshot" => Some(FIELDS_SNAPSHOT),
        _ => None,
    }
}
fn repeated_limit(message: &str, field: &str) -> Option<u64> {
    match (message, field) {
        ("Snapshot", "palette") => Some(256),
        _ => None,
    }
}
fn repeated_exact_count(message: &str, field: &str) -> Option<u64> {
    match (message, field) {
        ("Snapshot", "palette") => Some(256),
        _ => None,
    }
}
fn enum_accepts(kind: &str, value: i32) -> bool {
    match kind {
        "ImageFormat" => matches!(value, 1),
        "CursorPhase" | "Gate" | "Source" => matches!(value, 1..=2),
        "CursorShape" | "Scope" => matches!(value, 1..=3),
        "Underline" => matches!(value, 1..=6),
        _ => false,
    }
}
fn image_grid_scale() -> (u32, u32) {
    (10, 20)
}
