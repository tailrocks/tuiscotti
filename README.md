# tuiscotti — offline visual-regression workflow for Ratatui views

Two shipping crates. One executable. No live engine, no renderer, no
adapter, no daemon downstream.

| Crate | Function |
|---|---|
| `tuiscotti` | Pure-view capture (`view`), versioned samples (`sample`), frozen checks (`check`), explicit approval (`approve`), static reports (`report`), one typed error (`error`), one pinned render profile (`render_profile`) |
| `tuiscotti-cli` | Offline binary `tuiscotti`: exactly four commands — `check`, `accept`, `render`, `report` |

Live terminal work (child process, PTY, input, locators, waits,
capture, rendering, pixel comparison) belongs to upstream `tui-test`.
Application tests use `tui_test::Session` directly. There is no
tuiscotti session, locator engine, input API, operation enum, or
re-export of the upstream API.

```text
production Ratatui draw closure ──▶ capture_view ──▶ TerminalSnapshot
    ──▶ Sample (tuiscotti.sample.v1) ──▶ FrozenStore::check ──▶ pass / missing / different
    ──▶ ApprovalStore::accept (explicit digests, refuses under CI) ──▶ baseline
    ──▶ tuiscotti report ──▶ static HTML evidence
```

## Pure-view test

Render the actual production view in the test process. No shell,
child process, PTY, daemon, or terminal session starts. Compiling
`tui-test` runtime code as a dependency does not start anything: only
calling its session APIs opens a terminal.

```rust
use ratatui::widgets::Paragraph;
use tuiscotti::check::FrozenStore;
use tuiscotti::render_profile::{PROFILE_NAME, platform_key, resolve_profile};
use tuiscotti::sample::{Observation, Sample};
use tuiscotti::view::ViewSettings;

let settings = ViewSettings::new(80, 24)?;
let snapshot = tuiscotti::view::capture_view(
    |frame| frame.render_widget(Paragraph::new("hello"), frame.area()),
    &settings,
)?;
let profile = resolve_profile(PROFILE_NAME)?;
let sample = Sample::with_pixels(
    &format!("views/hello-{}", platform_key()),
    &snapshot,
    Observation::empty(),
    profile,
)?;
let store = FrozenStore::new("baselines", "artifacts")?;
assert!(store.check(&sample.to_bytes()?)?.is_pass());
```

Runnable end to end: `cargo run -p tuiscotti --example 01-pure-view`
(exit 0, prints `EXAMPLE-01-OK`). The other examples cover a live
upstream session (`02-live-session`), a missing baseline and its
explicit approval (`03-missing-baseline`), read-only evidence outside
the baseline root (`04-frozen-baseline`), and a state-only sample
with no pixel requirement (`05-state-only`).

## Baseline review procedure

A changed sample never self-heals. `check` writes the candidate and
its evidence to the artifact root and reports `different`. Review
means reading the candidate evidence, then running one explicit
command with both digests:

```sh
tuiscotti accept --candidate <CANDIDATE> --candidate-sha256 <SHA> \
  --expect-baseline <SHA_OR_MISSING> --baselines <DIR>
```

Approval is refused before any write when the `CI` variable is set
(any value, including empty), on digest mismatch, on stale baselines,
and on unknown profiles. There is no bless flag, no auto-accept, and
no override. See [docs/CLI.md](docs/CLI.md) and [docs/API.md](docs/API.md).

## Docs

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — crates, dependency graph, data flow
- [docs/API.md](docs/API.md) — public Rust API per section-3.1 responsibility
- [docs/CLI.md](docs/CLI.md) — CLI reference (from the implemented Clap grammar)
- [docs/LIMITATIONS.md](docs/LIMITATIONS.md) — fonts, platform limits, unsupported features
- [docs/MIGRATION.md](docs/MIGRATION.md) — schema and layout migrations (historical)

## Status labels

Docs use five truthful labels: **implemented** (shipped, tested),
**partial** (works with documented gaps), **unsupported**
(explicitly rejected, fails closed), **future** (planned, absent),
and per-platform **tested / compiles-only / not run**. Removed APIs
stay removed: they appear only in the migration record under
[docs/migration/](docs/migration/), never as available features.

## License

MIT OR Apache-2.0 — see [LICENSE](LICENSE).
