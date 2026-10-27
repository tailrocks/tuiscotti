# tuiscotti — offline visual-regression workflow for Ratatui views

tuiscotti is for Rust developers who render terminal UI with Ratatui
and want cell-exact regression gates on what the user would see.
It covers the offline half of visual testing: capture a view,
freeze it as a versioned sample, compare it against an approved
baseline, review differences, and approve new baselines explicitly.
Live terminal work (child processes, PTY sessions, input, locators,
waits, capture, rendering, pixel comparison) belongs to upstream
`tui-test`; application tests call `tui_test::Session` directly.
tuiscotti never spawns a process and never opens a terminal.

## Supported workflow

```text
capture -> sample -> check -> inspect -> accept -> recheck
```

1. **capture** — render the production Ratatui view in-process with
   `tuiscotti::view::capture_view` (no shell, no child, no PTY).
2. **sample** — bind the snapshot as a versioned sample
   (`tuiscotti.sample.v1`) with `Sample::with_pixels` or
   `Sample::state`.
3. **check** — run `tuiscotti check --sample <file>
   --baselines <dir> --artifacts <dir>` (or `FrozenStore::check`).
   A pass exits 0. A missing baseline reports `missing`; a changed
   sample reports `different` with a typed reason. Both exit 1 and
   write candidate evidence into the artifact root. Baselines are
   never written by a check.
4. **inspect** — read the candidate evidence: `tuiscotti render
   --sample <file> --out <file.png|html>` and `tuiscotti report
   --artifacts <dir> --out <file.html>`.
5. **accept** — approve one candidate with both digests:
   `tuiscotti accept --candidate <file> --candidate-sha256 <sha>
   --expect-baseline <sha|missing> --baselines <dir>`. Approval
   re-renders image samples through the upstream renderer and
   installs the baseline atomically, or rejects with a typed code
   and writes nothing. Approval refuses before any write when the
   `CI` variable is set, in any form; there is no bless flag and no
   override. See [docs/SNAPSHOTS.md](docs/SNAPSHOTS.md).
6. **recheck** — run the check again; it passes against the new
   baseline.

A changed sample never self-heals: only an explicit `accept` with
matching digests changes a baseline.

## Library and CLI capabilities

Two crates, one offline binary:

| Crate | Provides |
|---|---|
| `tuiscotti` | Pure-view capture (`view`), versioned samples (`sample`), frozen checks (`check`), explicit approval (`approve`), offline rendering (`render`), static reports (`report`), one pinned render profile (`render_profile`), one typed error (`error`) |
| `tuiscotti-cli` | Offline binary `tuiscotti` with exactly four commands: `check`, `accept`, `render`, `report` |

The full Rust surface is listed in [docs/API.md](docs/API.md).
The command grammar is transcribed in
[docs/CLI.md](docs/CLI.md).

## Relation to `tui-test`

Upstream `tui-test` (`tui-test-rs`) owns launching terminal
sessions, sending input, locators, waits, live capture, rendering,
and pixel comparison. Application test code uses its Rust API
(`tui_test::Session`) directly; tuiscotti does not wrap or
re-export it. tuiscotti consumes upstream snapshots
(`TerminalSnapshot`), converts Ratatui views into them, and runs
the offline compare-and-approve workflow on the result. The
dependency is the `tuiscotti-tui-test 0.2.1` registry release
(renamed fork carrying the required snapshot and offline-render
APIs; lib name `tui_test`); see
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and
[docs/LIMITATIONS.md](docs/LIMITATIONS.md). How the two sides split
is summarized in [docs/COMPARISON.md](docs/COMPARISON.md).

## Install and run today

Requires Rust 1.99.1 (`rust-toolchain.toml`; `cargo` on PATH may be
a cache shim, behavior is identical).

```sh
cargo fetch --locked        # warm the registry for offline builds
cargo build --locked --offline
cargo nextest run --workspace --locked --offline   # full suite
```

Use the CLI from the workspace build:

```sh
cargo run -q -p tuiscotti-cli --locked --offline -- \
  check --sample <file> --baselines <dir> --artifacts <dir>
```

Pure-view test in your own crate (needs `tuiscotti` plus
`ratatui 0.30`):

```rust
use ratatui::widgets::Paragraph;
use tuiscotti::check::FrozenStore;
use tuiscotti::render_profile::{PROFILE_NAME, platform_key, resolve_profile};
use tuiscotti::sample::{Observation, Sample};
use tuiscotti::view::{ViewSettings, capture_view};

let snapshot = capture_view(80, 24, &ViewSettings::builtin(), |frame| {
    frame.render_widget(Paragraph::new("hello"), frame.area());
})?;
let profile = resolve_profile(PROFILE_NAME)?;
let sample = Sample::with_pixels(
    &format!("views/hello/pixels-{}", platform_key()),
    &snapshot,
    Observation::empty(),
    profile,
)?;
let root = tempfile::tempdir()?;
let store = FrozenStore::new(
    &root.path().join("baselines"),
    &root.path().join("artifacts"),
)?;
assert!(!store.check(&sample.to_bytes()?)?.is_pass());
```

The `functional` suite in `crates/tuiscotti-cli/tests/` drives the
real binary through check, accept, render, and report end to end.
The maintained example in `crates/tuiscotti/examples/hello.rs`
renders one production-style view and saves it as a versioned
sample (`cargo run -p tuiscotti --example hello -- <out.tui.json>
[--pixels]`); the full first run is walked through in
[docs/FIRST-SUCCESS.md](docs/FIRST-SUCCESS.md).

## Offline means offline

`check`, `accept`, `render`, and `report` never spawn a child, open
a terminal device, or touch the network. Samples are read from
files with bounded reads (64 MiB). "Transitive dependencies" is a
different statement: the build closure includes
`tuiscotti-tui-test` and its registry dependencies (PTY, font, image
crates), because the offline renderer and the snapshot model live
there. Those crates execute only when application test code opens a
live session or when approval re-renders an image sample; the four
CLI commands themselves perform no live work.

## Docs

- [docs/FIRST-SUCCESS.md](docs/FIRST-SUCCESS.md) — first-success tutorial with observed outputs
- [docs/SNAPSHOTS.md](docs/SNAPSHOTS.md) — sample, check, and approval semantics
- [docs/CLI.md](docs/CLI.md) — command reference
- [docs/API.md](docs/API.md) — Rust API surface
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — crates, dependencies, data flow
- [docs/LIMITATIONS.md](docs/LIMITATIONS.md) — fonts, platform limits, unsupported features
- [docs/COMPARISON.md](docs/COMPARISON.md) — how tuiscotti and upstream `tui-test` split the work
- [docs/TESTING.md](docs/TESTING.md) — test suite and CI wiring
- [docs/MIGRATION.md](docs/MIGRATION.md) — upgrade notes: renames, layout, schemas, removed surfaces
- [docs/migration/](docs/migration/) — full change history (superseded designs, review records)

## Contributing and license

See [CONTRIBUTING.md](CONTRIBUTING.md) for build, test, and doc
rules. Apache-2.0 — see [LICENSE](LICENSE). Bundled fonts keep
their own licenses under `crates/tuiscotti/fonts/`.
