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
the old names as history (marked; not current surfaces).

## Current layout (head `75ff479`)

The workspace moved from a single crate (`src/`, `tests/`,
`examples/`) to `crates/`:

| Before | After |
|---|---|
| `src/*` (crate `tuisnap`) | `crates/tuiscotti-{core,render,runtime,insta}/src/*` (facade `tuiscotti`) |
| `src/main.rs` (binary `tuisnap`) | `crates/tuiscotti-cli/src/main.rs` (binary renamed to `tuiscotti` in G8) |
| `tests/visual/approved` | `crates/tuiscotti-fixtures/tests/visual/approved` |
| `tests/fixtures/*` | `crates/tuiscotti-fixtures/tests/fixtures/*` |
| `tests/snapshots/*` | `crates/tuiscotti-fixtures/tests/snapshots/*` |
| `examples/01–08` | `crates/tuiscotti/examples/01–08` |
| `tuisnap::…` paths | `tuiscotti::…` paths (binary and config renamed in G8; see above) |

Rust imports change (`tuisnap::` → `tuiscotti::`); the CLI grammar
is unchanged. The `tuisnap` binary name and `tui-snap.toml` named
here were renamed in G8 (see above).

## Schema versions

- `Frame` is versioned; current `FRAME_VERSION = 3` (`UnderlineStyle`
  + underline color canonical; blink frozen-visible with slow/rapid
  combined; hidden is conceal; overline dropped). Version-2 JSON
  does not import — re-capture.
- `Frame::digest` mixes the schema version and the cursor; old
  digests are incomparable by design.
- Grouped scenarios have no `.frame.json` in the approved tree: the
  `.ansi` dump is the cell-exact gate; under `ArtifactSet::All`,
  renderer changes are caught by the `.html` byte gate plus the PNG
  pixel gate, while under `ArtifactSet::AnsiCursor` only `<name>.ansi`
  (with canonical VT cursor trailer via `render::ansi_dump_with_cursor`)
  is committed and expected/actual `.png` + `.html` + diff PNG are
  rendered on demand via `formats::ansi::frame_from_ansi` when a check
  mismatches.

## Removed surfaces (do not look for them)

| Removed | Replacement |
|---|---|
| `BLESS=1` / `UPDATE_SNAPSHOT=1` ambient approval | Explicit `Store::accept` / `GroupedStore::accept(_matching)` / `tuiscotti accept` / `cargo insta review` |
| CLI `check`, `run`, `digest`; `accept --all`; `report --store` | Current grammar in [CLI.md](CLI.md) (`tuiscotti accept --store <dir> <name_or_glob>`, `tuiscotti review --filter`, `tuiscotti render --input <file.ansi>`); scriptable surface is `tuiscotti machine` + `proto::execute` |
| `tuisnap::ratatui_shot::{widget_frame, draw_frame}` | `tuiscotti::ratatui::{widget_frame, draw_frame, capture}` (signatures take `Provenance`) |
| `tuisnap::Baseline` | `tuiscotti::snapshot::Store` / `grouped::GroupedStore` |
| `PtySession` / `run_once(argv, opts, sends)` | `tui::{Tui, Session}`; waits fail on timeout instead of returning `false` |
| `tools/*.py` helpers (`subset_fonts.py`, `test_migration.py`, `migrate_fixture_v3.py`) | Gone with `tools/`; font subsets are committed under `assets/fonts/` (see `assets/fonts/FONTS.md`) |
| Vendored `vt100` fork, `vendor/termlens`, `termpane` git dependency, `portable-pty` + `alacritty_terminal` bridge | PTY engine is released `termpane` `0.1` (PTY + shadow terminal surface) + `termpane-vt` `0.1` (ECMA-48 / VT parser) from crates.io — no git/path deps, no patches (`vt100` is dev-only differential oracle in `tuiscotti-fixtures`) |
| Hand-rolled SGR replay parser, `ansi::replay_raw`, `tuisnap::pty`, `tuisnap::termlens` re-export | Deleted; normalized `.ansi` decoding is `formats::ansi::frame_from_ansi`, and raw-ANSI stream replay is `observe::Replay` / `tui_shell::Recording` over the current engine |
| `Locator::count_at_least(n)` (unbound) | `Locator::expect_count_at_least(n, screen, rev)` on unbound `Locator`, or `session.get_by(...).count_at_least(n)` (`BoundLocator::count_at_least(n)`) for session-bound visibility assertions |

## Engine history (why the old docs mention other emulators)

1. Vendored `vt100` 0.16.2 fork → replaced by `termpane` v0.1.0
   (git-only) over vendored `termlens` 0.9, schema 3 unchanged.
2. `termpane` git + vendored `termlens` → interim `portable-pty` 0.9 +
   `alacritty_terminal` 0.26 bridge while `termpane` was prepared for
   crates.io publication.
3. `portable-pty` + `alacritty_terminal` → released `termpane` `0.1` +
   `termpane-vt` `0.1` from crates.io (current). `vt100` remains a
   dev-only differential oracle in `tuiscotti-fixtures`.
   See [DECISIONS.md](DECISIONS.md).

MSRV history: 1.97 during the early prototype window; current pin is
1.98.1 (`rust-toolchain.toml`).

## Behavior changes in P1–P8 (7,550-scenario visual suite refinements)

1. **Dependency floor (`ratatui 0.30`, `crossterm 0.29`)**:
   `tuiscotti-core` now targets `ratatui = "0.30"` (and the workspace
   uses `crossterm = "0.29"`), up from `=0.29.0` and `=0.28.1`.
   `crossterm` is removed from `tuiscotti-core` normal dependencies.
   Consumers still on `ratatui 0.29` must upgrade to `ratatui 0.30` or
   stay on the previous commit.
2. **Bare `'f'` / `'F'` / `'-'` key parsing and `-`-joined chords (`P1`)**:
   `parse_chord("f")` and `Key::from_str("f")` now return `Key::Char('f')`,
   `"F"` returns `Key::Char('F')`, `"-"` returns `Key::Char('-')`, and
   `"backtab"` parses as `Shift+Tab`. Function keys require an explicit
   number in `"F1"`..`"F12"` (`"F0"`, `"F01"`, `"F13"` are rejected).
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
   `Frame::to_json_pretty` — including trailing-slash multi-segment host
   directories (`/Users/alice/proj/`, `C:\Users\alice\proj\`),
   container/CI roots (`/srv/`, `/workspace/`, `/builds/`, `/github/`,
   `/app/`), and runtime `HOME` / `current_dir` / `TMPDIR` /
   `CARGO_HOME` / `CARGO_TARGET_DIR` prefixes, while preserving `/` and
   single-segment `/foo/` or non-host slash arguments like `/api/v1`. In
   `ArtifactSet::All` stores, frames whose provenance previously carried
   absolute host paths will produce sanitized
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

# now
cargo nextest run --locked --offline --all-features   # writes actuals, fails missing/changed
tuiscotti accept --store <dir> <name_or_glob>        # explicit per-name or pattern, after review
cargo nextest run --locked --offline --all-features   # green
```

