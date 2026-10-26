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

Pinned upstream: `donbeave/tui-test` `goal/qual-stack-r4` @
`299a9ac15339935d630a6ee99a5ccc7b974bc5c7` (short `299a9ac`).
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

## Round-3 verification record (2026-10-27 UTC, downstream tip 06469b7)

Sources are the twelve files inventoried below; this section records
outcomes only, no code changes.

### P1 reconciliation per upstream PR (heads / code / body / CI)

All 11 upstream PRs OPEN/draft with Testing sections (except #277, see
note); CI `action_required` on every one (license/cla pass only, Rust CI
never ran).

- #276 U01 `c14303e` — follow-ups verified no-ops at tip; body clean, Testing present.
- #277 U06 `4fb316a` — FOREIGN branch, not ours; body uses `Tests` heading, not `Testing`; hygiene not ours to fix.
- #278 U02 `cb40f25` — follow-ups fixed at tip, verified (see independent verification below); body clean, Testing present.
- #279 U09 `97f09f0` — fixed at tip, verified (pipeline routing + 4/4 raw_bytes); body clean.
- #280 U03 `a6adbcc` — already fixed at tip, no-op re-verified (clippy no-default clean); body clean, Testing present.
- #281 U10 `5472eea` — fixed at tip, verified (21/21 + 4/4); body clean.
- #282 U07 `212ba1d` — fixer in progress; working tree dirty +434/-295 across 6 files plus tests/common; body clean (282-body variant).
- #283 U11 `7a1c7c9` — already fixed at tip; body clean, Testing present.
- #284 U04 `d5b1469` — already fixed at tip (fixed-width u64 digest); body clean.
- #285 U05 `ee633fa` — follow-ups verified present and tested at tip (blank-PNG and MissingGlyph pre-fix reproductions confirmed); no new commit needed.
- #286 U08 `8b44501` — already fixed at tip; body clean, Testing present.

Live bodies verified by normalized diff (trailing-whitespace-insensitive)
against drafts; live head SHAs equal each draft `Head:` line.

### Already-fixed confirmations (no new commit needed)

- U03 (`a6adbcc`): already fixed at tip; no-op re-verified, clippy no-default clean.
- U04 (`d5b1469`): already fixed at tip; fixed-width u64 digest variant confirmed.
- U08 (`8b44501`): already fixed at tip; body clean, Testing present.
- U11 (`7a1c7c9`): already fixed at tip; body clean, Testing present.

### Independent verifications with suite results

- U09 claim (write_bytes routes through shared request pipeline): VERIFIED.
  Code: `engine.rs` `write_bytes` delegates via `execute_raw_bytes` to shared
  `execute_request`; `RequestPlan::for_raw_bytes` carries the same pieces as
  `for_operation`. Suite: `cargo test -p tui-test-rs --test raw_bytes` 4 passed,
  0 failed (child_fixture, keep_nul/escape/invalid_utf8, large-payload order,
  write-after-close typed error).
- U10 claim (quiet-wait acquisition checks before blocking; timeouts use
  shared finalization): VERIFIED. Code: `run_quiet_wait_cancellable` routes
  through shared `execute_planned_request` with acquire-before-poll and
  acquisition deadline; in-engine unit tests pin behavior under held
  lifecycle write lock. Suites: `request_waits` 21 passed 0 failed (incl.
  timeout-finalizes-artifact and cancelled-keeps-reason), `quiet_lifecycle`
  4 passed 0 failed.
- U02 claim (clipping semantics, single-lock atomic capture, ungated-on-stable
  tests): VERIFIED. Code: `snapshot.rs` cursor `in_capture` clipping flag and
  per-scope computation; `engine.rs` capture holds one session + one
  terminal-state lock. Suite: `owned_snapshot` 11/11 pass at `cb40f25` in
  0.82 s under stable rustc 1.99.1.
- U06 claim (workspace png crate is the codec, no hand-rolled decoder):
  VERIFIED. Code: workspace `png = "0.17.16"`; `encode_png` uses
  `png::Encoder`, `decode_png` uses `png::Decoder`; zero hand-rolled-codec
  matches in `pixel_compare.rs`. Suite: `pixel_compare` 5/5 pass at `4fb316a`.

### Downstream triage at tip 06469b7 (branch goal/tui-test-extension)

Counts: FIXED 7, OPEN 1, UNCERTAIN 0. Tip
`06469b7a2cc74c9eea9c00feb8309bd17d3b305`.

- FIXED: approve.rs transaction ordering; check.rs diff boundedness;
  sample.rs Deserialize validation; bounded readers; json_out broken-pipe
  handling; ops_report link relativity; Cargo.toml fork pin + fonts in crate
  package.
- OPEN (1): deny.toml fork exception — `deny.toml:19-23` still permits the
  single git source `https://github.com/donbeave/tui-test`; deny-by-default
  otherwise holds; scoped and marked provisional pending upstream merge +
  registry release.

### Downstream #15 CI state at 06469b7

At the earlier recorded head `1501d5a`, CI was all green after a retry
(Required verdict pass; four Rust jobs pass; Jactionlint, Plan, DCO pass).
At the current tip `06469b7`, the body was cleaned of scaffolding codes and
CI was rerunning — no green run observed at this tip in the sources above.

### Upstream CI state and approval request

All upstream PRs #276–#286 show CI conclusion `action_required`:
first-time-contributor fork approval pending upstream, so the Rust CI
workflow never ran on any branch; only `license/cla` passes. Commit-status
state is pending with zero statuses. The ready-to-send maintainer approval
request (per-PR run URLs for the heads listed above) is at
`/tmp/approval-request.md`.

### Artifact inventory (purpose of each source file)

- `/tmp/round-3-status.md` — read-only consolidation of round-3 rework
  status: per-PR head/code/body/CI table, late U02/U06/U05 results,
  downstream #15 note, dependency-ordered pending list.
- `/tmp/pr-hygiene.md` — PR hygiene check: downstream #15 title/CI reads,
  upstream #276–#286 draft/Testing/code/CI table, and drift list (missing
  title expectation, unrun Rust CI, #282 body variant, foreign #277,
  no head drift, no stray codes).
- `/tmp/u09-u10-verify.md` — read-only claim verification for U09
  (pipeline routing) and U10 (quiet-wait acquisition/finalization) with
  code refs and focused suite results.
- `/tmp/u02-u06-verify.md` — read-only fixed-at-tip verification for U02
  (clipping, atomic capture, stable tests) and U06 (workspace png codec)
  with code refs and suite results.
- `/tmp/downstream-triage.md` — downstream triage at tip 06469b7: per-area
  FIXED/OPEN table with file:line evidence.
- `/tmp/fourth-review-brief.md` — fourth-review scope, reviewer
  assignments with overlaps, verdict rules, read-only rule, mandatory
  heads-refresh step.
- `/tmp/release-prep-proposal.md` — draft (not applied) v0.3.0 release
  prep: version-bump checklist, provisional binary-release config delta,
  pre-tag verification checklist, non-goals.
- `/tmp/external-review-prompt.md` — read-only independent merge-readiness
  review prompt: pinned heads with commit links, verdicts requested,
  evidence rules.
- `/tmp/approval-request.md` — maintainer message requesting approval of
  the 11 pending fork-branch CI runs (per-PR run URLs); approval runs CI
  only, merges nothing.
- `/tmp/release-notes-v0.3.0.md` — draft v0.3.0 notes (no tag/release/bump
  yet): delivery summary, fork/stack pins, verification evidence,
  upstream PR table, limitations.
- `/tmp/qual-r4-plan.md` — read-only qual-r4 integration plan: expected tip
  SHAs, parent-first merge order, predicted conflict points, post-merge
  gate commands. U07 tip provisional (fixer uncommitted changes).
- `/tmp/release-route.md` — release-route research: current mechanism
  (CI-only, no release configured), unconfigured items, minimal
  provisional GitHub-release delta, version-bump list.

### Pending chain

U07 land (only dirty tree; fix in progress) → qual-r4 run over fresh tips
→ re-pin fork + fonts step → fourth review round → human review gate →
merge the 11 upstream PRs → release + adopt registry release downstream.
