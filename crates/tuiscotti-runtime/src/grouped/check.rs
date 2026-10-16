use super::{
    ArtifactPaths, ArtifactSet, CheapActuals, GroupedOutcome, GroupedStore, first_difference,
    fresh_grouped, missing_approved_names, run_byte_gates, run_html_gate, validate_name,
};
use crate::snapshot::{SnapshotError, Status, compare_cells, write_atomic};
use std::path::{Path, PathBuf};
use tuiscotti_core::frame::Frame;
use tuiscotti_render::diff;
use tuiscotti_render::formats::ansi::frame_from_ansi;
use tuiscotti_render::profile::Profile;
use tuiscotti_render::render::{self, Renderer};

/// `root/<name>.<ext>` for the four artifacts plus the frame sidecar.
/// `name` must be pre-validated ([`validate_name`]).
pub(crate) fn artifact_paths(root: &Path, name: &str) -> ArtifactPaths {
    ArtifactPaths {
        ansi: root.join(format!("{name}.ansi")),
        txt: root.join(format!("{name}.txt")),
        png: root.join(format!("{name}.png")),
        html: root.join(format!("{name}.html")),
        frame_json: root.join(format!("{name}.frame.json")),
    }
}

/// Missing-glyph sidecar next to a PNG (`<name>.png.fidelity.json`).
fn fidelity_sidecar(png: &Path) -> PathBuf {
    png.with_extension("png.fidelity.json")
}

pub(crate) fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, SnapshotError> {
    match std::fs::read(path) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(SnapshotError(format!(
            "cannot read {}: {e}",
            path.display()
        ))),
    }
}

impl GroupedStore {
    /// Check one frame against the approved artifacts. Writes the actual
    /// artifacts (plus debug sidecars) under the actual root BEFORE
    /// comparing; on pixel mismatch also writes the diff PNG under the diff
    /// root. Gate semantics are in the module docs.
    ///
    /// The default gate renders through the thread-local shared
    /// [`Renderer`]; custom profiles/faces construct per call, so bulk
    /// gates with custom profiles should build one and call
    /// [`Self::check_with`] instead.
    ///
    /// # Errors
    ///
    /// Returns `SnapshotError` when rendering, reading, or writing artifacts fails.
    pub fn check(
        &self,
        name: &str,
        actual: &Frame,
        profile: &Profile,
        faces: &tuiscotti_render::profile::FontFaces<'_>,
        pixel_threshold: f64,
    ) -> Result<GroupedOutcome, SnapshotError> {
        Renderer::with_profile(profile, faces, |r| {
            self.check_with(r, name, actual, pixel_threshold)
        })
    }

    /// [`Self::check`] through a caller-owned [`Renderer`], so a suite
    /// reuses the parsed faces and the glyph cache across checks.
    ///
    /// # Errors
    ///
    /// Returns `SnapshotError` when rendering, reading, or writing artifacts fails.
    pub fn check_with(
        &self,
        renderer: &mut Renderer,
        name: &str,
        actual: &Frame,
        pixel_threshold: f64,
    ) -> Result<GroupedOutcome, SnapshotError> {
        // C04: validate pixel_threshold up front so NaN/out-of-range fails loudly.
        let _perceptual = diff::PerceptualPolicy::new(pixel_threshold)?;
        if self.artifacts == ArtifactSet::AnsiCursor {
            return self.check_ansi_cursor(renderer, name, actual, pixel_threshold);
        }
        let cheap = self.write_cheap_actuals(name, actual)?;
        let mut grouped = fresh_grouped(name, &cheap, actual);

        // Missing ANY approved artifact fails closed as MissingApproval.
        let approved_ansi = read_optional(&grouped.approved.ansi)?;
        let approved_txt = read_optional(&grouped.approved.txt)?;
        let approved_html = read_optional(&grouped.approved.html)?;
        let approved_png = read_optional(&grouped.approved.png)?;
        let missing = missing_approved_names([
            approved_ansi.as_deref(),
            approved_txt.as_deref(),
            approved_html.as_deref(),
            approved_png.as_deref(),
        ]);
        if !missing.is_empty() {
            self.seal_missing_approval(
                renderer,
                name,
                actual,
                &mut grouped,
                pixel_threshold,
                &missing,
            )?;
            return Ok(grouped);
        }
        let (Some(approved_ansi), Some(approved_txt), Some(approved_html), Some(approved_png)) =
            (approved_ansi, approved_txt, approved_html, approved_png)
        else {
            return Err(SnapshotError(format!(
                "approved artifacts for `{name}` failed the presence check twice"
            )));
        };
        grouped.outcome.expected_png = Some(grouped.approved.png.clone());
        grouped.outcome.expected_png_bytes = Some(approved_png.clone());

        let mut notes: Vec<String> = Vec::new();
        run_byte_gates(
            &mut grouped,
            &approved_ansi,
            &cheap.ansi,
            &approved_txt,
            &cheap.txt,
            &mut notes,
        );

        // C02: actual evidence always renders fresh from the candidate frame.
        let (actual_html_bytes, actual_png_bytes) =
            Self::render_actual_artifacts(renderer, name, actual, &grouped.actual)?;
        run_html_gate(&mut grouped, &approved_html, &actual_html_bytes, &mut notes);
        self.run_png_gate(
            name,
            &mut grouped,
            &approved_png,
            &actual_png_bytes,
            pixel_threshold,
            &mut notes,
        )?;

        self.finalize_check(renderer, name, &mut grouped, pixel_threshold, &notes)?;
        Ok(grouped)
    }

    /// Minimal-artifact check (`ArtifactSet::AnsiCursor`): only `<name>.ansi`
    /// (with cursor trailer) is committed under `approved_root`. Passing checks
    /// never rasterize PNG/HTML; mismatches render actual + expected PNG/HTML
    /// and a diff PNG to the scratch roots.
    fn check_ansi_cursor(
        &self,
        renderer: &mut Renderer,
        name: &str,
        actual: &Frame,
        pixel_threshold: f64,
    ) -> Result<GroupedOutcome, SnapshotError> {
        let cheap = self.write_cheap_actuals(name, actual)?;
        let mut grouped = fresh_grouped(name, &cheap, actual);
        let profile_name = renderer.profile().name.clone();

        let Some(approved_ansi) = read_optional(&grouped.approved.ansi)? else {
            self.seal_missing_approval(
                renderer,
                name,
                actual,
                &mut grouped,
                pixel_threshold,
                &[".ansi"],
            )?;
            return Ok(grouped);
        };

        if approved_ansi == cheap.ansi.as_bytes() {
            grouped.ansi_match = Some(true);
            grouped.outcome.status = Status::Matched;
            grouped.outcome.digest_expected = Some(grouped.outcome.digest_actual.clone());
            self.seal_candidate(&profile_name, name, &grouped.actual)?;
            self.seal_verdict(name, &grouped, pixel_threshold, &["ansi-cursor-byte-gate"])?;
            return Ok(grouped);
        }

        grouped.ansi_match = Some(false);
        let (_, actual_png_bytes) =
            Self::render_actual_artifacts(renderer, name, actual, &grouped.actual)?;

        let Ok(approved_str) = std::str::from_utf8(&approved_ansi) else {
            grouped.outcome.status = Status::CorruptApproval;
            grouped.outcome.note = format!(
                "approved file {} is not valid UTF-8",
                grouped.approved.ansi.display()
            );
            self.seal_candidate(&profile_name, name, &grouped.actual)?;
            self.seal_verdict(name, &grouped, pixel_threshold, &["ansi-cursor-byte-gate"])?;
            return Ok(grouped);
        };
        let expected_frame = match frame_from_ansi(approved_str, actual.provenance.clone()) {
            Ok(f) => f,
            Err(e) => {
                grouped.outcome.status = Status::CorruptApproval;
                grouped.outcome.note = format!(
                    "approved file {} is corrupt: {e}",
                    grouped.approved.ansi.display()
                );
                self.seal_candidate(&profile_name, name, &grouped.actual)?;
                self.seal_verdict(name, &grouped, pixel_threshold, &["ansi-cursor-byte-gate"])?;
                return Ok(grouped);
            }
        };

        grouped.outcome.digest_expected = Some(format!("{:016x}", expected_frame.digest()));
        compare_cells(actual, &expected_frame, &mut grouped.outcome);
        if matches!(grouped.outcome.status, Status::MissingApproval) {
            grouped.outcome.status = Status::CellsDiffer;
        }
        grouped.outcome.note = format!(
            "ansi+cursor differs: {}",
            first_difference(&approved_ansi, cheap.ansi.as_bytes())
        );

        let expected_paths = artifact_paths(&self.expected_root, name);
        let expected_artifacts = renderer
            .render_artifacts(&expected_frame, name)
            .map_err(SnapshotError::from)?;
        write_atomic(&expected_paths.png, &expected_artifacts.png)?;
        write_atomic(&expected_paths.html, expected_artifacts.html.as_bytes())?;
        grouped.outcome.expected_png = Some(expected_paths.png);
        grouped.outcome.expected_png_bytes = Some(expected_artifacts.png.clone());

        let verdict = diff::compare_png(&expected_artifacts.png, &actual_png_bytes)?;
        if verdict.dims_equal {
            grouped.outcome.pixel_score = Some(verdict.score);
            if verdict.score < 1.0 {
                let diff_path = sibling_diff(&self.diff_root, name);
                write_atomic(&diff_path, &verdict.diff_png)?;
                grouped.outcome.diff_png = Some(diff_path);
            }
        }

        self.seal_candidate(&profile_name, name, &grouped.actual)?;
        self.seal_verdict(name, &grouped, pixel_threshold, &["ansi-cursor-byte-gate"])?;
        Ok(grouped)
    }

    /// Validate, render the cheap actuals (ansi/txt/frame), and write them.
    /// A failing gate still leaves reviewable evidence.
    fn write_cheap_actuals(
        &self,
        name: &str,
        actual: &Frame,
    ) -> Result<CheapActuals, SnapshotError> {
        validate_name(name)?;
        actual.validate().map_err(SnapshotError::from)?;
        let actual_paths = artifact_paths(&self.actual_root, name);
        let approved_paths = artifact_paths(&self.approved_root, name);
        let (actual_ansi, actual_txt) = match self.artifacts {
            ArtifactSet::All => {
                let ansi = render::ansi_dump(actual);
                let txt = actual.text();
                write_atomic(&actual_paths.ansi, ansi.as_bytes())?;
                write_atomic(&actual_paths.txt, txt.as_bytes())?;
                (ansi, txt)
            }
            ArtifactSet::AnsiCursor => {
                let ansi = render::ansi_dump_with_cursor(actual);
                write_atomic(&actual_paths.ansi, ansi.as_bytes())?;
                (ansi, String::new())
            }
        };
        write_atomic(&actual_paths.frame_json, actual.to_json().as_bytes())?;
        Ok(CheapActuals {
            ansi: actual_ansi,
            txt: actual_txt,
            actual: actual_paths,
            approved: approved_paths,
        })
    }

    /// Render the expensive actuals (png/html/fidelity) fresh from the
    /// candidate frame and write them. Returns `(html_bytes, png_bytes)`.
    pub(crate) fn render_actual_artifacts(
        renderer: &mut Renderer,
        name: &str,
        actual: &Frame,
        grouped_actual: &ArtifactPaths,
    ) -> Result<(Vec<u8>, Vec<u8>), SnapshotError> {
        let artifacts = renderer
            .render_artifacts(actual, name)
            .map_err(SnapshotError::from)?;
        write_atomic(&grouped_actual.png, &artifacts.png)?;
        write_atomic(&grouped_actual.html, artifacts.html.as_bytes())?;
        write_atomic(
            &fidelity_sidecar(&grouped_actual.png),
            artifacts.fidelity.to_json().as_bytes(),
        )?;
        Ok((artifacts.html.into_bytes(), artifacts.png))
    }

    /// Missing-approval path: render fresh actuals anyway (reviewable
    /// evidence), note the gap, and seal so reports reuse this verdict.
    fn seal_missing_approval(
        &self,
        renderer: &mut Renderer,
        name: &str,
        actual: &Frame,
        grouped: &mut GroupedOutcome,
        pixel_threshold: f64,
        missing: &[&str],
    ) -> Result<(), SnapshotError> {
        Self::render_actual_artifacts(renderer, name, actual, &grouped.actual)?;
        grouped.outcome.note = format!(
            "missing approved artifact(s) for `{name}`: {}",
            missing.join(" ")
        );
        let profile_name = renderer.profile().name.clone();
        self.seal_candidate(&profile_name, name, &grouped.actual)?;
        self.seal_verdict(name, grouped, pixel_threshold, &["approved-presence"])?;
        Ok(())
    }

    /// PNG pixel gate: decoded pixels, same threshold semantics as the
    /// classic store. A corrupt approved PNG is an explicit error.
    fn run_png_gate(
        &self,
        name: &str,
        grouped: &mut GroupedOutcome,
        approved_png: &[u8],
        actual_png_bytes: &[u8],
        pixel_threshold: f64,
        notes: &mut Vec<String>,
    ) -> Result<(), SnapshotError> {
        let outcome = &mut grouped.outcome;
        let verdict = diff::compare_png(approved_png, actual_png_bytes)?;
        if verdict.dims_equal {
            outcome.pixel_score = Some(verdict.score);
            if verdict.score < pixel_threshold
                && !matches!(
                    outcome.status,
                    Status::CellsDiffer | Status::DimensionMismatch
                )
            {
                outcome.status = Status::PixelsDiffer;
            }
            if verdict.score < 1.0 {
                let path = sibling_diff(&self.diff_root, name);
                write_atomic(&path, &verdict.diff_png)?;
                outcome.diff_png = Some(path);
            }
        } else {
            outcome.status = Status::DimensionMismatch;
            notes.push(format!(
                "png dimensions differ: approved {:?}, actual {:?}",
                verdict.expected_dims, verdict.actual_dims
            ));
        }
        Ok(())
    }

    /// Flip a clean run to `Matched`, join the notes, and seal the
    /// candidate file set plus this exact verdict (C05/C08-grouped).
    fn finalize_check(
        &self,
        renderer: &Renderer,
        name: &str,
        grouped: &mut GroupedOutcome,
        pixel_threshold: f64,
        notes: &[String],
    ) -> Result<(), SnapshotError> {
        if matches!(grouped.outcome.status, Status::MissingApproval) {
            grouped.outcome.status = Status::Matched;
        }
        grouped.outcome.note = notes.join("; ");
        let profile_name = renderer.profile().name.clone();
        self.seal_candidate(&profile_name, name, &grouped.actual)?;
        self.seal_verdict(
            name,
            grouped,
            pixel_threshold,
            &[
                "ansi-byte-gate",
                "txt-byte-gate",
                "html-byte-gate",
                "png-pixel-gate",
            ],
        )
    }
}

/// Diff PNG path of one scenario under the diff root (`<name>.png`).
pub(crate) fn sibling_diff(diff_root: &Path, name: &str) -> PathBuf {
    diff_root.join(format!("{name}.png"))
}
