# Verification (V01)

All `cargo` invocations via `rust-build-guard -- mise exec -- cargo …`
with a `disk-check` probe first; `nextest run` only (never `cargo test`
or `--config`); the `tui-test-rs` patch from untracked task-local
`.cargo/config.toml` (upstream-qual `049fb8c`), never committed.

## Downstream gates (V01, re-run after doc fixes)

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`: zero warnings.
- `cargo doc --workspace --no-deps --locked --offline`: zero errors, zero warnings (post-D07 intra-doc link fixes).
- `cargo nextest run --workspace --locked --offline`: 167 run, 167 passed, 0 skipped.
- Final re-run on the F01 HEAD (docs-only changes after V01): 167/167 green, disk `ok`.
- Per suite: `pure_view` 30, `approval` 30, `samples` 20, `tuiscotti`
  unit 19, `compare` 14, `session_migration` 12, `cli_check_accept` 12,
  `cli_render_report` 12, `cli_surface` 8, `no_duplicate_engine` 5,
  `bin/tuiscotti` 4, `examples_lane` 1. Doctests in `view.rs` (2 bodies)
  proven via a temporary nextest mirror, then removed.
- C18: no duplicate live engine/renderer/input encoder (5/5 removal
  tests); dependency graph has no local PTY runtime — lockfile holds
  only `tuiscotti`, `tuiscotti-cli`, `tui-test-rs` without a registry
  source; PTY enters only through the `tui-test-rs` subtree.

## Round-2 / round-3 re-verification (2026-10-26, ~18:45 UTC)

Pinned upstream: `donbeave/tui-test` `goal/qual-stack-r3` @
`9e97999763244b817781b4f49064a9b76d0a2632` (short `9e97999`).
Fork tip, the `upstream-qual` checkout HEAD, and the `Cargo.toml` /
`Cargo.lock` pin all agree. Stack suites, verified locally by the
tasking session: 129/129 integration + 567/567 lib. Reproducing run
(from a checkout carrying the stack's task-local cargo config):
`cargo nextest run --workspace --locked --offline`. This record did
not re-run the stack suite: on this host the build guard denies
cargo under the stack's config-include setup
(`rust build guard: denied: Cargo configuration include/environment
can alter build outputs`), so the totals stand as session-reported.

Downstream HEAD `1501d5a264fdb4f7a4b4997744d5108a893a9d1d`
(`goal/tui-test-extension`; docs-only over code HEAD `49453af`),
re-run this session via `rust-build-guard -- mise exec -- cargo …`
with a `disk-check` probe first:

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`: zero warnings.
- `cargo nextest run --workspace --locked --offline`: 205 run, 205 passed, 0 skipped.
- `CI=true` lane (same command with `CI=true` in the guard
  environment; the guard merges the caller env into the build env):
  205 run, 205 passed, 0 skipped.
- DCO: all 80 commits since `origin/main` (`a1ea27f`) carry
  `Signed-off-by`; fork tip `fork/goal/tui-test-extension` ==
  `1501d5a`, i.e. the DCO-fixed force-push has landed and the remote
  is in sync (PR `tailrocks/tuiscotti#15` head `1501d5a`, READY,
  updated 2026-10-26T18:40:48Z).

Reviews: the tasking session reports third-review ACCEPT verdicts
on the round-3 rework; the fourth review round is pending. No
GitHub reviews are submitted on #276–#286 (verified via
`gh pr view --json reviews`: empty on all 11) and no on-disk
round-3 review docs exist under `docs/migration/reviews/`.

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
  reports); no maintainer reviews yet (probed 2026-10-26, all OPEN;
  re-probed ~18:44 UTC: still OPEN/DRAFT/REVIEW_REQUIRED, only
  `license/cla` via `gh pr checks`, no submitted reviews).
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
