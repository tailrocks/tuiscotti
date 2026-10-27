# Migration

No shims, no aliases, no deprecation periods: removed items stay
removed. This file records what changed and what to do about it.

## G8 brand rename (`tuiscotti`)

The `tuisnap` binary became `tuiscotti` (crate `tuiscotti-cli`
`[[bin]]` name); `TUISNAP_*` env became `TUISCOTTI_*` with no
compat aliases; `tui-snap.toml` became `tuiscotti.toml`;
`tuisnap-default` / `tuisnap-assert` profile and provenance strings
became `tuiscotti-*`; machine-protocol brand fields
(`Capabilities.tuiscotti`, `OpResult::Version.tuiscotti`) renamed
likewise. The "Before" column and the history sections below keep
the old names as history (marked; not current surfaces). The machine
protocol and its brand fields were removed at D07 with the
old layered crates; no `tuiscotti.toml` config surface exists on this
branch (see [migration/removals.md](migration/removals.md)).

## Layout after D07 (head `da3d9ae`)

The workspace moved from a single crate (`src/`, `tests/`,
`examples/`) to `crates/`, and D07 removed every crate but two:

| Before | After |
|---|---|
| `src/*` (crate `tuisnap`) | `crates/tuiscotti/src/*` (library `tuiscotti`: `view`, `sample`, `check`, `approve`, `report`, `render`, `render_profile`, `error`) |
| `src/main.rs` (binary `tuisnap`) | `crates/tuiscotti-cli/src/main.rs` (binary renamed to `tuiscotti` in G8) |
| `tests/visual/approved` | Removed at D07; baselines are per-test `<name>.tui.json` files managed by `FrozenStore` / `tuiscotti accept` |
| `tests/fixtures/*` | Removed at D07 with `tuiscotti-fixtures` |
| `tests/snapshots/*` | Removed at D07 with `tuiscotti-fixtures` |
| `examples/01–08` | Removed as a set; the 0.4.0 branch adds one maintained example, `crates/tuiscotti/examples/hello.rs` (`cargo run -p tuiscotti --example hello`) |
| `tuisnap::…` paths | `tuiscotti::…` paths (binary renamed in G8; see above) |

Rust imports change (`tuisnap::` → `tuiscotti::`); the move itself
did not change the CLI grammar (the grammar later narrowed at D05/D07;
see [CLI.md](CLI.md)). The `tuisnap` binary name and `tui-snap.toml` named
here were renamed in G8 (see above).

## Schema versions

- Samples are versioned (`SAMPLE_SCHEMA = "tuiscotti.sample.v1"`);
  checks (`CHECK_SCHEMA = "tuiscotti.check.v1"`) and CLI output
  (`tuiscotti.cli.v1`) carry their own schema identifiers. A sample
  seals each part with SHA-256 digests (`png_sha256` / `rgba_sha256`
  over the image, plus the render-contract palette digest); old
  snapshot schemas do not import — re-capture. See
  [SNAPSHOTS.md](SNAPSHOTS.md).
- Frozen checks compare a saved sample against a baseline tree and
  report `pass`, `missing`, or `different` (`FrozenStore::check` /
  `check_file`); evidence lands in a separate artifacts directory.
  There is no `.ansi` dump, `.html` byte gate, or ANSI replay surface
  on this branch (removed at D07).

## Removed surfaces (do not look for them)

| Removed | Replacement |
|---|---|
| `BLESS=1` / `UPDATE_SNAPSHOT=1` ambient approval | Explicit `ApprovalStore::accept` / `tuiscotti accept` with both digests (refuses under `CI`) |
| CLI `check`, `run`, `digest`; `accept --all`; `report --store` | Current grammar in [CLI.md](CLI.md): `tuiscotti check --sample <file> --baselines <dir> --artifacts <dir>`, `tuiscotti accept --candidate <file> --candidate-sha256 <sha> --expect-baseline <sha\|missing> --baselines <dir>`, `tuiscotti render --sample <file> --out <file.png\|html>`, `tuiscotti report --artifacts <dir> --out <file.html>`; no `review`, `machine`, or glob commands |
| `tuisnap::ratatui_shot::{widget_frame, draw_frame}` | `tuiscotti::view::{capture_view, snapshot_from_buffer}` (pure-view capture through a Ratatui `TestBackend`; no child or PTY) |
| `tuisnap::Baseline` | `tuiscotti::check::FrozenStore` |
| `PtySession` / `run_once(argv, opts, sends)` | Upstream `tui_test::{Session, Engine}` used directly from application test code (no downstream session wrapper); waits fail on timeout instead of returning `false` |
| `tools/*.py` helpers (`subset_fonts.py`, `test_migration.py`, `migrate_fixture_v3.py`) | Gone with `tools/`; font bytes are committed under `crates/tuiscotti/fonts/` (see [LIMITATIONS.md](LIMITATIONS.md); `assets/fonts/` holds reference-only files, see `assets/fonts/FONTS.md`) |
| Vendored `vt100` fork, `vendor/termlens`, `termpane` git dependency, `portable-pty` + `alacritty_terminal` bridge | PTY engine is upstream `tui-test-rs` `0.1.1` (provisional pinned fork per D-R02-1, never a direct downstream PTY dep); `portable-pty` + `alacritty_terminal` remain only transitively through `tui-test-rs`; `vt100` and `termpane` are gone entirely |
| Hand-rolled SGR replay parser, `ansi::replay_raw`, `tuisnap::pty`, `tuisnap::termlens` re-export | Deleted with no replacement; samples carry canonical state plus decoded RGBA, and all live behavior is upstream-owned (see [API.md](API.md)) |
| `Locator::count_at_least(n)` (unbound) | Removed in P4 from the pre-D07 tree; locators are upstream-owned since D06 — no `Locator` type exists downstream (application tests drive `tui_test` directly) |

## Engine history (why the old docs mention other emulators)

1. Vendored `vt100` 0.16.2 fork → replaced by `termpane` v0.1.0
   (git-only) over vendored `termlens` 0.9, schema 3 unchanged.
2. `termpane` git + vendored `termlens` → interim `portable-pty` 0.9 +
   `alacritty_terminal` 0.26 bridge while `termpane` was prepared for
   crates.io publication.
3. `portable-pty` + `alacritty_terminal` → released `termpane` `0.1` +
   `termpane-vt` `0.1` from crates.io.
4. `termpane` backend → upstream `tui-test-rs` `0.1.1` (D07; provisional
   pinned fork per D-R02-1). `vt100` is gone entirely; `portable-pty` +
   `alacritty_terminal` are transitive through `tui-test-rs` only.
   See [DECISIONS.md](DECISIONS.md).

MSRV history: 1.97 during the early prototype window, 1.98.1
through the D07 migration; current pin is 1.99.1
(`rust-toolchain.toml`).

## Behavior changes in P1–P8 (7,550-scenario visual suite refinements)

> VINTAGE (pre-D07 engine era): the items below record refinements to
> surfaces that D07 removed (`parse_chord`/key codecs, `Session` waits,
> `Locator`, `WaitError`, `proto` verdicts, `Provenance`, grouped
> approval with manifest seals, ANSI dumps). They stay as the change
> record, not current API. What survives on this branch: explicit
> digest-arbitrated approval with stale-digest rejection and CI refusal
> (`ApprovalStore::accept`; see [SNAPSHOTS.md](SNAPSHOTS.md)), and the
> `ratatui 0.30` floor from item 1.

1. **Dependency floor (`ratatui 0.30`)**:
   `tuiscotti` targets `ratatui = "0.30"` (workspace-wide), up from
   `=0.29.0`. `crossterm` is not a workspace dependency at all on this
   branch. Consumers still on `ratatui 0.29` must upgrade to
   `ratatui 0.30` or stay on the previous commit.
2. **Bare `'f'` / `'F'` / `'-'` / `'+'` key parsing and `-`-joined chords (`P1`)**:
   `parse_chord("f")` and `Key::from_str("f")` now return `Key::Char('f')`,
   `"F"` returns `Key::Char('F')`, `"-"` returns `Key::Char('-')`, and
   `"backtab"` parses as `Shift+Tab`. Bare `"+"`, trailing `"+"` after
   modifiers (`"ctrl++"`, `"ctrl-+"`), and `"plus"` (`"ctrl+plus"`) now
   parse as `Key::Char('+')` (while malformed `"++"` or trailing-modifier
   `"Ctrl+"` still fail). Function keys require an explicit number in
   `"F1"`..`"F12"` (`"F0"`, `"F01"`, `"F13"` are rejected).
3. **`Session::mouse_drag` DEC mode gate (`P8`)**: `mouse_drag` now
   requires `DECSET 1002` (button-event motion) or `DECSET 1003`
   (any-event motion) in addition to pointer encoding (`DECSET 1006` SGR
   recommended). `DECSET 1000` (press/release only) is now rejected with
   `TuiError::ModeNotEnabled("mouse drag reporting (DEC 1002/1003) not enabled by the application")`
   (or `ActionError::Session(TuiError::ModeNotEnabled(...))`).
4. **Count-policy click errors and `Locator::count_at_least` removal (`P4`)**:
   `Locator::count_at_least(n)` was removed from the unbound `Locator`
   API (`BoundLocator::count_at_least(n)` remains for session-bound
   visibility assertions, and unbound callers use
   `Locator::expect_count_at_least(n, screen, rev)`). Calling `.click()`
   on `BoundLocator::count_at_least(n)` fails closed with
   `ActionError::Locate(LocateError::Usage(..))` (not
   `LocateError::Multiple`).
5. **`WaitError` multi-line `Display` (`P5`)**: `WaitError` (`Timeout`,
   `Cancelled`, `Unsupported`, and `Closed` when `WaitEvidence` is
   present) now formats a multi-line message ending with
   `\n--- last screen (<cols>x<rows>) ---\n<bounded screen dump>`
   (capped at 40 non-empty rows and 4096 bytes, or `<blank screen>`).
   Code matching exact single-line `to_string()` equality should match
   the first-line prefix or inspect `err.evidence()`.
6. **Recursive `proto::read_verdicts` and `Verdict::passed()`**:
   `read_verdicts` now walks nested subdirectories recursively and
   `Verdict::passed()` recognizes `"matched"` alongside `"pass"` and
   `"ok"`.
7. **Provenance host-path sanitization (`Provenance::now` and
   `Frame::to_json`)**: `argv[0]` is normalized to its basename and host
   filesystem paths in `argv[1..]` and `source` are stripped to
   basenames during `Provenance::now`, `Frame::to_json`, and
   `Frame::to_json_pretty`:
   - Trailing-slash multi-segment host directories
     (`/Users/alice/proj/`, `C:\Users\alice\proj\`), container/CI roots
     (`/srv/`, `/workspace/`, `/builds/`, `/github/`, `/app/`), and
     runtime `HOME` / `current_dir` / `TMPDIR` / `CARGO_HOME` /
     `CARGO_TARGET_DIR` prefixes are stripped to their basename.
   - Single-segment values directly under a Unix root or Windows drive
     root (`/foo`, `/foo/`, `C:\proj`, `C:\config.toml`) and `/` are
     preserved verbatim, whereas multi-segment Windows drive paths
     (`C:\Users\alice\proj\`, `D:/data/x.bin`) and UNC paths
     (`\\server\share\file.txt`, `\\é\x`) are reduced to their basename.
   - Multi-segment `/...` values with 3 or more segments whose final
     segment has an ASCII alphanumeric file extension (`name.ext`, via
     `last_segment_looks_like_file`) are also treated as host file paths
     and reduced to `name.ext` (for example `/custom/mount/view.rs` →
     `view.rs` and `/api/v1/users.json` → `users.json`, while
     extensionless routes such as `/api/v1` and `/api/v1/users` are
     preserved verbatim).
   In `ArtifactSet::All` stores, frames whose provenance previously
   carried absolute host paths will produce sanitized
   `<script id="tuiscotti-frame">` JSON inside `.html` and should be
   re-accepted once.
8. **Explicit approval guards (`CI` refusal, candidate seal, `run_id`
   freshness, single-segment `*`)**:
   - `tuiscotti accept`, `Store::accept`, and
     `GroupedStore::{accept, accept_matching, accept_all}` refuse when
     `CI` is set in the environment (non-empty and not `"0"` or
     `"false"`).
   - Prefix filters in `tuiscotti accept`, `tuiscotti review --filter`,
     and `GroupedStore::accept_matching` are segment-aware
     (`showcase/pages/tab` matches `showcase/pages/tab` and
     `showcase/pages/tab/...`, never `showcase/pages/tables/...`); `*`
     matches within a single path segment (no `/`), and `**` matches
     across `/`.
   - `accept` / `accept_matching` / `accept_all` require a sealed
     `<name>.manifest.json` and `<name>.verdict.json` whose recorded
     SHA-256 digests and artifact mode match the candidate on disk
     (rejecting unsealed "ghost" `.ansi` files or post-check tampering),
     skip scenarios whose fresh `.verdict.json` already recorded
     `status == "matched"`, and `accept_matching` enforces a `run_id`
     freshness guard across multi-scenario selections (`NEXTEST_RUN_ID`,
     `TUISCOTTI_RUN_ID`, or `GroupedStore::with_run_id`).
   - `tuiscotti accept` defaults `--actual` strictly to `<store>.actual`
     for grouped stores; pass `--actual target/tuiscotti/actual`
     explicitly when using a custom actual directory.
9. **Grapheme-boundary SGR reset in ANSI dumps (`render::ansi_dump` /
   `render::ansi_dump_with_cursor`)**: `ansi_dump` and
   `ansi_dump_with_cursor` now emit `ESC[0m` (and re-apply the active
   cell SGR) between adjacent cells whose text would otherwise join into
   one extended grapheme cluster under UAX #29 (such as
   regional-indicator pairs, base character + standalone combining mark
   across adjacent cells, emoji ZWJ sequences split across cells, or
   adjacent Hangul L+V Jamo cells). Any existing approved `.ansi` file
   containing such a cell pair will fail byte comparison once and should
   be re-accepted.

## Workflow then and now

```text
# v0.1
UPDATE_BASELINE=1 cargo test   # ambient bless — gone

# now (the workspace defines zero cargo features)
cargo nextest run --locked --offline   # writes actuals, fails missing/changed
tuiscotti accept --candidate <file> --candidate-sha256 <sha> --expect-baseline <sha|missing> --baselines <dir>
cargo nextest run --locked --offline   # green
```

