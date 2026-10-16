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
    } else if !grouped.actual_root().exists() {
        let target_actual = store
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("target/tuiscotti/actual");
        if target_actual.is_dir() {
            grouped = grouped.with_actual_root(&target_actual);
        }
    }
    let set = match artifacts_arg {
        Some(ArtifactSetArg::All) => tuiscotti::grouped::ArtifactSet::All,
        Some(ArtifactSetArg::AnsiCursor) => tuiscotti::grouped::ArtifactSet::AnsiCursor,
        None => detect_artifact_set(&grouped, name),
    };
    grouped = grouped.with_artifacts(set);

    let is_pattern = name.contains(['*', '?']);
    if !is_pattern && let Err(e) = tuiscotti::grouped::validate_name(name) {
        eprintln!("error: {e}");
        return EXIT_OP_ERROR;
    }
    let exact_ansi = grouped.actual_root().join(format!("{name}.ansi"));
    let names = if !is_pattern && exact_ansi.is_file() {
        match grouped.accept(name) {
            Ok(()) => vec![name.to_string()],
            Err(e) => {
                eprintln!("error: {e}");
                return EXIT_OP_ERROR;
            }
        }
    } else {
        match grouped.accept_matching(name) {
            Ok(list) => list,
            Err(e) => {
                eprintln!("error: {e}");
                return EXIT_OP_ERROR;
            }
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
    name: &str,
) -> tuiscotti::grouped::ArtifactSet {
    let check_name = |n: &str| -> Option<tuiscotti::grouped::ArtifactSet> {
        let act_ansi = grouped.actual_root().join(format!("{n}.ansi"));
        let act_txt = grouped.actual_root().join(format!("{n}.txt"));
        if act_ansi.is_file() {
            return Some(if act_txt.is_file() {
                tuiscotti::grouped::ArtifactSet::All
            } else {
                tuiscotti::grouped::ArtifactSet::AnsiCursor
            });
        }
        let app_ansi = grouped.approved_root().join(format!("{n}.ansi"));
        let app_png = grouped.approved_root().join(format!("{n}.png"));
        if app_ansi.is_file() {
            return Some(if app_png.is_file() {
                tuiscotti::grouped::ArtifactSet::All
            } else {
                tuiscotti::grouped::ArtifactSet::AnsiCursor
            });
        }
        None
    };
    if let Some(s) = check_name(name) {
        return s;
    }
    if let Ok(actuals) = grouped.actual_names()
        && let Some(first) = actuals.first()
        && let Some(s) = check_name(first)
    {
        return s;
    }
    tuiscotti::grouped::ArtifactSet::All
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
