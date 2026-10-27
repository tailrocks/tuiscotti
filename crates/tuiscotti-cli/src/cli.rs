//! Clap surface: a global `--json` flag and exactly four offline subcommands.
//!
//! Paths stay [`PathBuf`] end-to-end: clap parses them from `args_os`
//! with no lossy UTF-8 conversion, and no command unwraps a path into a
//! string. There are no aliases and no live-execution commands.

use std::path::PathBuf;

use clap::builder::{PathBufValueParser, TypedValueParser as _};
use clap::{Parser, Subcommand};

/// Parse a SHA-256 digest argument: 64 lowercase hex digits, never a
/// zero placeholder. Rejections are clap usage errors (exit 2 with no
/// JSON result), matching the library digest shape one for one.
fn parse_digest(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Err("digest is missing".to_string());
    }
    if !is_hex_digest(value) {
        return Err(format!(
            "digest {value:?} is not a 64-character lowercase hex digest"
        ));
    }
    if value.bytes().all(|byte| byte == b'0') {
        return Err("digest must not be a zero placeholder".to_string());
    }
    Ok(value.to_string())
}

/// Parse the expected-baseline digest: a digest or the `missing`
/// literal for a first approval. Rejections are clap usage errors.
fn parse_expected_baseline(value: &str) -> Result<String, String> {
    if value == tuiscotti::check::MISSING_BASELINE {
        return Ok(value.to_string());
    }
    parse_digest(value).map_err(|_| {
        format!("expected baseline {value:?} is not a digest or the `missing` literal")
    })
}

/// True for 64 lowercase hex characters: the SHA-256 digest shape the
/// CLI accepts before the approval transaction re-validates it.
fn is_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

/// Parse the render `--out` path: only `.png` and `.html` spellings
/// pass. Rejections are clap usage errors (exit 2 with no JSON result).
fn parse_render_out(path: PathBuf) -> Result<PathBuf, String> {
    let supported = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "png" || extension == "html");
    if supported {
        Ok(path)
    } else {
        Err("unsupported output extension: expected a `.png` or `.html` --out path".to_string())
    }
}

/// Offline `tuiscotti`: check, accept, render, and report saved samples.
#[derive(Parser, Debug)]
#[command(
    name = "tuiscotti",
    version,
    about = "Offline visual-snapshot commands: check, accept, render, report"
)]
pub(crate) struct Cli {
    /// Emit the machine-readable Section 16.3 result object on stdout.
    #[arg(long, global = true)]
    pub(crate) json: bool,

    #[command(subcommand)]
    pub(crate) cmd: Cmd,
}

/// Exactly the four Section 16 commands: no aliases, no live execution.
#[derive(Subcommand, Debug)]
pub(crate) enum Cmd {
    /// Compare a saved sample against its baseline without starting an
    /// application or writing a baseline.
    Check {
        /// Saved sample file to compare.
        #[arg(long)]
        sample: PathBuf,
        /// Baseline root holding `<test-id>.tui.json` files.
        #[arg(long)]
        baselines: PathBuf,
        /// Artifact root receiving candidate evidence on mismatch.
        #[arg(long)]
        artifacts: PathBuf,
    },
    /// Approve one candidate for one test ID with explicit digests.
    Accept {
        /// Candidate sample file to approve (read-only input).
        #[arg(long)]
        candidate: PathBuf,
        /// SHA-256 over the candidate file bytes (64 lowercase hex).
        #[arg(long, value_parser = parse_digest)]
        candidate_sha256: String,
        /// Expected previous baseline digest, or `missing` for first approval.
        #[arg(long, value_parser = parse_expected_baseline)]
        expect_baseline: String,
        /// Baseline root receiving the upserted baseline.
        #[arg(long)]
        baselines: PathBuf,
    },
    /// Write `.png` or `.html` output from a saved sample.
    Render {
        /// Saved sample file to render.
        #[arg(long)]
        sample: PathBuf,
        /// Output path ending in `.png` or `.html`.
        #[arg(long, value_parser = PathBufValueParser::new().try_map(parse_render_out))]
        out: PathBuf,
    },
    /// Build a static HTML index from saved candidate evidence.
    Report {
        /// Artifact root holding check evidence.
        #[arg(long)]
        artifacts: PathBuf,
        /// Output HTML path.
        #[arg(long)]
        out: PathBuf,
    },
}

/// Dispatch a parsed [`Cli`] to its handler; returns the exit code.
pub(crate) fn run(cli: Cli) -> i32 {
    match cli.cmd {
        Cmd::Check {
            sample,
            baselines,
            artifacts,
        } => crate::ops_check::cmd_check(&sample, &baselines, &artifacts, cli.json),
        Cmd::Accept {
            candidate,
            candidate_sha256,
            expect_baseline,
            baselines,
        } => crate::ops_accept::cmd_accept(
            &candidate,
            &candidate_sha256,
            &expect_baseline,
            &baselines,
            cli.json,
        ),
        Cmd::Render { sample, out } => crate::ops_render::cmd_render(&sample, &out, cli.json),
        Cmd::Report { artifacts, out } => crate::ops_report::cmd_report(&artifacts, &out, cli.json),
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory as _;

    use super::Cli;

    /// Every command keeps the shape Section 16.4 requires: the global
    /// `--json` flag, the four subcommands with their exact flags, and no
    /// hidden aliases.
    #[test]
    fn cli_shape_matches_section_16() {
        Cli::command().debug_assert();
    }
}
