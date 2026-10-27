# CLI reference (`tuiscotti`)

Transcribed from the implemented Clap grammar (`tuiscotti --help`,
per-command `--help`), verified by running the binary. Exactly four
commands, all offline: nothing here starts an application, opens a
terminal device, or writes a baseline on its own.

Global shape: `tuiscotti [--json] <COMMAND>` with
`check | accept | render | report`. The only custom global flag is
`--json` (one machine-readable `tuiscotti.cli.v1` result object on
stdout; diagnostics stay on stderr); otherwise only clap's
`-h/--help` and `-V/--version` exist.

Exit codes: `0` for a pass or a successful offline operation;
`1` for a missing baseline or a differing sample; `2` for invalid
arguments (clap usage errors exit 2 with no JSON result, as does
`--out` extension rejection on `render` and malformed-digest
arguments on `accept`); `3` for file, format, profile, rendering,
approval, or I/O failures. Error codes (`INVALID_SAMPLE`,
`INVALID_STORE_PATH`, `INVALID_IMAGE`, `UNKNOWN_PROFILE`,
`MISSING_GLYPH`, `IO`, …) travel in the `tuiscotti.cli.v1` JSON
object with `--json`, or on stderr otherwise.

## Commands

```text
tuiscotti [--json] check --sample PATH --baselines DIR --artifacts DIR
tuiscotti [--json] accept --candidate PATH --candidate-sha256 SHA --expect-baseline SHA_OR_MISSING --baselines DIR
tuiscotti [--json] render --sample PATH --out PATH
tuiscotti [--json] report --artifacts DIR --out PATH
```

### `check --sample <PATH> --baselines <DIR> --artifacts <DIR>`

Frozen comparison of a saved sample. Reads the sample, compares it
against the baseline for its test ID, writes candidate evidence to
the artifact root on mismatch, and never writes a baseline. A missing
baseline reports `missing` at exit 1; a difference reports
`different` at exit 1 with the mismatch reason; a pass exits 0.

### `accept --candidate <PATH> --candidate-sha256 <SHA> --expect-baseline <SHA_OR_MISSING> --baselines <DIR>`

Explicit digest-arbitrated approval (the only path that writes a
baseline):

1. Read the candidate evidence and review it (pixels, state, diff).
2. Run `accept` with the candidate SHA-256 and the expected baseline
   digest (`missing` for a first approval).
3. The transaction re-locks the baseline root, re-validates both
   digests, re-renders image samples through the upstream renderer,
   and atomically installs the baseline — or rejects with a typed
   code and writes nothing.

Refusals: when `CI` is set in any form, approval stops before it
creates files (`approval is forbidden while CI is set`, no override).
Other refusals (digest mismatch, stale baselines, unknown profiles,
inconsistent re-render pixels, malformed samples) preserve baseline
contents: no baseline is created or replaced, although the store
directory and the stable lock file can exist after a refusal. There
is no `--all`, no glob, and no bless flag.

### `render --sample <PATH> --out <PATH>`

Offline rendering of a saved sample: extracts the stored PNG, or
re-renders the snapshot through the pinned upstream profile, or
writes a static `.html` page — selected by the `--out` extension.
An unsupported extension is an argument error (exit 2). Never starts
an application.

### `report --artifacts <DIR> --out <PATH>`

Builds a static HTML index from saved candidate evidence. Approval
state is never modified.

## Removed commands

`capture`, `inspect`, `diff`, `review`, `import`, `session`,
`record`, `trace`, `machine`, `init`, `doctor`, and `schema` do not
exist: live capture, recording, daemons, session registries, and the
op protocol moved to upstream `tui-test`. There are no shims:
unknown subcommands exit 2. The scriptable surface for live sessions
is the upstream Rust API (`tui_test::Session`), not this binary.
