//! `tuiscotti`: offline visual-snapshot commands.
//!
//! Exactly four commands, all offline:
//!
//! ```text
//! tuiscotti check --sample PATH --baselines DIR --artifacts DIR
//! tuiscotti accept --candidate PATH --candidate-sha256 SHA256 --expect-baseline SHA256_OR_MISSING --baselines DIR
//! tuiscotti render --sample PATH --out PATH
//! tuiscotti report --artifacts DIR --out PATH
//! ```
//!
//! `check` applies the frozen comparison without starting an application
//! or writing a baseline. `accept` runs the explicit digest-arbitrated
//! approval transaction. `render` extracts or re-renders a `.png`, or
//! writes a static `.html` page, from a saved sample. `report` builds a
//! static HTML index from saved candidate evidence.
//!
//! Exit codes follow GOAL Section 16.2: 0 for a pass or a successful
//! offline operation, 1 for a missing baseline or a differing sample, 2
//! for invalid arguments (clap usage errors stay at 2 with no JSON
//! result), and 3 for file, format, profile, rendering, approval, or I/O
//! failures. Error codes follow the Section 16.3 table. Exit 2 covers
//! both argument-error shapes: `render`'s `--out` extension rejection
//! and `accept`'s malformed-digest arguments (both report
//! `INVALID_SAMPLE`); every sample-content failure, including corrupt
//! samples, exits 3.
//!
//! The requested result goes to stdout (one JSON object plus a newline
//! with `--json`, one summary line otherwise). Diagnostics go to stderr.
//! Stdout writes tolerate a closed pipe: a vanished reader is a clean
//! exit 0, never a panic.

mod cli;
mod input;
mod json_out;
mod ops_accept;
mod ops_check;
mod ops_render;
mod ops_report;

use clap::Parser;

/// How a buffered stdout flush ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StdoutFlush {
    /// The bytes reached stdout; the caller returns its outcome code.
    Written,
    /// The reader went away; the caller exits 0 (nothing is lost).
    BrokenPipe,
    /// Stdout failed another way; the caller exits 3.
    Failed,
}

/// Write a complete report to stdout without panicking on a closed pipe.
///
/// Pure-report commands buffer their output and flush once here: a
/// broken pipe is [`StdoutFlush::BrokenPipe`] (the reader went away;
/// nothing is lost), any other error is reported on stderr.
pub(crate) fn write_stdout(text: &str) -> StdoutFlush {
    use std::io::Write as _;
    let mut out = std::io::stdout().lock();
    let result = out.write_all(text.as_bytes()).and_then(|()| out.flush());
    match result {
        Ok(()) => StdoutFlush::Written,
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => StdoutFlush::BrokenPipe,
        Err(error) => {
            eprintln!("error: stdout: {error}");
            StdoutFlush::Failed
        }
    }
}

/// Append one `\n`-terminated line to a report buffer.
pub(crate) fn push_line(buffer: &mut String, line: &str) {
    buffer.push_str(line);
    buffer.push('\n');
}

fn main() {
    // Native parsing: child argv stays `OsString` end-to-end, and clap
    // usage errors exit 2 with no JSON result.
    let cli = match cli::Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => error.exit(),
    };
    std::process::exit(cli::run(cli));
}
