# Verification (V01)

All `cargo` invocations via `rust-build-guard -- mise exec -- cargo …`
with a `disk-check` probe first; `nextest run` only (never `cargo test`
or `--config`); the `tui-test-rs` patch from untracked task-local
`.cargo/config.toml` (upstream-qual `049fb8c`), never committed.

## Downstream gates (V01, re-run after doc fixes)

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`: zero warnings.
- `cargo nextest run --workspace --locked --offline`: 167 run, 167 passed, 0 skipped.
- Per suite: `pure_view` 30, `approval` 30, `samples` 20, `tuiscotti`
  unit 19, `compare` 14, `session_migration` 12, `cli_check_accept` 12,
  `cli_render_report` 12, `cli_surface` 8, `no_duplicate_engine` 5,
  `bin/tuiscotti` 4, `examples_lane` 1. Doctests in `view.rs` (2 bodies)
  proven via a temporary nextest mirror, then removed.
- C18: no duplicate live engine/renderer/input encoder (5/5 removal
  tests); dependency graph has no local PTY runtime — lockfile holds
  only `tuiscotti`, `tuiscotti-cli`, `tui-test-rs` without a registry
  source; PTY enters only through the `tui-test-rs` subtree.

## Upstream per-task evidence (recorded at acceptance)

U01 `snapshot_policy` 14/14 · U02 `owned_snapshot` 11/11 · U03
`offline_render` 7/7 (13/13 raster) · U04 `pinned_fonts` 6/6 ·
U05 `graphemes` 14/14 · U06 `pixel_compare` 5/5 · U07 `checked_input`
12/12 · U08 `launch_options` 12/12 ×4 runs · U09 `raw_bytes` 4/4 ·
U10 `request_waits` 18/18 ×3 runs · U11 `underline_color` 1/1 (5/5
raster). Full upstream suites green per task (fmt + clippy clean).
Details: `docs/migration/prs/U01.md` … `U11.md`,
`docs/migration/reviews/U01.md` … `U11.md`.

## Not available (recorded, not waived)

- No Rust CI runs on draft PRs #276–#286 (only `license/cla`
  reports); no maintainer reviews yet (probed 2026-10-26, all OPEN).
- Windows runs (raw_bytes `#![cfg(unix)]`, read-only-root, launch
  Windows rules) and Linux runs: pending upstream/repo CI.
- Registry check unreachable from this host (crates.io API probe
  returned empty); release contents unverified — adoption stays pending.
- `cargo metadata` / `cargo tree` are denied by the build guard; graph
  claims use manifest + lockfile analysis (the same method the C18 test
  documents). `cargo test --doc` is guard-denied; doctest bodies proven
  by mirror execution instead.

## Completion state

`UPSTREAM_PENDING` (path patch in use, no release yet), never
`VENDOR_COMPLETE`. Local gates: all pass (`LOCAL_COMPLETE` per §23.3).
