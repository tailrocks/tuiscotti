# Durable decisions

Decisions that must survive refactors. Each names the rule, the
rationale, and where it is pinned in code/tests.

> History note: sections 1, 7, 8, 9, 11 (middle items), and 14
> describe the pre-tui-test-migration product (removed surfaces such
> as `Store`/`GroupedStore`, `cargo insta`, `proto`/`machine`/MCP,
> `tuiscotti.toml`, and the old layered crates). They are retained
> as history; the tree wins on any disagreement. Current contracts
> live in [SNAPSHOTS.md](SNAPSHOTS.md), [CLI.md](CLI.md), and
> [API.md](API.md).

## 1. CI must never auto-accept

> VINTAGE (pre-migration wording kept as history): the rule still
> holds — approval is explicit, workstation-only, and never
> automatic in CI. The current approval surface is
> `ApprovalStore::accept` (library) and `tuiscotti accept` (CLI);
> `FrozenStore` performs the frozen checks; see
> [SNAPSHOTS.md](SNAPSHOTS.md).

There is no bless flag, env var, or accepting CI mode — and there
must never be. Historically, approval was `Store::accept` /
`GroupedStore::accept(_all)` / `tuiscotti accept` (per-name) /
`cargo insta review`, on a workstation only, after reviewing
actuals. A test proves no env var accepts. Rationale: ambient
approval lets a broken renderer bless its own breakage.

## 2. Fail closed everywhere

Missing or corrupt approvals fail the gate; unknown input to a
guard fails rather than passes. Concretes: missing approved PNG is
`missing-approval` (never regenerated in memory — a renderer
upgrade must fail loudly, not re-render the expectation it gates);
interrupted candidate writes are `capture-incomplete` via the
sealed manifest; mouse input without app-enabled reporting is
`ModeNotEnabled`, never silently dropped; `PerceptualPolicy`
rejects NaN/out-of-range thresholds; `#[must_use]` on outcomes
turns dropped gates into warnings.

## 3. Actuals before assertions

Every gate writes reviewable evidence (actual frame/PNG/fidelity,
diff PNG, report) BEFORE the assertion runs, so a failure still
leaves the full story on disk. Timeouts fail WITH the last
screen/observation, not a bare deadline error.

## 4. Exact decoded-pixel gate

The PNG gate compares decoded pixels: re-encoding passes, one
changed channel fails. Review leniency lives only on a validated
perceptual threshold. Rationale: fuzzy-by-default gates hide
renderer regressions; strict-by-default with explicit leniency
keeps every exception visible at the call site.

## 5. Deterministic rendering, no system fonts

Same frame + same profile + same vendored font bytes =
byte-identical PNGs on any machine. The renderer tries styled
primary → regular primary → pinned fallback chain (SHA-256
verified at load; mismatch refuses to render) → deterministic tofu
with correct advance + fidelity record. Offline `frame.json`
re-renders are byte-identical (pinned by tests). Runners need no
system fonts.

## 6. Pure-cargo PTY engine: `termpane` + `termpane-vt`

> SUPERSEDED by the tui-test migration (GOAL, D-R02-1): the terminal
> engine now lives upstream in `tui-test-rs`, consumed through a
> provisional full-SHA `[patch.crates-io]` pin to the maintained
> fork until the upstream registry release carrying the required APIs
> is adopted and the patch is removed. The termpane plan below is
> retained as history and no longer describes the tree.

The PTY engine is released `termpane` `0.1` (PTY transport, process
management, and damage-grid shadow terminal surface) over
`termpane-vt` `0.1` (ECMA-48 / VT parser) from crates.io — no
git/path deps, no patches, no vendored forks. Replaces, in order:
the vendored `vt100` fork, the interim `portable-pty` +
`alacritty_terminal` bridge, and the earlier unreleased `termpane`
git prototype. Upstream `vt100` remains a dev-only differential
oracle in `tuiscotti-fixtures` (never in normal runtime deps).
The adapter owns input encoding (`tui/encode.rs`, `tui/encode_key.rs`)
and drains emulator passthrough replies (`tui/capture.rs`); that
behavior is qualified by `tool_qualification` + PTY matrix tests.
Known backend gaps (`O5`, `O6`, `O7`, `O13`, silent unknown
sequences, no scrollback reflow) are documented in
[LIMITATIONS.md](LIMITATIONS.md), not hidden.

## 7. Typed op protocol over stdio, no daemon

> VINTAGE: the `proto`/`machine`/MCP surface below was removed by
> the tui-test migration; retained as history.

Agents drive `proto::{Op, execute}` / `tuiscotti machine`: 15 typed
ops, JSON envelopes, versioned protocol (`2.0.0`) with a printed
schema (`tuiscotti schema`). Named sessions use versioned endpoint
files in an owner-only runtime dir — no daemon process, no sockets
to leak. The MCP bridge (`mcp::serve`) speaks the same ops.

## 8. Config responsibilities are split by owner

> VINTAGE: `tuiscotti.toml`, `tuiscotti init`, and Insta config no
> longer exist; retained as history. `.config/nextest.toml`
> (scheduling only, never parsed here) is still current.

`tuiscotti.toml` (capture + assertion policy, owned by tuiscotti) vs
`.config/nextest.toml` (scheduling only, owned by cargo-nextest,
never parsed here) vs Insta config (review behaviour, owned by
Insta). Printed by `tuiscotti init` and pinned in `proto::CONFIG_DOCS`.
Rationale: each tool owns its file; no cross-parsing, no drift.

## 9. Layering: pure core, thin edges

> VINTAGE: the `tuiscotti-core`/`tuiscotti-runtime` crate split
> below no longer exists (two shipping crates remain); retained as
> history. The current layering is in
> [ARCHITECTURE.md](ARCHITECTURE.md).

`tuiscotti-core` is pure (no rendering, PTY, or I/O) and every
crate depends on it; `tuiscotti` is re-exports only; `tuiscotti`
`main.rs` is arg parsing over the facade. Only `tuiscotti-runtime`
may spawn. New public API goes in the owning leaf crate and is
re-exported. Rationale: the dependency graph in ARCHITECTURE.md is
a reviewable contract — a new edge is a design decision, not an
accident.

## 10. No legacy code

Migrations finish: old paths are removed completely, never shimmed
or aliased. Breaking changes are preferred over compatibility
layers. This file, MIGRATION.md, and git history are the record —
not the source tree.

## 11. Superseded ledger requirements (old 82-item ledger retired)

The old `docs/REDESIGN-LEDGER.md` (82 items, frozen at `d6b2574`) was
retired with the crates/ restructure; full text survives in git history
(e.g. `git show 2509bbf:docs/REDESIGN-LEDGER.md`). Disposition:

- SUPERSEDED — A09 (thin JS/TS + Python clients): foreign-language SDKs
  are out of scope. Clients, manifests, and client tests were removed;
  transport stays Rust-only (`tuiscotti` + `tuiscotti machine`).
- COMPLETED (G1 landed) — R04 (registry `termpane` + `termpane-vt`
  backend): `tuiscotti-runtime` uses released `termpane` `0.1` +
  `termpane-vt` `0.1` from crates.io as its sole PTY + terminal-state
  backend per decision 6 above; `vt100` is a dev-only differential
  oracle in `tuiscotti-fixtures`.
- MOOT (G1 landed) — R05 (Ghostty binding qualification): moot now
  that the backend boundary is `termpane` + `termpane-vt`.
- RETAINED (re-mechanized) — M09 (pure-view builds without PTY/native
  deps): still required as workspace policy (pure view/screenshot
  configuration must not compile the terminal runtime); enforced via
  crate layering and feature isolation instead of the old root
  `pty` feature.

Every other still-relevant correctness, three-mode testing, fidelity,
lifecycle, Insta, nextest, and Rust/CLI requirement from the old ledger
is retained and covered by the current suite; nothing was deleted merely
because the ledger file was removed.

Historical studies (`docs/packets/IMPLEMENTATION-GOAL.md`,
`docs/packets/REFERENCE-SPEC.md`, frozen 2026-09-29 at `7e8272b`)
are context only: when they disagree with the tree, the tree wins.
They are not updated; new decisions land here.

## 12. Name collision check: `tuiscotti` (2026-09-29)

No blocking collision found; nothing reserved; no trademark
clearance claimed. Checks performed: crates.io API lookup for
`tuiscotti` → 404; `cargo search` → empty; GitHub
`tailrocks/tuiscotti` → 404; local `PATH` → absent; web search →
no software usage of the name. Follow-on constraint: the hosted
repo rename and crates.io publish need human admin/auth, so they
stay manual steps outside any agent task.

## 13. Path sanitization in provenance and candidate seals

Committed and reviewable JSON artifacts (`Frame.provenance.argv`,
`Manifest.profile.font_file`, and verdict/report links) sanitize
filesystem paths to basenames (or relative store paths) before
serialization so workstation-specific absolute paths (`/Users/...`,
`/home/...`, temp directories) never leak into git-tracked snapshots
or portable evidence bundles.

## 14. Downstream 7,550-scenario feedback decisions (P1–P8)

> VINTAGE: the P1–P8 items below describe the removed pre-migration
> product (key parsing, grouped stores, locators, `WaitError`,
> rasterizer internals); retained as history, not current API.

Findings from running a 7,550-scenario downstream Ratatui visual
suite (`ratatui` `0.30`, `crossterm` `0.29`, 4 themes × 2 sizes ×
multiple locales) drove eight durable design refinements:

- **P1 — Bare `'f'`/`'F'` key parsing**: `parse_chord("f")` and
  `Key::from_str("f")` parse as `Key::Char('f')`, while uppercase
  `"F"` parses as `Key::Char('F')` (matching every other single
  ASCII letter); function keys require explicit digits (`"F1"`..`"F12"`).
- **P2 — SemVer caret ranges (`ratatui 0.30`, `crossterm 0.29`)**:
  workspace dependencies use standard SemVer caret ranges (`Cargo.lock`
  pins exact resolution for reproducible CI), `tuiscotti-core`'s
  `ratatui` dependency targets `0.30`, and `crossterm` is removed from
  `tuiscotti-core`.
  - *Alternatives weighed*:
    1. **(a) Standard SemVer caret ranges (`^`) on `ratatui = "0.30"` /
       `crossterm = "0.29"` with `crossterm` dropped from `tuiscotti-core`
       [chosen]**: downstream crates on current `ratatui 0.30` unify on a
       single `ratatui`, `crossterm`, and `unicode-width` in `Cargo.lock`
       with zero duplicate-crate `Buffer`/`Terminal` type mismatches and a
       single `Frame` capture surface.
    2. **(b) Per-major-version Cargo feature flags (`ratatui-029`,
       `ratatui-030`) [rejected]**: violates Decision 10 (no legacy shims
       or dual-version compatibility layers) and doubles the `ratatui`
       adapter surface.
    3. **(c) Separate `tuiscotti-ratatui` adapter crate [rejected]**:
       splits the pure-view capture entrypoint across another crate
       boundary without removing the version pin problem.
- **P3 — Minimal `.ansi`-only grouped store (`ArtifactSet::AnsiCursor`)**:
  `GroupedStore::with_artifacts(ArtifactSet::AnsiCursor)` commits only
  `<name>.ansi` (via `render::ansi_dump_with_cursor`, which appends a
  canonical VT cursor trailer `ESC[?25l` or
  `ESC[?25h ESC[<y>;<x>H ESC[<Ps> q`). `formats::ansi::frame_from_ansi`
  inverts `.ansi` back to a validated `Frame` so expected/actual
  `.png` + `.html` and `.diff.png` are rendered on demand into
  `actual_root`, `expected_root`, and `diff_root` only when a check
  fails or approval is missing — cutting passing-run I/O and repo
  footprint to 1 file per scenario while keeping full visual diff
  artifacts on failure.
  - *Alternatives weighed*:
    1. **(a) Closed `ArtifactSet` enum (`All` default, `AnsiCursor`
       minimal mode) on `GroupedStore` [chosen]**: makes invalid artifact
       combinations unrepresentable, keeps fail-closed manifest/seal
       schemas explicit per mode, and preserves one `GroupedStore` API.
    2. **(b) Per-artifact boolean builder flags (`with_png(bool)`,
       `with_html(bool)`, `with_txt(bool)`) [rejected]**: permits invalid
       combinations (e.g. disabling all gates or writing `.html` without
       the `.png` it references) and explodes the state space for
       `verify_candidate` and `accept`.
    3. **(c) Pluggable gate trait or separate store struct [rejected]**:
       fragments CLI auto-detection (`tuiscotti accept --artifacts`) and
       duplicates candidate sealing, `run_id` freshness, and report logic.
- **P4 — Multi-match `BoundLocator` policies (`MatchPolicy`)**:
  `BoundLocator` stays fail-closed (`MatchPolicy::Unique`) by default
  and adds explicit `.first()`, `.last()`, `.nth(i)`, `.any()`,
  `.count_at_least(n)` (assertion-only; `.click()` rejected with
  `ActionError::Locate(LocateError::Usage(..))`),
  `.expect_count_at_least(n)`, and `.expect_count_at_least_within(n, d)`
  combinators so callers never drop down to detached `Locator::resolve`
  loops when waiting on repeated UI chrome (`Locator::count_at_least`
  was removed from unbound `Locator` in favor of
  `Locator::expect_count_at_least` and `BoundLocator::count_at_least`).
- **P5 — Bounded screen dump in `WaitError` `Display`**: `WaitError`'s
  `Display` implementation appends the revision, grid dimensions, and a
  bounded plain-text screen dump (up to 40 non-empty trailing-trimmed
  rows / 4 KiB, or `<blank screen>`) so `unwrap()` / `?` failures in CI
  logs show what was on screen immediately.
- **P6 — Mismatch-only rasterization and full-resolution pixel verdicts**:
  passing checks in `AnsiCursor` mode compare normalized `.ansi` bytes
  in memory and skip `swash` PNG/HTML rasterization entirely (~0.37 ms
  per passing gate at 120×40), while mismatch rendering uses slice-based
  glyph blending, fast diff PNG encoding, and full-resolution
  `PixelVerdict::score`.
  - *Alternatives weighed*:
    1. **(a) Skip rasterization on pass in `AnsiCursor` mode + slice-based
       glyph/diff loops + `CompressionType::Fast` / `FilterType::Sub` diff
       PNG encoding + consumer `[profile.dev.package."*"] opt-level = 3`,
       keeping full-resolution 1:1 `PixelVerdict::score` [chosen]**:
       cuts passing-gate cost in `AnsiCursor` mode to ~0.37 ms and keeps
       mismatch checks at `35–70 ms` under `opt-level = 3` without
       changing `PixelVerdict::score` or `PerceptualPolicy::allows`
       outcomes.
    2. **(b) Downsampling `hybrid_score` inside `compare_png` for frames
       above `320×240` [rejected]**: changes `PixelVerdict::score` and
       therefore alters `pixel_threshold` and `PerceptualPolicy::allows`
       pass/fail decisions in default (`ArtifactSet::All`) and classic
       stores.
    3. **(c) Replacing the `swash` rasterizer with `ab_glyph` or
       `fontdue` [rejected]**: breaks byte-identical PNG determinism and
       glyph shaping/fallback fidelity (`SHA256SUMS`).
- **P7 — Scoped CLI `accept` / `review --filter` + `.ansi` rendering**:
  `tuiscotti accept` auto-detects classic vs grouped stores, accepts
  exact names, segment-aware prefixes, or `*` (single-segment) / `**`
  (multi-segment) / `?` globs (`GroupedStore::accept_matching`),
  enforces candidate manifest sealing and `run_id` freshness, refuses in
  `CI`, supports `--actual` and `--artifacts all|ansi-cursor`, and fails
  loudly on zero matches. `tuiscotti review` adds `--filter <pattern>`,
  and `tuiscotti render --input <file.ansi> --out <prefix>` renders
  standalone `.ansi` captures to `png`, `html`, `svg`, `txt`, `json`,
  `ansi`, or `ansi-cursor`.
- **P8 — Explicit DEC mouse mode contract & PTY clock guidance**:
  mouse methods document their exact DEC tracking (`1000`/`1002`/`1003`)
  and SGR encoding (`1006`) requirements and error strings, and PTY
  subprocess tests document the two-part deterministic clock/tick
  environment + semantic-wait pattern ([LIMITATIONS.md](LIMITATIONS.md),
  [TESTING.md](TESTING.md)).

## 15. cargo-deny advisory ignores and the registry fork release

`deny.toml` carries advisory ignores `RUSTSEC-2026-0206` (`rustybuzz`
unmaintained; transitive via the `tuiscotti-tui-test` registry release,
no safe upgrade, ecosystem successor is `skrifa`) and `RUSTSEC-2026-0192`
(`ttf-parser` unmaintained, same migration path via fontdb/rustybuzz).
Remove both when shaping migrates off rustybuzz. No git source is
permitted at all (`deny.toml` `allow-git` is empty; the former
`[patch.crates-io]` fork pin was removed when `tuiscotti-tui-test`
0.2.0 shipped on crates.io from `tailrocks/tui-test`); source validation
stays deny-by-default. Rationale lives here because `deny.toml`
contains no prose comments: cargo-deny consumes only the IDs and URLs.
`MPL-2.0` is allowed solely for `option-ext` 0.2.0, pulled transitively
via `dirs-sys <- dirs <- tuiscotti-tui-test` (unavoidable upstream dep
of the registry fork release). Retire this entry only if the upstream
dependency graph drops `option-ext` and `cargo deny check licenses`
stays green without it.

## 16. Local-only mise gates

The `ci-*` tasks in `mise.toml` never run in generated CI: schema 2
`.velnor/config.toml` has no custom-task hook, and generated CI covers
fmt, clippy, nextest, doctests, and doc through its own
compile_driver/test_runner. The tasks exist for local pre-release
verification (`ci-msrv` is the only MSRV vehicle). They run online
without `--offline` because fresh CI runners have no cargo cache, and
every command in them is verified locally before commit.

## 17. Exact toolchain and helper pins

`mise.toml` pins every tool exact, never floating: `mr-boxington`,
`rust` 1.99.1 with `clippy`/`rustfmt` components and both Linux targets,
`alint`, and `cargo-deny` as upstream release binaries (same versions
the old source builds used, but CI-fresh-runner installs download
binaries instead of compiling). The `rust` 1.99.1 pin stays consistent
with `rust-toolchain.toml`. Root `mise.toml` matches the Renovate
`mise` manager file patterns and the Velnor Rust-unit watch list.

## 18. Binary sample persistence; Zstandard level 22 provisional

The current sample API writes and reads versioned Protobuf messages in
Zstandard-compressed `.tui.pb` containers. The reader rejects JSON
samples without converting them; result records and CLI output remain
JSON. This implementation contract is covered by binary reader and CLI
integration tests and described in [SNAPSHOTS.md](SNAPSHOTS.md).

The public sample writer provisionally uses Zstandard level 22 with a
32 MiB window ceiling, content size and frame checksum enabled, dictionary
ID disabled, and zero workers. The level is the maximum regular level in
the pinned vendored Zstandard 1.5.7 build. This follows five release-mode
real-corpus runs that passed the 1 GiB per-process RSS, 60-minute per-pass,
and 72-hour cumulative caps with substantial margin. The level is a
provisional implementation choice, not release approval or final
cross-platform proof.

An independently reviewed exact-head follow-up at
`9e8eab42e072936824186bbaa17919181410a6e9` exercised the public writer and
reader on 20 real captures across five release-mode processes. All 100 output
files matched the preserved outputs, and every process stayed within the
absolute RSS/time limits. It establishes exact-head resource feasibility and
output identity for that corpus. Its process timing includes output writes;
the harness's per-record timing field stops before writing and is not an
end-to-end measurement. The heavily contended host makes timing unsuitable
for codec ranking. The harness and inputs are local untracked evidence, so a
fresh clone cannot reproduce the result without restoring them.

A real downstream-history storage experiment used 105 unapproved candidate
snapshots: 104 replacements, one new PB file, and five retained JSON files.
The measured repacked history grew by 292,207 bytes; its incremental thin pack
was 286,339 bytes, and a fresh PB-only pack was 286,025 bytes. This is a
single-snapshot storage measurement, not a full multi-revision migration or
proof of equivalence to historical JSON content. The inputs and results are
also local untracked evidence. Neither follow-up is release approval or
cross-platform proof.

The run was made under sustained unrelated host load after the original
benchmark freeze. The later contended-run amendment permits provisional
selection only from absolute cap results; the run cannot support relative
speed rankings. Evidence is native macOS/aarch64 only. Linux behavior and
broader workloads remain unverified, including accept and check over the full
capture corpus, report generation over realistic non-empty evidence, and
workloads beyond the measured 20 captures. Details,
comparisons, hashes, and reproduction commands are in
[the provisional level 22 benchmark record](benchmarks/zstd-level-22-provisional.md).
Before release, complete the remaining format and resource checks in
[COMPRESSED-BASELINES.md](COMPRESSED-BASELINES.md) and the cross-platform
evidence described by the benchmark record.

A read-only audit of the inventoried downstream legacy corpora examined
2,608 path occurrences: 2,600 valid state samples and eight deliberately
invalid coverage fixtures. A field-based upper bound for every valid sample's
Protobuf message is 5,641,690 bytes. For the pinned encoder's no-flush
streaming path, the independently reviewed conservative container bound is
5,789,892 bytes, below the 64 MiB limit. This is evidence for those enumerated
corpora, not for arbitrary future samples; pixel-gated legacy samples were not
present in that inventory. The audit did not convert or approve baselines.
The [legacy-cap evidence archive](../benchmark-evidence/legacy-cap-results-20261106.tar.gz)
(SHA-256 `a618c6e81be9968b8b2a7680d36751feaeb87c97ce01a914cccbc9bf30f9e1a1`)
preserves the manifests, per-sample bounds, validation results, and counter source.

The CLI follow-up measured merged Tuiscotti commit
`6d01628170de8bed3f00f52713f3a510b782e41e` (the then-main commit) in five full
check passes and five full valid-accept passes over the 20 paired benchmark
captures (10 state and 10 state-and-pixels). All 100 checks and all 100 accepts
passed. Maximum child RSS was 58,736,640 bytes for check and 171,540,480 bytes
for accept; median full-pass wall times were 0.598698 s and 1.946359 s
respectively. Accepts wrote only byte-identical files in a disposable baseline
tree with `CI` unset. This closes those CLI feasibility measurements for the
20 captures; it is not a full downstream-corpus result or a build from the
current main head. The evidence bundle and limitations are recorded in
[the level 22 benchmark record](benchmarks/zstd-level-22-provisional.md).

A separate actual TCB mainline history measurement spans the merged binary
migration and one subsequent PB baseline update. It records real full-history
and thin-pack sizes, but the latter commit also adds protected historical-root
baselines, acceptance receipts and test material. It is not a baseline-only
Git cost or a JSON-versus-PB comparison for equivalent updates. See
[the TCB mainline history record](benchmarks/real-tcb-mainline-history.md).

## 19. Dependency refresh and prior pixel round-trip evidence

The pixel round-trip replay used an isolated source snapshot based on commit `b0276ae7b99792d13d2abbd77bfd4fd6633f904f`, plus captured working diff SHA-256 `ffd7ca554fc19dcab0d95e045472378251052b3ec92dc4236d0b570345a9b972` (snapshot tree `a2021ba123bd0eef0b1d20a63e237b541f1c0d63`). These are the recorded replay identities. The snapshot's `Cargo.toml`, `Cargo.lock`, `crates/schema-gen/Cargo.toml`, and `crates/schema-gen/Cargo.lock` SHA-256 values are respectively `983e16c3fa426fb6aa1c7d7a3540bd01c17a3e6736000c8e7578a80f6781b2fc`, `8224ecf03f296a5bba1e79474e56ca664eded69b530bc79670d7e4857e61707a`, `97dc7bb4671e11d0584cb42c5f6f4fa5ef3af157b6cf13b0aa83074bb0eea381`, and `a5dab25a21e903d3dcd87653a8fb5a640840178ce2119919774b9145de0e2a59`. These four hashes also match the files at commit `503ace94a8303b94d55ba9d90965d86b76353a52`; the replay source itself remains the distinct recorded `b0276ae`-based snapshot. That graph selected [toml 1.2.0](https://crates.io/crates/toml/1.2.0), and the standalone schema generator pinned toml 1.2.0. Test-only random sampling used rand 0.10's `RngExt`. The old manifest directly requested miniz_oxide 0.8, though its lock also contained 0.9.1 transitively.

On that exact replay snapshot and aligned audit-harness lock, exact round-trip parity passed for 20 frozen captures on native macOS arm64: 10 state-only and 10 state-and-pixels. The independent verifier passed 140 checks, including state digests, PNG bytes and decoded RGBA hashes, against archived captures and preserved outputs. The normal/build dependency graph matched byte-for-byte between the workspace and audit harness (137 package names). This is evidence for the recorded replay snapshot and dependency graph; it does not establish parity or dependency-graph identity after the refresh. It also does not cover x86, fresh terminal rendering, or cross-platform determinism. No captures or baselines were written. The local report and reproduction artifacts are at `/private/tmp/tuiscotti-roundtrip-pixel-current-head-b0276ae-deps/REPORT.md`; they are not present in a fresh clone.

The dependency refresh changes the workspace toml resolution to 1.2.1 and changes the direct miniz_oxide requirement to 0.9, locked at 0.9.1; `crates/schema-gen/Cargo.toml` pins toml exactly to 1.2.1. The pinned alint tool is 0.22.0. The historical pixel results above have not been rerun against this refreshed graph.

## 20. Velnor Actions 0.1.78 and the Ubuntu 26.04 runner floor

The repository pins Velnor Actions 0.1.78 in
`.velnor/release-manifest.json`. Generated workflow and action files must
come from that release and `.velnor/config.toml`; do not hand-edit generated
outputs. This release emits `.github/workflows/state-seal.yml` as a separate
workflow triggered by pushes to `main`, alongside normal CI. It holds the
source, state, and tool cache producer and seal jobs.

Every Velnor job must use `ubuntu-26.04`; `ubuntu-24.04` is not an allowed
fallback or alternate runner. Keep the hosted profile on `ubuntu-26.04` and
the scale-set profile on its configured Ubuntu 26.04 runner, and review all
generated workflow runner declarations after regeneration.

Protected-main run [42595554573](https://github.com/tailrocks/tuiscotti/actions/runs/42595554573)
completed with an overall `success` conclusion, but its `State producer`
job [129087406972](https://github.com/tailrocks/tuiscotti/actions/runs/42595554573/job/129087406972)
failed with `task_not_in_plan_for_job`. Its state payload publication steps
were skipped, as was the `State seal / root` job
[129087705705](https://github.com/tailrocks/tuiscotti/actions/runs/42595554573/job/129087705705).
The overall workflow conclusion does not verify cache production or sealing.

Velnor Actions PR [#561](https://github.com/tailrocks/velnor-new/pull/561)
merged the producer-execution fix as
[f724e78](https://github.com/tailrocks/velnor-new/commit/f724e78ca617ecd7d06515c07d2753de1f05e2ce).
The pinned stable [Velnor 0.1.78 release](https://github.com/tailrocks/velnor-new/releases/tag/v0.1.78)
was built from `ff0f3ce3e7f87c172dbfc3b4ff9640f82c5a6905`; GitHub's
[commit comparison](https://github.com/tailrocks/velnor-new/compare/ff0f3ce3e7f87c172dbfc3b4ff9640f82c5a6905...f724e78ca617ecd7d06515c07d2753de1f05e2ce)
places the fix after that release. The fix is therefore absent from the
latest stable generator. Keep the state-seal path unresolved until a stable
release containing the fix is published and its manifest and assets are
verified. Do not pin an unpublished or unverified generator build, and do
not publish a Tuiscotti package release on the assumption that this path
works.
