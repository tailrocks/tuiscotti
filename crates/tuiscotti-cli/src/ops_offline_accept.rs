//! Offline `accept` plumbing: classic, frozen, and grouped (both
//! `ArtifactSet::All` and `ArtifactSet::AnsiCursor`) store acceptance.

use std::path::Path;

use tuiscotti::proto::EXIT_OP_ERROR;

use crate::cli::ArtifactSetArg;

pub(crate) fn cmd_accept(
    store: &Path,
    name: &str,
    actual_override: Option<&Path>,
    artifacts_arg: Option<ArtifactSetArg>,
) -> i32 {
    if tuiscotti::grouped::is_ci_env() {
        eprintln!(
            "error: refusing to accept snapshots because CI is set: approving snapshots is local only"
        );
        return EXIT_OP_ERROR;
    }
    // Frozen roots (`Policy::Frozen` layout: `<name>.canonical.txt` approvals
    // directly in the root) reject acceptance unconditionally — route through
    // `frozen_accept` so the refusal stays in one place. Checked before
    // `Store::accept` so a planted `actual/` tree inside a frozen root can
    // never bless into it.
    if is_frozen_root(store) {
        return match tuiscotti::assert::frozen_accept(store, name) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("error: {e}");
                EXIT_OP_ERROR
            }
        };
    }
    if actual_override.is_none() && artifacts_arg.is_none() && is_classic_store(store) {
        return match tuiscotti::snapshot::Store::new(store).accept(name) {
            Ok(()) => {
                let buf = format!("accepted `{name}` in {}\n", store.display());
                crate::write_stdout(&buf)
            }
            Err(e) => {
                eprintln!("error: {e}");
                EXIT_OP_ERROR
            }
        };
    }
    accept_grouped(store, name, actual_override, artifacts_arg)
}

fn is_classic_store(store: &Path) -> bool {
    store.join("approved").is_dir() || store.join("actual").is_dir()
}

fn accept_grouped(
    store: &Path,
    name: &str,
    actual_override: Option<&Path>,
    artifacts_arg: Option<ArtifactSetArg>,
) -> i32 {
    let mut grouped = tuiscotti::grouped::GroupedStore::new(store);
    if let Some(act) = actual_override {
        grouped = grouped.with_actual_root(act);
    }
    let filter_norm = name.replace("::", "/");
    let is_pattern = filter_norm.contains(['*', '?']);
    if !is_pattern {
        let trimmed = filter_norm.strip_suffix('/').unwrap_or(&filter_norm);
        if let Err(e) = tuiscotti::grouped::validate_name(trimmed) {
            eprintln!("error: {e}");
            return EXIT_OP_ERROR;
        }
    }
    let set = match artifacts_arg {
        Some(ArtifactSetArg::All) => tuiscotti::grouped::ArtifactSet::All,
        Some(ArtifactSetArg::AnsiCursor) => tuiscotti::grouped::ArtifactSet::AnsiCursor,
        None => match detect_artifact_set(&grouped, &filter_norm) {
            Ok(s) => s,
            Err(msg) => {
                eprintln!("error: {msg}");
                return EXIT_OP_ERROR;
            }
        },
    };
    grouped = grouped.with_artifacts(set);

    let names = match grouped.accept_matching(name) {
        Ok(list) => list,
        Err(e) => {
            eprintln!("error: {e}");
            return EXIT_OP_ERROR;
        }
    };
    let mut buf = String::new();
    for n in &names {
        crate::push_line(&mut buf, &format!("accepted `{n}` in {}", store.display()));
    }
    crate::write_stdout(&buf)
}

fn detect_artifact_set(
    grouped: &tuiscotti::grouped::GroupedStore,
    filter_norm: &str,
) -> Result<tuiscotti::grouped::ArtifactSet, String> {
    use tuiscotti::grouped::ArtifactSet;
    let check_actual = |n: &str| -> Option<ArtifactSet> {
        let manifest_path = grouped.actual_root().join(format!("{n}.manifest.json"));
        if let Ok(text) = std::fs::read_to_string(&manifest_path)
            && let Ok(v) = serde_json::from_str::<serde_json::Value>(&text)
        {
            return Some(if v.get("txt_sha256").is_some() {
                ArtifactSet::All
            } else {
                ArtifactSet::AnsiCursor
            });
        }
        let act_ansi = grouped.actual_root().join(format!("{n}.ansi"));
        let act_txt = grouped.actual_root().join(format!("{n}.txt"));
        if act_ansi.is_file() {
            return Some(if act_txt.is_file() {
                ArtifactSet::All
            } else {
                ArtifactSet::AnsiCursor
            });
        }
        None
    };
    let check_approved = |n: &str| -> Option<ArtifactSet> {
        let app_ansi = grouped.approved_root().join(format!("{n}.ansi"));
        let app_png = grouped.approved_root().join(format!("{n}.png"));
        let app_txt = grouped.approved_root().join(format!("{n}.txt"));
        if app_ansi.is_file() {
            return Some(if app_png.is_file() || app_txt.is_file() {
                ArtifactSet::All
            } else {
                ArtifactSet::AnsiCursor
            });
        }
        None
    };

    let matched: Vec<String> = grouped
        .actual_names()
        .unwrap_or_default()
        .into_iter()
        .filter(|n| tuiscotti::grouped::name_matches_filter(n, filter_norm))
        .collect();

    let mut detected: Option<ArtifactSet> = None;
    let mut record = |s: ArtifactSet| -> Result<(), String> {
        if let Some(prev) = detected
            && prev != s
        {
            return Err(
                "matched candidates mix artifact sets; pass --artifacts all or --artifacts ansi-cursor explicitly"
                    .to_string(),
            );
        }
        detected = Some(s);
        Ok(())
    };

    for n in &matched {
        if let Some(s) = check_actual(n) {
            record(s)?;
        }
        if let Some(s) = check_approved(n) {
            record(s)?;
        }
    }
    Ok(detected.unwrap_or(ArtifactSet::All))
}

/// A frozen root holds `<name>.canonical.txt` approvals directly in the root
/// (see `Policy::Frozen`); a classic store holds `approved/`/`actual/`/`diff/`
/// subdirs instead, so the marker never collides.
fn is_frozen_root(store: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(store) else {
        return false;
    };
    entries.flatten().any(|e| {
        e.file_name()
            .to_str()
            .is_some_and(|n| n.ends_with(".canonical.txt"))
    })
}
