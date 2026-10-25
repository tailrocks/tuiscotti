# Decision log

## D-P01-1: Work placement

Worktrees live under `/Users/donbeave/tuiscotti/repos/` (`downstream`, `upstream`, `upstream-wt-*`). Migration records live in the downstream repository under `docs/migration/`. Evidence: `docs/migration/workspaces.md`.

## D-P02-1: Palette method

The built-in view palette comes from an in-memory `AlacrittyEmu` with `Profile::default()` through `Emulator::colors()`. No child, PTY, or session starts. Corroborated by `Colors::default()` source. Canonical digest `92757b147e00dec394b10b7be2fa42f782f0d30325a5922570080f91d60f641e`. Never recalculate from a later dependency version.

## D-P02-2: Font bytes

Upstream bundled JetBrains Mono bytes are `pub(crate)`. Downstream keeps licensed font assets under version control. U04 decides whether upstream exposes bytes publicly. Evidence: `render/font.rs` `REGULAR_FONTS`/`STYLED_FONTS` visibility; font table in `api-contract.md`.

## D-U01-1: CreateMissing without direct create-new

`CreateMissing` writes through a same-directory temp file plus `hard_link` instead of a direct `create_new` on the final path. Same failure semantics, and readers never see a partial baseline. Found through a real race flake during writer testing. Evidence: `snapshot_policy.rs` concurrency tests, 15/15 stress runs green.

## D-U01-2: Rejected review suggestions

U01 review (verdict accept) left two SUGGESTIONs, both rejected with reason:

- Temp-file `create_new` collision under PID reuse returns a typed `NotFound` error, never wrong data. A retry loop adds its own edge cases for a state that needs a stale temp file plus PID reuse at once. Rejected: safe typed error beats added machinery.
- Crash residue `.<stem>.new-*` dotfiles stay. A best-effort sweep risks deleting a live writer's temp file. Rejected: residue is the documented cost of atomic writes.

## D-U06-1: png as dev-dependency

U06-B decodes through the workspace `png` crate declared as a dev-dependency of `tui-test-rs`, mirroring `tui-test-cli`. No new package version enters the graph. The library keeps no PNG dependency. Evidence: `cargo tree -i png`, unmodified `Cargo.lock`.

## D-U02-1: Rejected review suggestions

U02 review (verdict accept) left two SUGGESTIONs, both recorded without code change:

- A mid-mutation contender would strengthen the atomicity proof beyond the specified barrier recipe. Rejected: the recipe in Section 11.2 is satisfied; tearing-during-mutation coverage is unrequested scope.
- Timeout `panic!` in live-test polling and load-dependent PTY flakes. Rejected: Section 21.3 forbids chasing flakes by weakening tests; the flakes are environmental (green on rerun in isolation, in suite, on base and branch).

## D-U09-1: Review dispositions and limits

U09 review (verdict accept) left three SUGGESTIONs:

- Deterministic short-write instrumentation via the `cfg(test)`-gated hook: rejected. The hook is unreachable from integration tests; the report used the specified large-payload fallback through a real PTY.
- Windows PTY byte behavior: recorded as unavailable. The test file is `#![cfg(unix)]`; Windows CI gives vacuous passage only.
- `write_bytes` insertion point (after `Session::write`): kept stable for U07/U08 sequencing. U07 and U08 add methods in distinct regions; merges stay mechanical.

## D-U10-1: Deadline-tie race and review rework

The first U10 review found a BLOCKER: coincident quiet and overall deadlines could return success when a blocking capture straddled the deadline. Fixed with one post-observation timestamp checked overall-before-quiet in both arms. The re-review closed all three findings after three consecutive 18/18 focused runs. Evidence: `request_waits.rs` U10-J stability runs.

## D-U07-1: Paste-terminator test gap

The first U07 review found one REQUIRED gap: the specified paste-terminator rejection had an implementation but no test. Fixed with `paste_terminator_rejected_before_write` (Rejected variant plus zero new bytes). Re-review closed all four findings. Empty-paste framing asymmetry is documented on the method; the unreachable `MouseMode::None` arm carries a comment.

## D-U11-1: Underline color interactions

An explicit underline color is undimmed and wins over the inverse swap; a missing underline color follows the dimmed and swapped foreground. This matches prior pixel behavior for the missing case and is unspecified in Section 11.11; recorded here per Section 2.2. All underline styles draw the recorded single bar; only `Single` is exercised in U11's suite because canonical equality already compares full cells including every style value.

## D-U05-1: Shaping review dispositions

U05 review (verdict accept) left four SUGGESTIONs, all recorded without code change:

- Degenerate-rect fallback draws unclipped. Rejected: validated snapshots cannot carry degenerate cell geometry (checked dimension validation at construction), and U05-C ink-bounds assertions pin the permitted-region rule.
- rustybuzz MSRV is inferred (no `rust-version` key in its manifest). Recorded: 0.20.1 released 2024-11-12 against stable ~1.82, edition 2021, pure Rust, dependencies all pre-1.90; workspace `cargo tree` shows one `ttf-parser 0.25.1`.
- The 5-line `offline.rs` glyph-evidence getter outside files_owned is accepted: additive and required by U05-F assertions.
- U+10FFFF as a negative combining scalar is a noncharacter but valid for the negative case; the named crab case stays primary.

Shaper: rustybuzz 0.20.1, MIT, optional under `recording-raster`. No new shaping engine written.

## D-U08-1: Fixture placement and suggestion dispositions

U08 review (verdict accept) left five SUGGESTIONs. Two were fixed: the launch-probe fixture moved from `examples/` to `tests/fixtures/` with a non-executed test target declaration, and `u08_j` asserts its PATH precondition. Three recorded without code change: session-construction duplication stays (zero-change-to-`open()` is the Section 8.1 priority); Windows assertions run in CI with results recorded on the PR; the on-demand probe build fallback is exercised by CI's bare test selection.

## D-VAL-1: Upstream continuation validation limit

A probe showed upstream `TerminalSnapshot::try_new` accepts content-incoherent covered cells (non-blank grapheme in a covered column; width-2 leading cell overhanging the row end). Its `check_at` validates structural continuation errors only (continuation at column 0 or with nonzero width). Live capture never produces incoherent combos, and Section 11.2 U02-F requires rejection of invalid geometry, which holds. The Section 12 producer-side rule ("reject invalid leading and continuation combinations") is enforced fail-closed at the D01 conversion (`CoveredCellOccupied`, `LeadingCellOverhang` with coordinates). No U02 rework: reopening an accepted task for beyond-spec hardening would churn the open PR for no specified requirement.

## D-QUAL-1: Qualification stack

Downstream tasks D03 onward patch against `goal/qual-stack` in `/Users/donbeave/tuiscotti/repos/upstream-qual` (frozen; never commit task work there): main `f276059` + U05-stack head `a9a412b` (U02, U03, U04, U05 — CORRECTION at V01: U11 was never in this stack; the original record wrongly listed it) + U06 head `4fb316a`, merged as `fb72d00`, plus U07/U08/U09/U10 merges and the `last_output`/`<<<<<<<` fixups (`2482ff5`, `049fb8c`). Stack assembly merges applied cleanly. Verified: `owned_snapshot` 11/11 and `pixel_compare` 5/5 green on the early stack; full downstream `cargo check` green on the final head `049fb8c` (V01: 167/167 nextest green through the patched dep). Disposition: U11 (underline color in shared paint) qualifies upstream on draft PR #283 only; no downstream test depends on its renderer delta (no pixel baselines are committed downstream), so the stack stays without it. Re-check U11 inclusion at R02 when the released dependency version is adopted. Never commit absolute patch paths; per D-GUARD-1 the patch is expressed as untracked task-local `.cargo/config.toml`, never `--config` args (the original direction is superseded).

## D-QUAL-3: Qual-stack reproduction commands

`a9a412b` is the `goal/u05-graphemes` stacked tip (U02→U03→U04→U05 by
branch ancestry, D-STACK-1), not a merge. From upstream `main`
`f276059`, in a new directory:

```sh
git checkout -b goal/qual-stack f2760592ed00e90436f8c3bae124dab712eaecf8
git merge a9a412b  # U02..U05 via stack ancestry
git merge 4fb316aa56c65cc659b4a43ee4921c45e33cecc0  # U06 pixel compare
git merge b0da6225c87d06ea3dc95624172052012ca229f2  # U07 checked input
git merge 6175db32c2a9c3841311813d58d8a339e012b92b  # U08 launch options
git merge 46a2864b934050cb099b6775381664f8b34e8f81  # U09 raw bytes
git merge 1742fba903495948d75fce50b285ac2e4ca0dac4  # U10 waits (resolve: keep U07 input methods + U10 last_output field)
```

Then the two assembly fixups: remove the leftover `<<<<<<<` marker in
`session.rs`, and add the missing `last_output: None` field to the
`spawn_with` `TermState` initializer. Result must equal `049fb8c`.
Downstream builds against the stack only through the untracked
task-local `.cargo/config.toml` patch (D-GUARD-1); upstream checkouts
never build through the guard (committed `[env]` config).

## D-D04-1: Packet errors corrected by spec

The first D04 review returned 3 BLOCKERs + 3 REQUIREDs, several caused by wrong directions in the primary agent's D04 packet. The GOAL text overrules the packet. Corrections for the rework:

- Locks: keep `.tuiscotti-approval.lock` after release; never delete a held lock file; never create anew at a held path. Hold the OS lock (fcntl/BYK; `fs2`/`fd-lock`/`rustix` only if already suitable — no new unsafe) through baseline read, digest comparison, and atomic replacement; release it at transaction step 19. Cross-process exclusion needs a real two-process test, not only threads.
- Approval re-render is mandatory: steps 10-12 resolve the candidate's specified profile, re-render through the upstream renderer, and compare decoded pixels; unknown profile rejects with `UNKNOWN_PROFILE`; pixel mismatch rejects with `INCONSISTENT_SAMPLE`. The "no renderer call" packet direction was wrong, as was "unknown profiles accept".
- Malformed digests map to `INVALID_SAMPLE` (no `INVALID_DIGEST` code exists) with exit code 2; stale digests, CI refusal, and invalid sample data use exit code 3 per Section 16.2.
- No ID cross-comparison when a baseline is present: validate the candidate test ID only on first approval (absent baseline). Restructure accept so a present baseline skips ID/gate comparison.
- Temp files: exclusive creation (`O_EXCL`) with owner-only mode; never follow symlinks.
- Transaction order follows the numbered steps exactly (lock at step 4, before the candidate read).

## D-STACK-1: Dependent PR stacks

U03 and U10 build on the U02 snapshot API, so their worktrees merge the accepted U02 head and record it as the parent PR. U11 and U04 then stack on U03; U05 stacks on U04. Each PR body records its parent. U07 follows U09 in time (same file, distinct regions) but stays based on `main` with no functional dependency.

## D-SIGN-1: Sign-off identity

Upstream history carries `Signed-off-by` on maintainer commits. Task commits get `Signed-off-by: Alexey Zhokhov <alexey@zhokhov.com>`, which matches both the repository git config and the `donbeave` GitHub identity (`gh api user`). No invented identity.

## D-GUARD-1: rust-build-guard compliance path

The standing Rust rule now routes all Cargo through `/Users/donbeave/.local/bin/rust-build-guard -- mise exec -- cargo ...`. The guard hard-denies (exit 125, no override): `cargo test` (only `nextest run/list` allowed), `--config` / `--manifest-path` args, and any applicable Cargo config containing `include` or `env`. Consequences recorded here because they overrule older packets (including D-QUAL-1's "`--config` args" direction):

- All downstream builds/tests run as `rust-build-guard -- mise exec -- cargo nextest run ...` (or `check`/`clippy`/`fmt`), preceded by a `disk-check` probe. Never `cargo test`, never `--config`, never plain `cargo`.
- The tui-test path patch is expressed as a task-local UNTRACKED `repos/downstream/.cargo/config.toml` (`[patch.crates-io] tui-test-rs = { path = ".../upstream-qual/crates/tui-test" }`), which passes the guard scan (no `include`/`env`). Never commit it, never add it to `.gitignore`; remove it at R02 when the released dependency is adopted.
- Upstream checkouts (`repos/upstream*`) can never build via the guard: their committed `.cargo/config.toml` carries an `[env]` table (simdutf backend vars), which the guard rejects categorically. No bypass, no config edits upstream. Upstream merge validation therefore runs compilation-through-downstream-dependency (`cargo check -p tuiscotti` compiles the patched `tui-test-rs`), plus each task branch's own CI on its draft PR. Upstream test suites on merged stacks are re-run only where a guard exception is granted.
- `cargo metadata` / `cargo tree` (needed at D07) are NOT in the guard's supported set (`build/check/clippy/fmt/doc/run/nextest` only); D07 must request a policy ruling or use `nextest`-reachable equivalents.

## D-D06-1: Session versus Engine surface split

GOAL 17 asks for direct `tui_test::Session` use plus the accepted
`spawn_with` + `LaunchOptions`, `mouse_checked`/`paste`/`focus`,
`write_bytes`, and `wait_quiet` methods. Those two requirements land on two
upstream types: the runtime `tui_test::Session` owns lifecycle, upstream
locators, snapshots, `write_bytes`, and quiet waits, while `spawn_with`
with `LaunchOptions` and the checked input methods live only on
`tui-test::Engine` (the runtime `Session` keeps its engine private and
exposes no checked-input path). Downstream migration tests and examples use
`Session` wherever it reaches and `Engine` only for `spawn_with` and
checked input. No tuiscotti session, locator, input, or wait helper remains
in the migrated tests. The live/PTY migration suite is `#![cfg(unix)]`,
following the U09 precedent: Windows CI gives vacuous passage there.

## D-D06-2: Missing baseline root under a symlinked parent

`FrozenStore` documents that a missing baseline root makes every check
`Missing`, but `missing_baseline_path` walked to the first existing
ancestor and demanded it sit inside the root. With a missing root under a
symlinked parent (every macOS tempdir), the walk stopped above the root and
reported `InvalidStorePath` instead. D06 fixes this at the walk: reaching
the baseline root itself with nothing existing below it returns the lexical
path, which is safe because no symlink can redirect a nonexistent path and
the root was resolved by `resolve_root`. D03, D04, and D05 suites re-run
green with the fix; no D03 test changes.

## D-QUAL-2: Qual-stack merge fixups and validation

Assembling `goal/qual-stack` (U07 `b0da622`, U08 `6175db3`, U09 `46a2864`, U10 `1742fba` merges) needed two fixups, both stack-assembly commits, not task work: `2482ff5` removed a leftover `<<<<<<< HEAD` marker in `session.rs`; `049fb8c` added the missing `last_output: None` field to the `spawn_with` `TermState` initializer (U10 added the field; the U08-side initializer was merged without it; E0063 caught by the guard-compliant downstream check). Validation: `cargo check -p tuiscotti --locked --offline` green through the guard (compiles `tui-test-rs` at the stack head). The stack is otherwise frozen; D06+ patch against it.
