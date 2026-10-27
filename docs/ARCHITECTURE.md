# Architecture

Two shipping crates, one workspace, no internal engine.

## Workspace

```text
crates/
  tuiscotti       facade: view, sample, check, approve, report, error, render_profile
  tuiscotti-cli   binary `tuiscotti`: thin Clap parsing + four offline ops (check, accept, render, report)
```

`tuiscotti-cli` → `tuiscotti` is the only internal edge. There are no
feature flags anywhere downstream: pure-view execution is
unconditional, and there is no `[features]` table that could hide
removed code behind a disabled feature.

## Dependency graph (Cargo.toml + Cargo.lock analysis)

`cargo metadata` and `cargo tree` are denied by the Rust guard
(decisions record D-GUARD-1), so the graph below is derived from the
workspace manifests plus the locked package list instead of tool
output. The V01 reviewer re-runs the check with real metadata if
policy permits. Direct and transitive `tui-test` dependencies are
listed separately.

`Cargo.lock`: 278 packages. Workspace-local packages: exactly two —
`tuiscotti` and `tuiscotti-cli`. The snapshot/render/pixel APIs come
from the `tuiscotti-tui-test 0.2.1` registry release (renamed fork;
lib name `tui_test`); no `[patch.crates-io]` entry and no git
dependency remains. No `termpane` package exists anywhere in the graph.

Direct dependencies of `tuiscotti` (normal): `base64`, `fs2`
(approval locking), `image` (PNG decode), `ratatui` (view capture),
`serde`, `serde_json`, `sha2`, `tui-test-rs` (snapshot model,
pinned rasterizer, pixel comparison). Dev-only extra: `tempfile`.

Direct dependencies of `tuiscotti-cli` (normal): `base64`, `clap`,
`image`, `serde`, `serde_json`, `tui-test-rs`, `tuiscotti`.
Dev-only extras: `sha2`, `tempfile`.

Transitive `tui-test` dependencies (direct deps of
`tuiscotti-tui-test`, all registry): `alacritty_terminal`, `anyhow`,
`base64`, `bitflags`, `compact_str`, `dirs`, `flate2`, `fontdb`,
`gif`, `libc`, `png`, `portable-pty` (the permitted PTY path),
`regex`, `rustybuzz`, `serde`, `serde_json`, `sha2`, `tiny-skia`,
`toml`, `ttf-parser`, `unicode-properties`, `unicode-width`,
`windows-sys`. The full closure is the remainder of the packages in
`Cargo.lock`; none of it is a downstream-local engine, renderer, or
adapter.

Removed from the graph at D07: `anyhow`, `crossterm`, `swash`,
`image-compare`, `insta`, `unicode-segmentation`, `unicode-width`,
`termpane` as direct workspace dependencies, and every
`tuiscotti-{core,render,runtime,insta,fixtures,bench}` and `xtask`
package.

## Data flow

```text
PURE VIEW (in-process, no subprocess):
  production Ratatui draw closure + ViewSettings
    ─▶ view::capture_view (TestBackend) ─▶ TerminalSnapshot (upstream)
    ─▶ Sample::with_pixels / Sample::state (tuiscotti.sample.v1)
    ─▶ FrozenStore::check ─▶ pass / missing / different + artifact evidence

APPROVAL (explicit, digest-arbitrated):
  reviewed candidate ─▶ ApprovalStore::accept
    (candidate SHA-256 + expected baseline, CI refused, upstream re-render)
    ─▶ baseline installed atomically ─▶ check passes

LIVE (upstream only, application test code):
  tui_test::Session ─▶ TerminalSnapshot ─▶ same Sample path as pure view
```

## Key types

- `view::ViewSettings` / `ViewCursor` — grid size and explicit
  cursor state for pure-view conversion.
- `sample::Sample` — versioned visual sample (`SAMPLE_SCHEMA`,
  gate, snapshot, render contract, PNG image, observation);
  `candidate_id` is the SHA-256 of the sample bytes.
- `check::FrozenStore` — read-only baselines; `CheckOutcome` is
  `pass` / `missing` / `different` with typed `MismatchReason`.
- `approve::ApprovalStore` — the 20-step transaction; `ApproveError`
  carries CI refusal, digest, profile, render, and I/O failures.
- `render_profile::Profile` — the single pinned profile
  `tuiscotti-pinned-v1` (upstream raster policy + licensed
  downstream font bytes).
- `report` — static HTML index from saved candidate evidence.
- `error::Error` — one typed error over the six modules above.

Public API design: [API.md](API.md). Snapshot semantics live in the
sample module docs (`sample.rs` header). Durable rationale:
[DECISIONS.md](DECISIONS.md) (historical).
