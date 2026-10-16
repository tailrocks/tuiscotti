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

## Workflow then and now

```text
# v0.1
UPDATE_BASELINE=1 cargo test   # ambient bless — gone

# now
cargo nextest run --locked --offline --all-features   # writes actuals, fails missing/changed
tuiscotti accept --store <dir> <name_or_glob>        # explicit per-name or pattern, after review
cargo nextest run --locked --offline --all-features   # green
```
