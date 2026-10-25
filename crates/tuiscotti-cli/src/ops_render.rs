//! `render`: `.png` or `.html` output from a saved sample, fully offline.
//!
//! A sample carrying a stored image validates that PNG before
//! extraction: the bytes decode, the image parses, and the decoded
//! dimensions match the recorded ones. A state-only sample renders
//! through the built-in D02 profile, which the result reports; the
//! command never claims to reproduce a missing historical image.
//! Any other output extension is an invalid argument (exit 2).

use std::ffi::OsStr;
use std::path::Path;

use base64::Engine as _;
use tui_test::{PinnedFontError, RenderError};
use tuiscotti::approve::sample_code;
use tuiscotti::render_profile::{PROFILE_NAME, resolve_profile};
use tuiscotti::report::escape_html;
use tuiscotti::sample::Sample;

use crate::input::read_sample_bytes;
use crate::json_out::{CliResult, emit, encode_path};

/// Map an upstream render refusal to its Section 16.3 code.
fn render_error_code(error: &RenderError) -> &'static str {
    match error {
        RenderError::PinnedFont(PinnedFontError::MissingGlyph { .. }) => "MISSING_GLYPH",
        _ => "RENDER_FAILED",
    }
}

/// One static HTML page for a rendered sample with its PNG embedded as
/// a data URI: no external files, no absolute paths, deterministic
/// output for one input.
fn render_page(
    sample: &Sample,
    profile: &str,
    png_base64: &str,
    png_width: u32,
    png_height: u32,
) -> String {
    let test_id = escape_html(sample.test_id());
    let gate = sample.gate().as_str();
    let profile = escape_html(profile);
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head><meta charset=\"utf-8\">\n<title>tuiscotti render {test_id}</title>\n</head>\n<body>\n<p>test: {test_id}</p>\n<p>gate: {gate}</p>\n<p>profile: {profile}</p>\n<p>image: {png_width}x{png_height}</p>\n<img src=\"data:image/png;base64,{png_base64}\" alt=\"rendered sample {test_id}\">\n</body>\n</html>\n"
    )
}

/// True for the `.png` output extension (the only other spelling is
/// `.html`; anything else is rejected before this runs).
fn wants_png(out: &Path) -> bool {
    out.extension().and_then(OsStr::to_str) == Some("png")
}

/// Write the rendered bytes: raw PNG for `.png`, a static embedding
/// page for `.html`.
fn write_output(
    out: &Path,
    sample: &Sample,
    profile_text: &str,
    png_bytes: &[u8],
) -> Result<(), String> {
    if wants_png(out) {
        return std::fs::write(out, png_bytes)
            .map_err(|error| format!("cannot write {}: {error}", out.display()));
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(png_bytes);
    let (width, height) = image_dimensions(png_bytes);
    let page = render_page(sample, profile_text, &encoded, width, height);
    std::fs::write(out, page.as_bytes())
        .map_err(|error| format!("cannot write {}: {error}", out.display()))
}

/// Run `render` and return the process exit code.
///
/// Only the `--out` extension rejection is an argument error (exit 2);
/// every sample-content failure, including `INVALID_SAMPLE` from a
/// corrupt sample, exits 3 per Section 16.2.
pub(crate) fn cmd_render(sample_path: &Path, out: &Path, as_json: bool) -> i32 {
    let failed =
        |reason: String, code: &'static str, candidate_sha256: Option<String>, exit_code: i32| {
            eprintln!("error: {reason}");
            let result = CliResult::new(
                "render",
                "error",
                None,
                Some(reason),
                Some(encode_path(sample_path)),
                candidate_sha256,
                None,
                None,
                Some(code.to_string()),
            );
            emit(&result, exit_code, as_json)
        };
    let extension = out.extension().and_then(OsStr::to_str);
    if extension != Some("png") && extension != Some("html") {
        return failed(
            "unsupported output extension: expected a `.png` or `.html` --out path".to_string(),
            "INVALID_SAMPLE",
            None,
            2,
        );
    }
    let bytes = match read_sample_bytes(sample_path) {
        Ok(bytes) => bytes,
        Err(failure) => {
            return failed(failure.message(sample_path), failure.code(), None, 3);
        }
    };
    let candidate_sha256 = Sample::candidate_id(&bytes);
    let sample = match Sample::from_bytes(&bytes) {
        Ok(sample) => sample,
        Err(error) => {
            let reason = error.to_string();
            let code = sample_code(&error);
            return failed(reason, code, Some(candidate_sha256), 3);
        }
    };
    let test_id = sample.test_id().to_string();
    // A stored image validates before extraction; a state-only sample
    // renders through the built-in profile and reports it.
    let (png_bytes, profile_text, reason) = match sample.image() {
        Some(image) => match decode_stored_png(image.png_base64(), image.width(), image.height()) {
            Ok(png) => (png, "stored image".to_string(), None),
            Err(reason) => return failed(reason, "INVALID_IMAGE", Some(candidate_sha256), 3),
        },
        None => match render_state_only(&sample) {
            Ok(png) => (
                png,
                PROFILE_NAME.to_string(),
                Some(PROFILE_NAME.to_string()),
            ),
            Err((reason, code)) => return failed(reason, code, Some(candidate_sha256), 3),
        },
    };
    if let Err(reason) = write_output(out, &sample, &profile_text, &png_bytes) {
        return failed(reason, "IO", Some(candidate_sha256), 3);
    }
    let result = CliResult::new(
        "render",
        "pass",
        Some(test_id),
        reason,
        Some(encode_path(sample_path)),
        Some(candidate_sha256),
        None,
        Some(encode_path(out)),
        None,
    );
    emit(&result, 0, as_json)
}

/// Decode and validate a stored sample PNG: the Base64 decodes, the
/// image parses, and the decoded dimensions match the recorded ones.
fn decode_stored_png(encoded: &str, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(encoded.as_bytes())
        .map_err(|error| format!("stored PNG is not valid Base64: {error}"))?;
    let parsed = image::load_from_memory(&raw)
        .map_err(|error| format!("stored PNG does not parse: {error}"))?;
    let rgba = parsed.to_rgba8();
    if rgba.width() != width || rgba.height() != height {
        return Err(format!(
            "stored PNG decodes to {}x{} but the sample records {width}x{height}",
            rgba.width(),
            rgba.height()
        ));
    }
    Ok(raw)
}

/// Render a state-only sample through the built-in D02 profile.
fn render_state_only(sample: &Sample) -> Result<Vec<u8>, (String, &'static str)> {
    let profile = resolve_profile(PROFILE_NAME).map_err(|error| {
        (
            format!("unknown render profile: {error}"),
            "UNKNOWN_PROFILE",
        )
    })?;
    let rendered = tui_test::render_snapshot_png(sample.snapshot(), &profile.render_options())
        .map_err(|error| {
            (
                format!("state-only render failed: {error}"),
                render_error_code(&error),
            )
        })?;
    Ok(rendered.png().to_vec())
}

/// Decoded PNG dimensions for the HTML page; zero when undecodable
/// (the PNG path already validated, so this never fails there).
fn image_dimensions(png: &[u8]) -> (u32, u32) {
    match image::load_from_memory(png) {
        Ok(parsed) => {
            let rgba = parsed.to_rgba8();
            (rgba.width(), rgba.height())
        }
        Err(_) => (0, 0),
    }
}
