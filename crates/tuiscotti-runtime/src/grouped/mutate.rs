use super::{ArtifactSet, GroupedStore, artifact_paths, validate_name};
use crate::snapshot::{
    CompareOutcome, SnapshotError, Status, StoreReport, report_entry, write_atomic, write_report_at,
};
use tuiscotti_core::frame::{Frame, Provenance};
use tuiscotti_render::formats::ansi::frame_from_ansi;
use tuiscotti_render::profile::Profile;
use tuiscotti_render::render::Renderer;

/// True when `CI` is set to a non-empty value other than `"0"` or `"false"`
/// (case-insensitive). Snapshot acceptance is a local-only workflow and is
/// refused under CI.
#[must_use]
pub fn is_ci_env() -> bool {
    match std::env::var("CI").ok().as_deref() {
        None | Some("") => false,
        Some(v) if v.eq_ignore_ascii_case("false") || v == "0" => false,
        Some(_) => true,
    }
}

impl GroupedStore {
    /// Explicitly approve one scenario: actual artifacts replace the approved
    /// ones (atomic per file). Sidecars stay in the scratch area.
    ///
    /// Requires a sealed candidate (`<name>.manifest.json` present and matching
    /// the on-disk actual artifacts). In [`ArtifactSet::AnsiCursor`] mode,
    /// `<name>.ansi` is also validated via [`frame_from_ansi`] plus lossless
    /// [`tuiscotti_render::render::ansi_dump_with_cursor`] round-trip, and any
    /// stale `<name>.{txt,png,html}` under `approved_root` are removed.
    ///
    /// # Errors
    ///
    /// Returns `SnapshotError` when `CI` is set, actual artifacts are missing,
    /// unsealed, invalid, or writes fail.
    pub fn accept(&self, name: &str) -> Result<(), SnapshotError> {
        validate_name(name)?;
        if is_ci_env() {
            return Err(SnapshotError(
                "refusing `accept` because CI is set: approving snapshots is local only"
                    .to_string(),
            ));
        }
        let actual = artifact_paths(&self.actual_root, name);
        let approved = artifact_paths(&self.approved_root, name);
        if !actual.ansi.exists() {
            return Err(SnapshotError(format!(
                "nothing to accept for `{name}` (.ansi {} does not exist)",
                actual.ansi.display()
            )));
        }
        self.inspect_candidate_for_accept(name)?;
        if self.artifacts == ArtifactSet::AnsiCursor {
            return Self::accept_ansi_cursor(name, &actual.ansi, &approved);
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

    fn accept_ansi_cursor(
        name: &str,
        actual_ansi: &std::path::Path,
        approved: &super::ArtifactPaths,
    ) -> Result<(), SnapshotError> {
        let bytes = std::fs::read(actual_ansi).map_err(|e| {
            SnapshotError(format!(
                "nothing to accept for `{name}` (.ansi {}: {e})",
                actual_ansi.display()
            ))
        })?;
        let text = std::str::from_utf8(&bytes).map_err(|e| {
            SnapshotError(format!(
                "cannot accept `{name}` (.ansi {} is not valid UTF-8: {e})",
                actual_ansi.display()
            ))
        })?;
        let prov = Provenance::now("tuiscotti", "accept", Vec::new());
        let decoded = frame_from_ansi(text, prov).map_err(|e| {
            SnapshotError(format!(
                "cannot accept `{name}` (.ansi {} is not a valid frame: {e})",
                actual_ansi.display()
            ))
        })?;
        if tuiscotti_render::render::ansi_dump_with_cursor(&decoded) != text {
            return Err(SnapshotError(format!(
                "cannot accept `{name}` (.ansi {} does not round-trip losslessly)",
                actual_ansi.display()
            )));
        }
        write_atomic(&approved.ansi, &bytes)?;
        let fid = approved.png.with_extension("png.fidelity.json");
        for stale in [&approved.txt, &approved.png, &approved.html, &fid] {
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
        Ok(())
    }

    /// Accept every scenario with actual artifacts via [`Self::accept_matching`]`("**")`
    /// (enforcing `CI` refusal, candidate seal/verdict checks, `run_id` freshness,
    /// and skipping already-[`Status::Matched`] scenarios).
    ///
    /// # Errors
    ///
    /// Returns `SnapshotError` when `CI` is set, listing actuals fails, any
    /// candidate is unsealed/unverified/stale, or any accept fails.
    pub fn accept_all(&self) -> Result<Vec<String>, SnapshotError> {
        if is_ci_env() {
            return Err(SnapshotError(
                "refusing `accept_all` because CI is set: approving snapshots is local only"
                    .to_string(),
            ));
        }
        if self.actual_names()?.is_empty() {
            return Ok(Vec::new());
        }
        self.accept_matching("**")
    }

    /// Accept every scenario in [`Self::actual_names`] matching `filter` (exact
    /// match, segment-prefix match `filter` / `filter/...`, or `*` / `**` / `?`
    /// glob pattern).
    ///
    /// Refuses under `CI`, requires every matched candidate to be sealed with a
    /// matching verdict, enforces a `run_id` freshness guard across multi-scenario
    /// selections (or against [`Self::run_id`] when set), and skips candidates
    /// whose fresh verdict is already [`Status::Matched`].
    ///
    /// # Errors
    ///
    /// Returns `SnapshotError` when `CI` is set, no scenario matches `filter`,
    /// any matched candidate is unsealed/unverified/stale, or any write fails.
    pub fn accept_matching(&self, filter: &str) -> Result<Vec<String>, SnapshotError> {
        if is_ci_env() {
            return Err(SnapshotError(
                "refusing `accept_matching` because CI is set: approving snapshots is local only"
                    .to_string(),
            ));
        }
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
        let mut metas = Vec::with_capacity(matched.len());
        for name in &matched {
            metas.push(self.inspect_candidate_for_accept(name)?);
        }
        let trimmed = filter_norm.strip_suffix('/').unwrap_or(&filter_norm);
        let is_exact_single =
            !trimmed.contains(['*', '?']) && matched.len() == 1 && matched[0] == trimmed;
        if let Some(expected_run) = self.run_id.as_deref() {
            let stale: Vec<&str> = metas
                .iter()
                .filter(|m| m.run_id != expected_run)
                .map(|m| m.name.as_str())
                .collect();
            if !stale.is_empty() {
                return Err(SnapshotError(format!(
                    "refusing to accept stale candidate(s) from an older run (expected run_id \
                     `{expected_run}`): {}",
                    stale.join(", ")
                )));
            }
        } else if !is_exact_single {
            let newest_run_id = metas
                .iter()
                .max_by_key(|m| m.written_unix_ns)
                .map(|m| m.run_id.as_str())
                .unwrap_or_default();
            let stale: Vec<&str> = metas
                .iter()
                .filter(|m| m.run_id != newest_run_id)
                .map(|m| m.name.as_str())
                .collect();
            if !stale.is_empty() {
                return Err(SnapshotError(format!(
                    "refusing to accept stale candidate(s) from an older run (newest run_id \
                     `{newest_run_id}`): {}",
                    stale.join(", ")
                )));
            }
        }
        let mut accepted = Vec::new();
        for meta in metas {
            if meta.status == Status::Matched
                && self
                    .fresh_verdict(&meta.name, meta.pixel_threshold)?
                    .is_some_and(|v| v.status == Status::Matched)
            {
                continue;
            }
            self.accept(&meta.name)?;
            accepted.push(meta.name);
        }
        Ok(accepted)
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

/// Match a scenario name against a filter: exact, whole-segment prefix, or
/// `*` (single-segment) / `**` (multi-segment) / `?` glob.
#[must_use]
pub fn name_matches_filter(name: &str, filter: &str) -> bool {
    let norm = filter.replace("::", "/");
    let prefix = norm.strip_suffix('/').unwrap_or(&norm);
    if prefix.is_empty() {
        return false;
    }
    if !prefix.contains(['*', '?']) {
        return name == prefix || name.starts_with(&format!("{prefix}/"));
    }
    glob_match(prefix, name)
}

fn glob_match(pat: &str, text: &str) -> bool {
    glob_bytes(pat.as_bytes(), text.as_bytes())
}

fn glob_bytes(pat: &[u8], text: &[u8]) -> bool {
    let Some(&p) = pat.first() else {
        return text.is_empty();
    };
    if pat.starts_with(b"**") {
        let mut rest = &pat[2..];
        while rest.first() == Some(&b'*') {
            rest = &rest[1..];
        }
        if rest.is_empty() {
            return true;
        }
        if rest.first() == Some(&b'/') {
            if glob_bytes(&rest[1..], text) {
                return true;
            }
            return (0..text.len()).any(|i| glob_bytes(rest, &text[i..]));
        }
        return (0..=text.len()).any(|i| glob_bytes(rest, &text[i..]));
    }
    if p == b'*' {
        if glob_bytes(&pat[1..], text) {
            return true;
        }
        return text.first().is_some_and(|&b| b != b'/') && glob_bytes(pat, &text[1..]);
    }
    let Some(&t) = text.first() else {
        return false;
    };
    if p == b'?' {
        return t != b'/' && glob_bytes(&pat[1..], &text[1..]);
    }
    p == t && glob_bytes(&pat[1..], &text[1..])
}

#[cfg(test)]
mod tests {
    use super::name_matches_filter;

    #[test]
    fn filter_matches_whole_segments_and_distinguishes_single_vs_double_star() {
        for (name, pat, expected) in [
            ("showcase/pages/tab", "showcase/pages/tab", true),
            ("showcase/pages/tab/80x24", "showcase/pages/tab", true),
            (
                "showcase/pages/tables/80x24/truecolor",
                "showcase/pages/tab",
                false,
            ),
            ("pages/lists", "pages/*", true),
            ("pages/lists/80x24", "pages/*", false),
            ("pages/lists", "pages/**", true),
            ("pages/lists/80x24/truecolor", "pages/**", true),
            ("showcase/truecolor", "showcase/**/truecolor", true),
            ("showcase/pages/truecolor", "showcase/**/truecolor", true),
            (
                "showcase/pages/tables/80x24/truecolor",
                "showcase/**/truecolor",
                true,
            ),
            ("showcase/other_truecolor", "showcase/**/truecolor", false),
        ] {
            assert_eq!(name_matches_filter(name, pat), expected, "{name} vs {pat}");
        }
    }
}
