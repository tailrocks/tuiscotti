#![doc = include_str!("../docs/cli.md")]
use std::path::PathBuf;

use clap::builder::{PathBufValueParser, TypedValueParser as _};
use clap::{Parser, Subcommand};

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

fn parse_expected_baseline(value: &str) -> Result<String, String> {
    if value == tuiscotti::check::MISSING_BASELINE {
        return Ok(value.to_string());
    }
    parse_digest(value).map_err(|_| {
        format!("expected baseline {value:?} is not a digest or the `missing` literal")
    })
}

fn is_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

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

#[derive(Parser, Debug)]
#[command(
    name = "tuiscotti",
    version,
    about = "Offline visual-snapshot commands: check, accept, render, report"
)]
pub(crate) struct Cli {
    #[arg(
        long,
        global = true,
        help = "Emit the machine-readable CLI result object (tuiscotti.cli.v1) on stdout"
    )]
    pub(crate) json: bool,

    #[command(subcommand)]
    pub(crate) cmd: Cmd,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Cmd {
    #[command(
        about = "Compare a saved sample against its baseline without starting an application or writing a baseline"
    )]
    Check {
        #[arg(long, help = "Saved sample file to compare")]
        sample: PathBuf,
        #[arg(long, help = "Baseline root holding `<test-id>.tui.json` files")]
        baselines: PathBuf,
        #[arg(long, help = "Artifact root receiving candidate evidence on mismatch")]
        artifacts: PathBuf,
    },
    #[command(about = "Approve one candidate for one test ID with explicit digests")]
    Accept {
        #[arg(long, help = "Candidate sample file to approve (read-only input)")]
        candidate: PathBuf,
        #[arg(
            long,
            value_parser = parse_digest,
            help = "SHA-256 over the candidate file bytes (64 lowercase hex)"
        )]
        candidate_sha256: String,
        #[arg(
            long,
            value_parser = parse_expected_baseline,
            help = "Expected previous baseline digest, or `missing` for first approval"
        )]
        expect_baseline: String,
        #[arg(long, help = "Baseline root receiving the upserted baseline")]
        baselines: PathBuf,
    },
    #[command(about = "Write `.png` or `.html` output from a saved sample")]
    Render {
        #[arg(long, help = "Saved sample file to render")]
        sample: PathBuf,
        #[arg(
            long,
            value_parser = PathBufValueParser::new().try_map(parse_render_out),
            help = "Output path ending in `.png` or `.html`"
        )]
        out: PathBuf,
    },
    #[command(about = "Build a static HTML index from saved candidate evidence")]
    Report {
        #[arg(long, help = "Artifact root holding check evidence")]
        artifacts: PathBuf,
        #[arg(long, help = "Output HTML path")]
        out: PathBuf,
    },
}

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
