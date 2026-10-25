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

## D-STACK-1: Dependent PR stacks

U03 and U10 build on the U02 snapshot API, so their worktrees merge the accepted U02 head and record it as the parent PR. U11 and U04 then stack on U03; U05 stacks on U04. Each PR body records its parent. U07 follows U09 in time (same file, distinct regions) but stays based on `main` with no functional dependency.

## D-SIGN-1: Sign-off identity

Upstream history carries `Signed-off-by` on maintainer commits. Task commits get `Signed-off-by: Alexey Zhokhov <alexey@zhokhov.com>`, which matches both the repository git config and the `donbeave` GitHub identity (`gh api user`). No invented identity.
