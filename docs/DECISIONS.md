# Durable decisions

Decisions that must survive refactors. Each names the rule, the
rationale, and where it is pinned in code/tests.

## 1. CI must never auto-accept

There is no bless flag, env var, or accepting CI mode — and there
must never be. Approval is `Store::accept` /
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

Agents drive `proto::{Op, execute}` / `tuiscotti machine`: 15 typed
ops, JSON envelopes, versioned protocol (`2.0.0`) with a printed
schema (`tuiscotti schema`). Named sessions use versioned endpoint
files in an owner-only runtime dir — no daemon process, no sockets
to leak. The MCP bridge (`mcp::serve`) speaks the same ops.

## 8. Config responsibilities are split by owner

`tuiscotti.toml` (capture + assertion policy, owned by tuiscotti) vs
`.config/nextest.toml` (scheduling only, owned by cargo-nextest,
never parsed here) vs Insta config (review behaviour, owned by
Insta). Printed by `tuiscotti init` and pinned in `proto::CONFIG_DOCS`.
Rationale: each tool owns its file; no cross-parsing, no drift.

## 9. Layering: pure core, thin edges

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

Findings from running a 7,550-scenario downstream Ratatui visual
suite (`ratatui` `0.30`, `crossterm` `0.29`, 4 themes × 2 sizes ×
multiple locales) drove eight durable design refinements:

- **P1 — Bare `'f'`/`'F'` key parsing**: `parse_chord("f")` and
  `Key::from_str("f")` parse as `Key::Char('f')`, while uppercase
  `"F"` parses as `Key::Char('F')` (matching every other single
  ASCII letter); function keys require explicit digits (`"F1"`..`"F12"`).
- **P2 — SemVer caret ranges + multi-version Ratatui (`0.29..0.31`)**:
  workspace dependencies use standard SemVer caret ranges (`Cargo.lock`
  pins exact resolution for reproducible CI), and `tuiscotti-core`'s
  `ratatui` dependency accepts `>=0.29, <0.31` so downstream crates on
  either `ratatui 0.29` or `ratatui 0.30` share one `Buffer`/`Terminal`
  adapter without duplicate-crate type mismatches.
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
- **P4 — Multi-match `BoundLocator` policies (`MatchPolicy`)**:
  `BoundLocator` stays fail-closed (`MatchPolicy::Unique`) by default
  and adds explicit `.first()`, `.last()`, `.nth(i)`, `.any()`,
  `.count_at_least(n)`, `.expect_count_at_least(n)`, and
  `.expect_count_at_least_within(n, d)`
  combinators so callers never drop down to detached `Locator::resolve`
  loops when waiting on repeated UI chrome.
- **P5 — Bounded screen dump in `WaitError` `Display`**: `WaitError`'s
  `Display` implementation appends the revision, grid dimensions, and a
  bounded plain-text screen dump (up to 40 non-empty trailing-trimmed
  rows / 4 KiB, or `<blank screen>`) so `unwrap()` / `?` failures in CI
  logs show what was on screen immediately.
- **P6 — Mismatch-only rasterization in `ArtifactSet::AnsiCursor`**:
  passing checks in `AnsiCursor` mode compare normalized `.ansi` bytes
  in memory and skip `swash` PNG/HTML rasterization entirely (~0.37 ms
  per passing gate at 120×40).
- **P7 — Scoped CLI `accept` / `review --filter` + `.ansi` rendering**:
  `tuiscotti accept` auto-detects classic vs grouped stores, accepts
  exact names, prefixes, or `*`/`**`/`?` globs (`GroupedStore::accept_matching`),
  supports `--actual` and `--artifacts all|ansi-cursor`, and fails
  loudly on zero matches. `tuiscotti review` adds `--filter <pattern>`,
  and `tuiscotti render --input <file.ansi>` renders standalone `.ansi`
  captures to `png`, `html`, `svg`, `txt`, `json`, `ansi`, or
  `ansi-cursor`.
- **P8 — Explicit DEC mouse mode contract & PTY clock guidance**:
  mouse methods document their exact DEC tracking (`1000`/`1002`/`1003`)
  and SGR encoding (`1006`) requirements and error strings, and PTY
  subprocess tests document the two-part deterministic clock/tick
  environment + semantic-wait pattern ([LIMITATIONS.md](LIMITATIONS.md),
  [TESTING.md](TESTING.md)).
