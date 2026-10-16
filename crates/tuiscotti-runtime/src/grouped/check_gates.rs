use super::{ArtifactPaths, GroupedOutcome};
use crate::snapshot::{CompareOutcome, Status};
use tuiscotti_core::frame::Frame;

/// Locate the first differing byte of two blobs, for human diagnostics.
pub(crate) fn first_difference(approved: &[u8], actual: &[u8]) -> String {
    let n = approved.len().min(actual.len());
    let mut i = 0;
    while i < n && approved[i] == actual[i] {
        i += 1;
    }
    let mut line = 1;
    for &b in &approved[..i] {
        if b == b'\n' {
            line += 1;
        }
    }
    format!(
        "first difference at byte {i} (approved line {line}); approved {} bytes, actual {} bytes",
        approved.len(),
        actual.len()
    )
}

/// Validated cheap actuals plus both artifact path sets.
pub(crate) struct CheapActuals {
    pub(crate) ansi: String,
    pub(crate) txt: String,
    pub(crate) actual: ArtifactPaths,
    pub(crate) approved: ArtifactPaths,
}

/// Fresh outcome skeleton: `MissingApproval` until a gate says otherwise.
pub(crate) fn fresh_grouped(name: &str, cheap: &CheapActuals, actual: &Frame) -> GroupedOutcome {
    let outcome = CompareOutcome {
        name: name.to_string(),
        status: Status::MissingApproval,
        cell_diffs: Vec::new(),
        cell_diff_total: 0,
        pixel_score: None,
        approved_png_regenerated: false,
        digest_expected: None,
        digest_actual: format!("{:016x}", actual.digest()),
        actual_frame: cheap.actual.frame_json.clone(),
        actual_png: cheap.actual.png.clone(),
        // Never exists in a conforming approved tree (four artifacts
        // only); reports simply omit the expected-frame panel.
        expected_frame: cheap.approved.frame_json.clone(),
        expected_png: None,
        expected_png_bytes: None,
        diff_png: None,
        note: String::new(),
    };
    GroupedOutcome {
        outcome,
        ansi_match: None,
        txt_match: None,
        html_match: None,
        actual: cheap.actual.clone(),
        approved: cheap.approved.clone(),
    }
}

/// Which approved artifacts are absent (empty = all present). `approved`
/// holds `.ansi`/`.txt`/`.html`/`.png` in that order (`None` = missing).
pub(crate) fn missing_approved_names(approved: [Option<&[u8]>; 4]) -> Vec<&'static str> {
    const LABELS: [&str; 4] = [".ansi", ".txt", ".html", ".png"];
    approved
        .iter()
        .zip(LABELS)
        .filter_map(|(slot, label)| slot.is_none().then_some(label))
        .collect()
}

/// ANSI + TXT byte gates: cell-exact, then content-only.
pub(crate) fn run_byte_gates(
    grouped: &mut GroupedOutcome,
    approved_ansi: &[u8],
    actual_ansi: &str,
    approved_txt: &[u8],
    actual_txt: &str,
    notes: &mut Vec<String>,
) {
    let outcome = &mut grouped.outcome;
    // ANSI byte gate: the cell-exact comparison (symbol+fg+bg+mods).
    let ansi_equal = approved_ansi == actual_ansi.as_bytes();
    grouped.ansi_match = Some(ansi_equal);
    if !ansi_equal {
        outcome.status = Status::CellsDiffer;
        notes.push(format!(
            "ansi differs (cell-exact gate): {}",
            first_difference(approved_ansi, actual_ansi.as_bytes())
        ));
    }

    // TXT byte gate: content only (style-only changes keep txt equal).
    let txt_equal = approved_txt == actual_txt.as_bytes();
    grouped.txt_match = Some(txt_equal);
    if !txt_equal {
        if matches!(outcome.status, Status::MissingApproval) {
            outcome.status = Status::CellsDiffer;
        }
        notes.push(format!(
            "txt differs: {}",
            first_difference(approved_txt, actual_txt.as_bytes())
        ));
    }
}

/// HTML byte gate: identical cells with a changed renderer/font fail
/// here — a render-level event, reported as `PixelsDiffer`.
pub(crate) fn run_html_gate(
    grouped: &mut GroupedOutcome,
    approved_html: &[u8],
    actual_html_bytes: &[u8],
    notes: &mut Vec<String>,
) {
    let html_equal = approved_html == actual_html_bytes;
    grouped.html_match = Some(html_equal);
    if !html_equal {
        if matches!(grouped.outcome.status, Status::MissingApproval) {
            grouped.outcome.status = Status::PixelsDiffer;
        }
        notes.push(format!(
            "html differs (render-level gate): {}",
            first_difference(approved_html, actual_html_bytes)
        ));
    }
}

/// Candidate seal: `<name>.manifest.json` under the actual root.
pub(crate) fn manifest_path(actual_root: &std::path::Path, name: &str) -> std::path::PathBuf {
    actual_root.join(format!("{name}.manifest.json"))
}

/// Persisted check verdict: `<name>.verdict.json` under the actual root.
pub(crate) fn verdict_path(actual_root: &std::path::Path, name: &str) -> std::path::PathBuf {
    actual_root.join(format!("{name}.verdict.json"))
}

/// Inverse of [`Status::as_str`] for persisted verdicts.
pub(crate) fn status_from_str(s: &str) -> Option<Status> {
    Some(match s {
        "matched" => Status::Matched,
        "cells-differ" => Status::CellsDiffer,
        "pixels-differ" => Status::PixelsDiffer,
        "dimension-mismatch" => Status::DimensionMismatch,
        "missing-approval" => Status::MissingApproval,
        "corrupt-approval" => Status::CorruptApproval,
        "capture-incomplete" => Status::CaptureIncomplete,
        "not-checked" => Status::NotChecked,
        _ => return None,
    })
}

/// Remove stale mismatch/missing-approval evidence files for `name` at the
/// start of an `AnsiCursor` check so a subsequent pass never leaves stale
/// `.png`/`.html`/diff artifacts behind.
pub(crate) fn clean_stale_ansi_cursor_evidence(
    actual: &ArtifactPaths,
    expected_root: &std::path::Path,
    diff_root: &std::path::Path,
    name: &str,
) -> Result<(), crate::snapshot::SnapshotError> {
    let expected = super::artifact_paths(expected_root, name);
    let diff = super::sibling_diff(diff_root, name);
    let act_fid = actual.png.with_extension("png.fidelity.json");
    let exp_fid = expected.png.with_extension("png.fidelity.json");
    for stale in [
        &actual.png,
        &actual.html,
        &actual.txt,
        &act_fid,
        &expected.png,
        &expected.html,
        &exp_fid,
        &diff,
    ] {
        match std::fs::remove_file(stale) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(crate::snapshot::SnapshotError(format!(
                    "cannot remove stale evidence {}: {e}",
                    stale.display()
                )));
            }
        }
    }
    Ok(())
}

/// Decode the approved `.ansi` baseline and populate cell diff + mismatch note
/// for an `AnsiCursor` failure, or record `CorruptApproval` and return `None`.
pub(crate) fn populate_ansi_cursor_mismatch(
    grouped: &mut GroupedOutcome,
    actual: &Frame,
    approved_ansi: &[u8],
    actual_ansi: &str,
) -> Option<Frame> {
    let Ok(approved_str) = std::str::from_utf8(approved_ansi) else {
        grouped.outcome.status = Status::CorruptApproval;
        grouped.outcome.note = format!(
            "approved file {} is not valid UTF-8",
            grouped.approved.ansi.display()
        );
        return None;
    };
    let expected_frame = match tuiscotti_render::formats::ansi::frame_from_ansi(
        approved_str,
        actual.provenance.clone(),
    ) {
        Ok(f) => f,
        Err(e) => {
            grouped.outcome.status = Status::CorruptApproval;
            grouped.outcome.note = format!(
                "approved file {} is corrupt: {e}",
                grouped.approved.ansi.display()
            );
            return None;
        }
    };
    grouped.outcome.digest_expected = Some(format!("{:016x}", expected_frame.digest()));
    let actual_observable =
        tuiscotti_render::formats::ansi::frame_from_ansi(actual_ansi, actual.provenance.clone())
            .unwrap_or_else(|_| actual.clone());
    crate::snapshot::compare_cells(&actual_observable, &expected_frame, &mut grouped.outcome);
    if matches!(grouped.outcome.status, Status::MissingApproval) {
        grouped.outcome.status = Status::CellsDiffer;
    }
    let legacy_note = if !approved_str.contains("\x1b[?25")
        && format!("{approved_str}\x1b[?25l") == actual_ansi
    {
        " (approved file predates the cursor trailer; re-accept)"
    } else {
        ""
    };
    grouped.outcome.note = format!(
        "ansi+cursor differs: {}{legacy_note}",
        first_difference(approved_ansi, actual_ansi.as_bytes())
    );
    Some(expected_frame)
}

/// Validated metadata for a candidate in [`super::GroupedStore::accept_matching`].
pub(crate) struct CandidateAcceptMeta {
    pub(crate) name: String,
    pub(crate) run_id: String,
    pub(crate) written_unix_ns: u64,
    pub(crate) status: Status,
    pub(crate) pixel_threshold: f64,
}

impl super::GroupedStore {
    pub(crate) fn inspect_candidate_for_accept(
        &self,
        name: &str,
    ) -> Result<CandidateAcceptMeta, crate::snapshot::SnapshotError> {
        use crate::snapshot::SnapshotError;
        if let Some(gap) = self.candidate_gap(name)? {
            return Err(SnapshotError(format!("cannot accept `{name}`: {gap}")));
        }
        let m_path = super::manifest_path(&self.actual_root, name);
        let m_text = std::fs::read_to_string(&m_path)
            .map_err(|e| SnapshotError(format!("cannot read {}: {e}", m_path.display())))?;
        let manifest: serde_json::Value = serde_json::from_str(&m_text)
            .map_err(|e| SnapshotError(format!("cannot parse {}: {e}", m_path.display())))?;
        let Some(run_id) = manifest
            .get("run_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
        else {
            return Err(SnapshotError(format!(
                "cannot accept `{name}`: {} is missing `run_id`",
                m_path.display()
            )));
        };
        let written_unix_ns = manifest
            .get("written_unix_ns")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let (status, pixel_threshold) = self.verify_candidate_seals(name, &manifest)?;
        Ok(CandidateAcceptMeta {
            name: name.to_string(),
            run_id,
            written_unix_ns,
            status,
            pixel_threshold,
        })
    }

    fn verify_candidate_seals(
        &self,
        name: &str,
        manifest: &serde_json::Value,
    ) -> Result<(Status, f64), crate::snapshot::SnapshotError> {
        use crate::snapshot::SnapshotError;
        let actual = super::artifact_paths(&self.actual_root, name);
        let required: &[(&std::path::PathBuf, &str, &str)] = match self.artifacts {
            super::ArtifactSet::All => &[
                (&actual.ansi, ".ansi", "ansi_sha256"),
                (&actual.txt, ".txt", "txt_sha256"),
                (&actual.png, ".png", "png_sha256"),
                (&actual.html, ".html", "html_sha256"),
                (&actual.frame_json, ".frame.json", "frame_sha256"),
            ],
            super::ArtifactSet::AnsiCursor => &[
                (&actual.ansi, ".ansi", "ansi_sha256"),
                (&actual.frame_json, ".frame.json", "frame_sha256"),
            ],
        };
        for (path, label, key) in required {
            let bytes = std::fs::read(path)
                .map_err(|e| SnapshotError(format!("cannot read {}: {e}", path.display())))?;
            let current_sha = super::seal::sha256_hex(&bytes);
            if manifest.get(*key).and_then(|v| v.as_str()) != Some(current_sha.as_str()) {
                return Err(SnapshotError(format!(
                    "cannot accept `{name}`: {label} ({name}{label}) hash disagrees with \
                     {name}.manifest.json"
                )));
            }
        }
        let v_path = super::verdict_path(&self.actual_root, name);
        let v_text = std::fs::read_to_string(&v_path).map_err(|_| {
            SnapshotError(format!(
                "cannot accept `{name}`: {name}.verdict.json missing (unverified candidate)"
            ))
        })?;
        let verdict: serde_json::Value = serde_json::from_str(&v_text).map_err(|e| {
            SnapshotError(format!(
                "cannot accept `{name}`: {name}.verdict.json unparsable: {e}"
            ))
        })?;
        if verdict.get("name").and_then(|v| v.as_str()) != Some(name) {
            return Err(SnapshotError(format!(
                "cannot accept `{name}`: {name}.verdict.json names a different snapshot"
            )));
        }
        let Some(status) = verdict
            .get("status")
            .and_then(|v| v.as_str())
            .and_then(status_from_str)
        else {
            return Err(SnapshotError(format!(
                "cannot accept `{name}`: {name}.verdict.json has invalid status"
            )));
        };
        for (_path, _label, key) in required {
            let m_sha = manifest.get(*key).and_then(|v| v.as_str());
            let v_sha = verdict
                .get("actual")
                .and_then(|a| a.get(*key))
                .and_then(|v| v.as_str());
            if v_sha.is_none() || v_sha != m_sha {
                return Err(SnapshotError(format!(
                    "cannot accept `{name}`: {name}.verdict.json does not match {name}.manifest.json"
                )));
            }
        }
        let pixel_threshold = verdict
            .get("pixel_threshold")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(1.0);
        Ok((status, pixel_threshold))
    }
}
