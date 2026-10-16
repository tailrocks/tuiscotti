# Tests, fixtures, and CI

## Layout

| Location | Contents |
|---|---|
| `crates/tuiscotti/tests/` | 20 suites: `cargo_bin_unified`, `cells`, `examples_lane`, `export`, `facade`, `g6_facade`, `grouped`, `import_compat`, `insta_spike`, `locate`, `p0_mutations`, `ratatui_views`, `render_qual`, `runner`, `screen`, `semant`, `snapshot`, `snapshot_lifecycle`, `snapshot_safety`, `tool_qualification` (269 tests) |
| `crates/tuiscotti-cli/tests/` | `agent_if`, `cli`, `epipe`, `observe`, `piped`, `readme_lock`, `vertical_slice_cli_error` (85 tests) |
| `crates/tuiscotti-runtime/tests/` | `tui`, `tui_shell` (PTY; need feature `pty`) |
| `crates/tuiscotti-render/`, `-insta/`, `xtask` | unit tests in `src/` (12 + 21 + 17 tests) |
| `crates/tuiscotti-fixtures/` | `fixture_app` model/view + `tests/{format_contracts, interaction_contracts, journey, render, underline, vertical_slice, view_contracts, visual}` + committed approvals (86 tests) |
| `crates/tuiscotti/examples/` | `01-pure-view` … `08-agent-workflow` (the learning lane) |

683 tests via `cargo nextest list --locked --offline
--all-features` on main `d996feb`; full run green 683/683 + 1 skipped
(2026-10-01; see [PERFORMANCE.md](PERFORMANCE.md)).
Doctest lane is separate (`cargo test --doc`).

## The fixture app

`tuiscotti-fixtures::fixture_app` is one model/view that doubles as
the PTY target and the headless-test source. Three fixture binaries
(`crates/tuiscotti-fixtures/Cargo.toml` `[[bin]]`):
`menu_fixture`, `streams_fixture`, `protocol_fixture`
(`tests/fixtures/apps/*.rs`). Committed approvals:

- `crates/tuiscotti-fixtures/tests/visual/approved/` — 24
  `*.frame.json` + `*.png` pairs (classic store).
- `crates/tuiscotti-fixtures/tests/snapshots/` — Insta snapshots
  (`.snap` + `.snap.png`) for journey/vertical-slice gates.
- `crates/tuiscotti-fixtures/tests/fixtures/` — consumer, data,
  expected, and slice fixtures.
- `crates/tuiscotti-fixtures/tests/SHA256SUMS` — pins every
  approval byte; re-record with `cargo xtask fixtures
  --bless-manifest` after a qualified change (a test fails
  otherwise).

## Examples lane (learning path)

Eight small programs, each one idea, each exiting 0 with an
`EXAMPLE-NN-OK` marker. Nothing writes outside temp dirs.

| # | Example | Idea |
|---|---|---|
| 01 | `01-pure-view` | Production draw closure → `Screen` → `assert_snapshot!` (pre-approved) |
| 02 | `02-styled-shot` | `assert_screenshot!`: canonical + PNG as one sample, evidence on disk |
| 03 | `03-piped-cli` | `Command` error cases: spawn errors vs exit codes, split streams |
| 04 | `04-interactive-tui` | PTY journey: spawn → `wait_predicate` → snapshot → close |
| 05 | `05-locators-waits` | `Locator::text` + `expect_visible` to one deadline |
| 06 | `06-artifacts-review` | `emit_four` + frozen root: pin artifacts, reject acceptance |
| 07 | `07-advanced-profiles` | Strict `RenderProfile`, `Strict` vs `Placeholder` missing policy |
| 08 | `08-agent-workflow` | `proto::execute` + `machine` JSON envelopes, no PTY |

```sh
cargo run -p tuiscotti --example 01-pure-view   # any single step
cargo test -p tuiscotti --test examples_lane    # runs all 8, asserts exit 0 + markers
```

## Running the suite

```sh
cargo nextest run --locked --offline --all-features   # preferred: parallel, ~33 s warm locally (32.7 s measured 2026-10-01)
cargo test --locked --offline                         # serial fallback
cargo test -p tuiscotti --no-default-features         # pure-view only, no PTY engine
cargo test --locked --offline --doc                   # doctests
```

Timings are local Apple-silicon numbers; see
[PERFORMANCE.md](PERFORMANCE.md). The 120 s CI fast-lane budget must
be re-proven with CI timings, not local ones.

## Clock and Time Control in PTY Subprocesses

`tui::Tui` launches a real OS child process attached to a kernel PTY.
Because the child executes in its own address space and schedules its
own timers, the test harness cannot inject a virtual Rust clock into
an arbitrary child binary over a PTY, and `Session::wait_stable_quiet`
measures real wall-clock PTY byte silence on the reader thread.

Use this two-part pattern for deterministic animations, spinners,
blinking cursors, and wall-clock timestamps in binaries under test:

1. **App-side deterministic clock / tick env convention**: pass an
   environment variable via
   `Tui::new(cmd).env("TUISCOTTI_FIXED_TIME", "2026-01-01T00:00:00Z")`
   or `Tui::new(cmd).env("APP_TICK_MS", "0")` (or a key-driven step
   mode) so blinking cursors, spinners, and wall-clock timestamps
   freeze during visual baseline runs.
2. **Wait-side semantic readiness + short quiet window**: wait on a
   semantic readiness condition first
   (`session.get_by_text("Ready").any().expect_visible()?` or
   `wait_predicate`), then pair it with a short quiet window
   (`session.wait_stable_quiet(deadline, Duration::from_millis(50), &cancel)?`)
   instead of relying on a large `wait_stable_quiet(400ms)` as a
   substitute for readiness.

```rust,no_run
use std::path::Path;
use std::time::{Duration, Instant};
use tuiscotti::grouped::{ArtifactSet, GroupedStore};
use tuiscotti::tui::{CancelToken, MouseButton, MouseMods, Tui};
use tuiscotti::{Profile, VENDORED_FACES, render};

fn pty_to_grouped_files() -> Result<(), Box<dyn std::error::Error>> {
    let session = Tui::new(["./target/debug/my-tui"])
        .size(120, 40)
        .env_clear(true)
        .env("TERM", "xterm-256color")
        .env("TUISCOTTI_FIXED_TIME", "2026-01-01T00:00:00Z")
        .env("APP_TICK_MS", "0")
        .spawn()?;

    session.get_by_text("Ready").any().expect_visible()?;
    let cancel = CancelToken::new();
    session.wait_stable_quiet(
        Instant::now() + Duration::from_secs(2),
        Duration::from_millis(50),
        &cancel,
    )?;

    // Hover via mouse_move (requires DEC 1003 + 1006) and press-and-hold:
    session.mouse_move(10, 5, MouseMods::NONE)?;
    session.mouse_down(MouseButton::Left, 10, 5, MouseMods::NONE)?;
    session.mouse_up(MouseButton::Left, 10, 5, MouseMods::NONE)?;

    let frame = render::frame_from_screen(&session.snapshot()?, "tuiscotti-default");
    let store = GroupedStore::new(Path::new("snapshots/visual"))
        .with_artifacts(ArtifactSet::AnsiCursor)
        .with_actual_root(Path::new("target/tuiscotti/actual"))
        .with_expected_root(Path::new("target/tuiscotti/expected"))
        .with_diff_root(Path::new("target/tuiscotti/diff"))
        .with_run_id("local-visual-run-1");
    let profile = Profile::default_profile();
    store
        .check("dashboard/hover_120x40", &frame, &profile, &VENDORED_FACES, 1.0)?
        .ensure_matched()?;

    session.close()?;
    Ok(())
}
```

## CI wiring

Gates run everywhere; approval happens only on a human workstation.
There is no flag or variable that approves — `accept` and `Store::accept`
/ `GroupedStore::accept*` explicitly refuse when `CI` is set in the
environment.

```yaml
- name: Visual gates
  run: cargo nextest run --locked --offline --all-features

- name: Publish visual evidence (on failure too)
  if: always()
  uses: actions/upload-artifact@v4
  with:
    name: visual-evidence
    path: |
      target/tuiscotti/actual/
      target/tuiscotti/expected/
      target/tuiscotti/diff/
      crates/tuiscotti-fixtures/tests/visual/actual/
      crates/tuiscotti-fixtures/tests/visual/diff/
      crates/tuiscotti-fixtures/tests/visual/report.html
```

Notes:

- Determinism: same frame + same profile + same vendored font
  bytes = byte-identical PNGs (tested). Runners need no system fonts.
- Review flow for a red run: download `visual-evidence`, open
  `report.html`, reproduce locally if needed, then run
  `tuiscotti accept --store <dir> <name>` (or a segment-aware prefix,
  single-segment `*` glob, or multi-segment `**` glob such as
  `tuiscotti accept --store snapshots/visual --actual target/tuiscotti/actual "dashboard/*"`
  or `"dashboard/**"`) locally on your workstation and push the updated
  approved tree.
- Parallel jobs are safe: per-name files + atomic renames.
  Concurrent jobs sharing one store directory only race on
  `report.html` (best-effort index); gate data never clobbers.
- Do not cache `actual/`, `expected/`, `diff/`, or `report.html`
  between runs — they are per-run evidence.

## Link and doc checks

Every relative link in `README.md`, `CONTRIBUTING.md`, and
`docs/*.md` must resolve to a file in the tree. Check:

```sh
cargo run -p xtask -- docs
```

`readme_lock.rs` additionally mirrors every README fence and pins
the CLI surface against `--help` — run it after any doc edit that
touches claimed behavior:
`cargo test -p tuiscotti-cli --test readme_lock`.
