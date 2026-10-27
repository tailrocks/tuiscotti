# cli

Clap surface: a global `--json` flag and exactly four offline subcommands.

Paths stay [`PathBuf`] end-to-end: clap parses them from `args_os`
with no lossy UTF-8 conversion, and no command unwraps a path into a
string. There are no aliases and no live-execution commands.

## Argument parsers

Parse a SHA-256 digest argument: 64 lowercase hex digits, never a
zero placeholder. Rejections are clap usage errors (exit 2 with no
JSON result), matching the library digest shape one for one.

Parse the expected-baseline digest: a digest or the `missing`
literal for a first approval. Rejections are clap usage errors.

True for 64 lowercase hex characters: the SHA-256 digest shape the
CLI accepts before the approval transaction re-validates it.

Parse the render `--out` path: only `.png` and `.html` spellings
pass. Rejections are clap usage errors (exit 2 with no JSON result).

## Commands

Offline `tuiscotti`: check, accept, render, and report saved samples.

Emit the machine-readable Section 16.3 result object on stdout.

Exactly the four Section 16 commands: no aliases, no live execution.

Compare a saved sample against its baseline without starting an
application or writing a baseline.

Saved sample file to compare.

Baseline root holding `<test-id>.tui.json` files.

Artifact root receiving candidate evidence on mismatch.

Approve one candidate for one test ID with explicit digests.

Candidate sample file to approve (read-only input).

SHA-256 over the candidate file bytes (64 lowercase hex).

Expected previous baseline digest, or `missing` for first approval.

Baseline root receiving the upserted baseline.

Write `.png` or `.html` output from a saved sample.

Saved sample file to render.

Output path ending in `.png` or `.html`.

Build a static HTML index from saved candidate evidence.

Artifact root holding check evidence.

Output HTML path.

## Dispatch

Dispatch a parsed [`Cli`] to its handler; returns the exit code.

Every command keeps the shape Section 16.4 requires: the global
`--json` flag, the four subcommands with their exact flags, and no
hidden aliases.
