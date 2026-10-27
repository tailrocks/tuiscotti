#![doc = include_str!("../docs/main.md")]
mod cli;
mod input;
mod json_out;
mod ops_accept;
mod ops_check;
mod ops_render;
mod ops_report;

use clap::Parser;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StdoutFlush {
    Written,
    BrokenPipe,
    Failed,
}

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

fn main() {
    let cli = match cli::Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => error.exit(),
    };
    std::process::exit(cli::run(cli));
}
