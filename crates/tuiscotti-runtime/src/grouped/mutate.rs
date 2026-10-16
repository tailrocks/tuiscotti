use super::{ArtifactSet, GroupedStore, artifact_paths, validate_name};
use crate::snapshot::{
    CompareOutcome, SnapshotError, StoreReport, report_entry, write_atomic, write_report_at,
};
use tuiscotti_core::frame::{Frame, Provenance};
use tuiscotti_render::formats::ansi::frame_from_ansi;
use tuiscotti_render::profile::Profile;
use tuiscotti_render::render::Renderer;

impl GroupedStore {
    /// Explicitly approve one scenario: actual artifacts replace the approved
    /// ones (atomic per file). Sidecars stay in the scratch area.
    ///
    /// In [`ArtifactSet::AnsiCursor`] mode, only `<name>.ansi` is validated via
    /// [`frame_from_ansi`] and written under `approved_root`, and any stale
    /// `<name>.{txt,png,html}` under `approved_root` are removed.
    ///
    /// # Errors
    ///
    /// Returns `SnapshotError` when actual artifacts are missing, invalid, or
    /// writes fail.
    pub fn accept(&self, name: &str) -> Result<(), SnapshotError> {
        validate_name(name)?;
        let actual = artifact_paths(&self.actual_root, name);
        let approved = artifact_paths(&self.approved_root, name);
        if self.artifacts == ArtifactSet::AnsiCursor {
            let bytes = std::fs::read(&actual.ansi).map_err(|e| {
                SnapshotError(format!(
                    "nothing to accept for `{name}` (.ansi {}: {e})",
                    actual.ansi.display()
                ))
            })?;
            let text = std::str::from_utf8(&bytes).map_err(|e| {
                SnapshotError(format!(
                    "cannot accept `{name}` (.ansi {} is not valid UTF-8: {e})",
                    actual.ansi.display()
                ))
            })?;
            let prov = Provenance::now("tuiscotti", "accept", Vec::new());
            frame_from_ansi(text, prov).map_err(|e| {
                SnapshotError(format!(
                    "cannot accept `{name}` (.ansi {} is not a valid frame: {e})",
                    actual.ansi.display()
                ))
            })?;
            write_atomic(&approved.ansi, &bytes)?;
            for stale in [&approved.txt, &approved.png, &approved.html] {
                match std::fs::remove_file(stale) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => {
                        return Err(SnapshotError(format!(
                            "cannot remove stale approved artifact {}: {e}",
                            stale.display()
                        )));
                    }
                }
            }
            return Ok(());
        }
        for (src, dst, label) in [
            (&actual.ansi, &approved.ansi, ".ansi"),
            (&actual.txt, &approved.txt, ".txt"),
            (&actual.png, &approved.png, ".png"),
            (&actual.html, &approved.html, ".html"),
        ] {
            let bytes = std::fs::read(src).map_err(|e| {
                SnapshotError(format!(
                    "nothing to accept for `{name}` ({label} {}: {e})",
                    src.display()
                ))
            })?;
            write_atomic(dst, &bytes)?;
        }
        Ok(())
    }

    /// Accept every scenario with actual artifacts, recursively. Returns the
    /// accepted names (sorted).
    ///
    /// # Errors
    ///
    /// Returns `SnapshotError` when listing actuals or any accept fails.
    pub fn accept_all(&self) -> Result<Vec<String>, SnapshotError> {
        let names = self.actual_names()?;
        for name in &names {
            self.accept(name)?;
        }
        Ok(names)
    }

    /// Accept every scenario in [`Self::actual_names`] matching `filter` (exact
    /// match, prefix match `filter` / `filter/...`, or `*` / `**` / `?` glob
    /// pattern). Fails loudly when zero scenarios match `filter`.
    ///
    /// # Errors
    ///
    /// Returns `SnapshotError` when listing actuals fails, no scenario matches
    /// `filter`, or any accept fails.
    pub fn accept_matching(&self, filter: &str) -> Result<Vec<String>, SnapshotError> {
        let filter_norm = filter.replace("::", "/");
        let names = self.actual_names()?;
        let matched: Vec<String> = names
            .into_iter()
            .filter(|n| name_matches_filter(n, &filter_norm))
            .collect();
        if matched.is_empty() {
            return Err(SnapshotError(format!(
                "nothing to accept matching `{filter}` in {}",
                self.actual_root.display()
            )));
        }
        for name in &matched {
            self.accept(name)?;
        }
        Ok(matched)
    }

    /// Rewrite the HTML review index from on-disk actual vs approved
    /// artifacts. Does not re-render. Unmatched gates do not error here:
    /// inspect [`StoreReport::failed`] and the outcomes.
    ///
    /// # Errors
    ///
    /// Returns `SnapshotError` when rendering or writing the report fails.
    pub fn report(
        &self,
        profile: &Profile,
        faces: &tuiscotti_render::profile::FontFaces<'_>,
        pixel_threshold: f64,
        title: &str,
    ) -> Result<StoreReport, SnapshotError> {
        Renderer::with_profile(profile, faces, |r| {
            self.report_with(r, pixel_threshold, title)
        })
    }

    /// Review index over the ONE persisted verdict per scenario (C05):
    /// a verdict is reused only when its artifact hashes still match the
    /// files on disk and the pixel threshold matches; anything stale is
    /// recomputed via `check`, never silently reused — so report status
    /// equals check status on the same inputs. Candidates with a missing
    /// manifest member report [`crate::snapshot::Status::MissingApproval`] (C08-grouped),
    /// never a pixel verdict. HTML links the PNG files; it does not embed
    /// them.
    ///
    /// # Errors
    ///
    /// Returns `SnapshotError` when rendering or writing the report fails.
    pub fn report_with(
        &self,
        renderer: &mut Renderer,
        pixel_threshold: f64,
        title: &str,
    ) -> Result<StoreReport, SnapshotError> {
        let names = self.actual_names()?;
        let mut entries = Vec::new();
        let mut outcomes = Vec::new();
        let profile = renderer.profile().clone();
        for name in names {
            let outcome = self.report_outcome(renderer, &name, pixel_threshold)?;
            entries.push(report_entry(&outcome, &profile)?);
            outcomes.push(outcome);
        }
        let path = write_report_at(&self.report_path(), title, &entries)?;
        Ok(StoreReport { path, outcomes })
    }

    /// One report row: incomplete evidence first, then the persisted verdict
    /// when fresh, else a genuine recompute through `check_with`.
    fn report_outcome(
        &self,
        renderer: &mut Renderer,
        name: &str,
        pixel_threshold: f64,
    ) -> Result<CompareOutcome, SnapshotError> {
        if let Some(problem) = self.candidate_gap(name)? {
            return Ok(self.incomplete_outcome(name, &problem));
        }
        if let Some(outcome) = self.fresh_verdict(name, pixel_threshold)? {
            return Ok(outcome);
        }
        let frame_path = artifact_paths(&self.actual_root, name).frame_json;
        let text = std::fs::read_to_string(&frame_path)
            .map_err(|e| SnapshotError(format!("cannot read {}: {e}", frame_path.display())))?;
        let frame = Frame::from_json(&text)?;
        let grouped = self.check_with(renderer, name, &frame, pixel_threshold)?;
        Ok(grouped.outcome)
    }
}

/// Match a scenario name against a filter: exact, prefix, or `*`/`**`/`?` glob.
#[must_use]
pub fn name_matches_filter(name: &str, filter: &str) -> bool {
    if filter.is_empty() {
        return false;
    }
    if name == filter {
        return true;
    }
    let prefix = filter.strip_suffix('/').unwrap_or(filter);
    if !filter.contains(['*', '?']) {
        if name.starts_with(&format!("{prefix}/")) || name.starts_with(filter) {
            return true;
        }
        return false;
    }
    glob_match(filter, name)
}

fn glob_match(pat: &str, text: &str) -> bool {
    glob_bytes(pat.as_bytes(), text.as_bytes())
}

fn glob_bytes(pat: &[u8], text: &[u8]) -> bool {
    let Some(&p) = pat.first() else {
        return text.is_empty();
    };
    if p == b'*' {
        let mut rest = &pat[1..];
        while rest.first() == Some(&b'*') {
            rest = &rest[1..];
        }
        if glob_bytes(rest, text) {
            return true;
        }
        for i in 0..text.len() {
            if glob_bytes(pat, &text[i + 1..]) {
                return true;
            }
        }
        return false;
    }
    let Some(&t) = text.first() else {
        return false;
    };
    if p == b'?' {
        return t != b'/' && glob_bytes(&pat[1..], &text[1..]);
    }
    p == t && glob_bytes(&pat[1..], &text[1..])
}
