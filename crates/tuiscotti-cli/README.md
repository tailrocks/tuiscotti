# tuiscotti-cli

Offline binary `tuiscotti` for the `tuiscotti` visual-regression
workflow: compare saved samples against frozen baselines, approve new
baselines explicitly, and render static evidence — all without starting
an application, opening a terminal device, or touching the network.

Live terminal work (sessions, input, locators, waits, live capture)
belongs to upstream `tui-test-rs` and is scripted through its Rust API
(`tui_test::Session`), not through this binary.

## Install

Requires Rust 1.99.1 with `cargo` on `PATH`. Install from
crates.io:

```sh
cargo install tuiscotti-cli --version 0.4.0 --locked
```

From a workspace checkout, run without installing via
`cargo run -q -p tuiscotti-cli --locked --offline -- <command>`.

## Commands

Exactly four subcommands; the only global flag is `--json` (one
machine-readable `tuiscotti.cli.v1` result object on stdout, diagnostics
stay on stderr):

```text
tuiscotti [--json] check --sample PATH --baselines DIR --artifacts DIR
tuiscotti [--json] accept --candidate PATH --candidate-sha256 SHA --expect-baseline SHA_OR_MISSING --baselines DIR
tuiscotti [--json] render --sample PATH --out PATH
tuiscotti [--json] report --artifacts DIR --out PATH
```

- **`check`** — frozen comparison of a saved sample against the baseline
  for its test ID. Never writes a baseline; on mismatch it writes
  candidate evidence under the artifact root. Missing baseline reports
  `missing`, a difference reports `different` with a typed reason.
- **`accept`** — the only path that writes a baseline. Approves one
  candidate for one test ID with two mandatory digests: the candidate
  SHA-256 and the expected previous digest (`missing` for a first
  approval). Re-locks the baseline root, re-validates both digests,
  re-renders image samples through the pinned upstream profile, and
  installs the baseline with one atomic rename — or rejects with a
  typed code and writes nothing. There is no `--all`, no glob, and no
  bless flag. Refuses before any write when the `CI` variable is set,
  in any form.
- **`render`** — offline rendering of a saved sample: extracts the
  stored PNG, re-renders a state-only sample through the pinned profile,
  or writes a static `.html` page — selected by the `--out` extension
  (`.png` or `.html` only).
- **`report`** — builds a static HTML index from saved candidate
  evidence under an artifact root. Never modifies approvals.

Exit codes: `0` pass or successful operation; `1` missing baseline or
differing sample (`check` only); `2` invalid arguments (clap usage
errors carry no JSON result, as do `render` `--out` extension
rejections and `accept` malformed-digest arguments); `3` file, format,
profile, rendering, approval, or I/O failures.

## Working workflow

```sh
# 1. check: first run reports missing (exit 1), writes candidate evidence
tuiscotti check --sample sample.tui.json --baselines baselines --artifacts artifacts
# 2. inspect the evidence
tuiscotti render --sample sample.tui.json --out candidate.png
tuiscotti report --artifacts artifacts --out index.html
# 3. accept: review the digests, then approve (missing = first approval)
CANDIDATE=$(sha256sum sample.tui.json | cut -d' ' -f1)
tuiscotti accept --candidate sample.tui.json --candidate-sha256 "$CANDIDATE" \
  --expect-baseline missing --baselines baselines
# 4. recheck: passes against the new baseline (exit 0)
tuiscotti check --sample sample.tui.json --baselines baselines --artifacts artifacts
```

A changed sample never self-heals: only an explicit `accept` with
matching digests changes a baseline. Under `CI`, `accept` always
refuses.

## Reference links

- [CLI reference](../../docs/CLI.md)
- [Snapshot, check, and approval semantics](../../docs/SNAPSHOTS.md)
- [API surface](../../docs/API.md)
- [Library crate](../tuiscotti/README.md)
