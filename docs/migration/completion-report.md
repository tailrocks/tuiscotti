# Tuiscotti migration: completion report (F01)

## 1. Completion state

`LOCAL_COMPLETE` with `UPSTREAM_PENDING`. All locally executable work
passes: 167/167 downstream tests, fmt/clippy clean, C01–C18 proved,
every task reviewed to ACCEPT. The `tui-test-rs` path patch stays in
use because no upstream release contains the new APIs yet. This report
is not `VENDOR_COMPLETE`, and no section claims finished external
review. Records use relative repository paths.

- Downstream base: `a1ea27f757d75d07b05499276cd932e9f8e067ad`, branch
  `goal/tui-test-extension` (repos/downstream).
- Upstream base: `f2760592ed00e90436f8c3bae124dab712eaecf8`
  (repos/upstream, fork remote `fork`).
- Qualification stack: `goal/qual-stack` at `049fb8c`
  (repos/upstream-qual) — main + U02/U03/U04/U05/U06/U07/U08/U09/U10
  heads plus two assembly fixups. U01 is excluded by design; U11
  qualifies on its PR only (D-QUAL-1 correction).
- Reproducible qualification: `docs/migration/decisions.md`
  (D-GUARD-1, D-QUAL-1, D-QUAL-2), `docs/migration/verification.md`.

## 2. Last architecture and crate list

Two shipping crates. `crates/tuiscotti` (facade: `view`, `sample`,
`check`, `approve`, `report`, `error`, `render_profile`, examples) owns
pure-view capture, the sample format, frozen checks, approval, and
reports. `crates/tuiscotti-cli` (binary `tuiscotti`, `main.rs` plus
`cli`/`input`/`json_out`/`ops_*`) owns the four offline commands. The `tui-test-rs` crate supplies
child processes, emulation, sessions, and input. It also supplies
locators, waiting, rendering, fonts, pixel comparison, and recording.
Application tests drive `tui_test::Session` directly; no tuiscotti
`Session`, locator engine, input API, or operation enum exists.
Guides: `README.md`, `docs/API.md`, `docs/CLI.md`,
`docs/ARCHITECTURE.md`, `docs/LIMITATIONS.md`, plus corrected
`docs/TESTING.md`, `docs/SNAPSHOTS.md`, `docs/PERFORMANCE.md`
(superseded banner), `CONTRIBUTING.md`.

## 3. Removed implementations and dependencies

Full record: `docs/migration/removals.md`. Removed crates:
`tuiscotti-core`, `-render`, `-runtime`, `-insta`, `-fixtures`,
`-bench`, `xtask` (311 → 33 Rust files, 65,679 → 14,973 lines).
Removed: termpane/PTY transport, workers/threads/reaping, key/mouse encoders, live locators, and wait loops. Removed also: daemon/registry/protocol/MCP, recorders/video/piped capture/shell exec, and local rasterizer/renderer. Removed also: old stores and generation tags, ANSI baselines/import/old schemas, frozen-tree import/export, semantic/manual-clock frameworks, sixel/Kitty inspection, and the Insta approval engine. Removed direct deps:
`termpane`, `crossterm`, `swash`, `image-compare`, `insta`, bench,
fixture, and xtask dependencies. Removed-contract CI lanes, xtask mise
tasks, and stale guide sections are gone; history stays only in
`docs/migration/`.

## 4. Kept features and specified limits

Kept: pure-view capture in a single process. Kept also: versioned samples with `state` / `state-and-pixels` gates, frozen baseline checks with external evidence dirs, and static reports. Kept also: explicit `ApprovalStore::accept` for workstations only (CI refuses), and four offline CLI commands with exit codes 0/1/2/3 and `tuiscotti.cli.v1` JSON. If a check fails, keep the gate unchanged.

Limits: no pixel-only gates, tolerances, masks, or redaction. Limits also: no live execution in the CLI and no approval globs. Limits also: pinned fonts only (`pinned-v1`, DejaVu slot-3), mandatory corpus ink per `samples` metrics tests, and `NO_COLOR` behavior through the CLI library.

Unsupported: graphics beyond the contract, semantic
roles from pixels, arbitrary-child control via application clocks.

## 5. Upstream PRs and their engineering requirements

Eleven drafts on microsoft/tui-test, all OPEN/REVIEW_REQUIRED, only
`license/cla` passing, no maintainer reviews (probed 2026-10-26).
U01 policy #276, U06 pixel compare #277, U02 owned snapshot #278, and U09 raw bytes #279 are open. U03 offline render #280, U10 waits #281, U07 checked input #282, and U11 underline color #283 are open. U04 pinned fonts #284, U05 graphemes #285, and U08 launch options #286 are open. Each carries
`SnapshotPolicy`/atomic capture/offline render/pinned fonts and
contracts/rustybuzz shaping/RGBA compare/checked input/`spawn_with`
/`write_bytes`/quiet waits/underline color, with mandatory corpus
cases, typed errors, no unsafe, and MSRV-compatible APIs. Per-PR
evidence: `docs/migration/prs/U01.md` … `U11.md`; status:
`docs/migration/prs/PR-STATUS.md`. U02–U11 are mandatory for all kept
features; U01 is recorded independently (policy improvement only).

## 6. Actual test results and not available checks

Full record: `docs/migration/verification.md`. Downstream V01:
167/167 nextest green (fmt/clippy clean), C01–C18 proved, 2 doctest
bodies proven by mirror execution. Upstream per-task suites green at
acceptance (14, 11, 7/13, 6, 14, 5, 12, 12×4, 4, 18×3, 1/5). Not
available: Rust CI on the draft PRs, maintainer reviews, and Windows runs (`raw_bytes` unix-only, read-only-root, launch Windows rules). Not available also: Linux runs and registry-content checks (crates.io unreachable from this host).
`cargo metadata/tree` and `cargo test --doc` are build-guard denied;
graph and doctest claims use the documented substitute methods.

## 7. Code review findings and the decision for each finding

Records: `docs/migration/reviews/`. Upstream: U06 rework (workspace
`png` dev-dep), U07 rework (paste-terminator test), U10 rework
(deadline-tie timestamp), U03/U04/U05/U08/U11 minor-fix rounds
(cache-key pinning, fixture move, doc precision) — all ACCEPT.
Downstream: D01 follow-up accept; D03 rework (atomic report writes);
D04 rework (real OS lock, mandatory re-render, §16.3 codes); D05
rework (render exit 3); D06 accept; D07 two rework rounds (CI lanes,
contributor surfaces); V01 rework (guide rewrites, record corrections).
Decision for every finding: fix in the code, re-run green, re-review to
ACCEPT. No finding was waived; no test was narrowed to pass.

## 8. Dependency source and full versions

Rust 1.98.1 (`rust-toolchain.toml`), edition 2024, nextest 0.9.148,
`tuiscotti` 0.2.0. `tui-test-rs` 0.1.1 provisional: path patch to
upstream-qual `049fb8c` via untracked task-local `.cargo/config.toml`
(never committed, removed at R02). Features enabled on the dep:
`recording-raster`, `recording-font-jetbrains-mono-styles`. Facade
direct deps: `base64`, `fs2`, `image`, `ratatui`, `serde`,
`serde_json`, `sha2` (workspace pins). Lockfile: 256 packages; only
`tuiscotti`, `tuiscotti-cli`, `tui-test-rs` lack a registry source;
no `termpane`/`swash`/`image-compare`/`insta`; portable-PTY enters only
through `tui-test-rs`. No global `CARGO_TARGET_DIR`/`RUSTC_WRAPPER`;
no `unsafe` downstream; DCO sign-off on all commits.

## 9. Remaining external conditions

Upstream maintainers must review and merge PRs #276–#286, then publish
a `tui-test-rs` release containing the U02–U11 APIs. Only the registry
package proves release contents — never a PR label. If a maintainer
rejects a necessary feature, the unchanged rejection text goes to the
decision log and the smallest supported alternative is prepared; the
removed engine never returns as a hidden fallback. While no mandatory
function is available, the state stays `UPSTREAM_PENDING`.

## 10. Full next commands when release adoption stays pending

Run these after a release containing the U02–U11 APIs exists (replace
`X.Y.Z` with the actual registry version):

```sh
/Users/donbeave/.local/bin/disk-check
cd repos/downstream
rm .cargo/config.toml   # remove the provisional path patch (untracked)
mise exec -- cargo update -p tui-test-rs --precise X.Y.Z
mise exec -- cargo fetch --locked
/Users/donbeave/.local/bin/rust-build-guard -- mise exec -- cargo nextest run --workspace --locked --offline
/Users/donbeave/.local/bin/rust-build-guard -- mise exec -- cargo fmt --all -- --check
/Users/donbeave/.local/bin/rust-build-guard -- mise exec -- cargo clippy --workspace --all-targets --locked --offline -- -D warnings
/Users/donbeave/.local/bin/rust-build-guard -- mise exec -- cargo doc --workspace --no-deps --locked
```

Then: confirm `Cargo.lock` lists the registry source for `tui-test-rs`
X.Y.Z, confirm no `path+` source remains, send the dependency change
for code review, and flip R02 to done only after that review accepts.
Keep monitoring PRs #276–#286 for maintainer feedback until then.
