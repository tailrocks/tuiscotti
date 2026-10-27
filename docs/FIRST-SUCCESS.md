# First success: from a blank checkout to an approved baseline

A hands-on walkthrough of the offline workflow:
capture a view, freeze it as a versioned sample, watch the first
check report `missing`, inspect the candidate evidence, approve it
explicitly, and recheck to `pass`. State-only first, then the
pinned-profile pixel gate. Every output below was pasted from a
real run on macOS arm64 (`macos-aarch64`); digests are real, not
placeholders.

The maintained example used here lives at
`crates/tuiscotti/examples/hello.rs`: one production-style Ratatui
draw function (`draw_dashboard`, header plus a list with a healthy
and an unhealthy entry) saved as a versioned sample. Run it with:

```sh
cargo run -q -p tuiscotti --example hello --locked --offline -- <out.tui.json> [--pixels]
```

Without `--pixels` it writes a `state` sample with test ID
`tutorial/hello-dashboard`. With `--pixels` it writes a
`state-and-pixels` sample whose test ID carries the platform key,
e.g. `tutorial/hello-dashboard/pixels-macos-aarch64`.

## 0. Prereqs and install

Requires Rust 1.99.1 (pinned by `rust-toolchain.toml`; observed
`cargo 1.99.1`, `rustc 1.99.1`, `tuiscotti 0.4.0`). No system
fonts, no terminal device, no network at run time. From the
workspace root:

```sh
cargo fetch --locked        # warm the registry cache for offline builds
cargo build --locked --offline
```

The `tuiscotti-tui-test` dependency is a plain crates.io release
(`0.2.1`, lib name `tui_test`): downstream crates resolve the same
release with no pin (see "Downstream crates" below).

All CLI commands below run from the workspace root with disjoint,
persistent directories. Pick a fresh location outside the repo so
scratch never mixes with versioned files:

```sh
mkdir -p /tmp/first-success/samples /tmp/first-success/baselines /tmp/first-success/artifacts
D=/tmp/first-success
```

(`baselines/` and `artifacts/` must be different directories; a
check refuses nested or equal roots.)

## 1. Create a state-only sample

```sh
cargo run -q -p tuiscotti --example hello --locked --offline -- $D/samples/hello-v1.tui.json
```

Observed:

```text
wrote /tmp/first-success/samples/hello-v1.tui.json (453046 bytes)
```

The file is a `tuiscotti.sample.v1` document:

```sh
python3 -c "import json; s=json.load(open('$D/samples/hello-v1.tui.json')); print(s['schema'], s['test_id'], s['gate'])"
```

```text
tuiscotti.sample.v1 tutorial/hello-dashboard state
```

Its SHA-256 over the file bytes is the candidate digest the
workflow will quote back at you:

```text
sha256: 02a88f1ac58f45b7999e72f3650fb901c0c6212d7d7459e31b84f50880b52655
```

## 2. First check: missing baseline, exit 1

```sh
cargo run -q -p tuiscotti-cli --locked --offline -- \
  check --sample $D/samples/hello-v1.tui.json \
  --baselines $D/baselines --artifacts $D/artifacts --json
```

Observed (one JSON line on stdout; `exit=1`):

```json
{"schema":"tuiscotti.cli.v1","command":"check","status":"missing","test_id":"tutorial/hello-dashboard","reason":"missing","candidate_path":{"encoding":"utf8","value":"/private/tmp/first-success/artifacts/tutorial/hello-dashboard/02a88f1ac58f45b7999e72f3650fb901c0c6212d7d7459e31b84f50880b52655/sample.tui.json"},"candidate_sha256":"02a88f1ac58f45b7999e72f3650fb901c0c6212d7d7459e31b84f50880b52655","baseline_sha256":"missing","output_path":{"encoding":"utf8","value":"/private/tmp/first-success/artifacts/tutorial/hello-dashboard/02a88f1ac58f45b7999e72f3650fb901c0c6212d7d7459e31b84f50880b52655/checks/missing/report.html"},"error_code":null}
```

Without `--json` the same run prints one human line:

```text
check: missing test=tutorial/hello-dashboard reason=missing
```

Validate the machine output instead of eyeballing it: `schema`
must be `tuiscotti.cli.v1`, `candidate_sha256` must equal your
locally computed file digest, `baseline_sha256` is the literal
string `missing`, and paths arrive as `{encoding, value}` objects
(`utf8` here; non-UTF-8 paths use `unix-bytes-base64` or
`windows-utf16le-base64`).

Exit codes are load-bearing; never hide them behind a pipe without
`pipefail`:

| Exit | Meaning |
|---|---|
| 0 | pass, or a successful offline operation |
| 1 | missing baseline or differing sample (`check` only) |
| 2 | invalid arguments: clap usage errors (no JSON on stdout), `render --out` with a non-`.png`/`.html` extension, malformed `accept` digests |
| 3 | file, format, profile, rendering, approval, or I/O failure (JSON result on stdout where one applies) |

Observed exit 2 (empty stdout, diagnostics on stderr):

```text
error: the following required arguments were not provided:
  --baselines <BASELINES>
```

```text
error: invalid value '/tmp/first-success/hello.txt' for '--out <OUT>': unsupported output extension: expected a `.png` or `.html` --out path
```

Observed exit 3 for a corrupt sample (JSON on stdout, `error_code`
names the fault):

```json
{"schema":"tuiscotti.cli.v1","command":"check","status":"error","test_id":null,"reason":"invalid sample encoding: expected ident at line 1 column 2","candidate_path":{"encoding":"utf8","value":"/tmp/first-success/samples/corrupt.tui.json"},"candidate_sha256":"92628a747890d02d1459c6eb45fd13cfa63bbb6d346412cff190297cf9c33d39","baseline_sha256":null,"output_path":null,"error_code":"INVALID_SAMPLE"}
```

A malformed `accept` digest is exit 2 with no JSON:

```text
error: invalid value 'deadbeef' for '--candidate-sha256 <CANDIDATE_SHA256>': digest "deadbeef" is not a 64-character lowercase hex digest
```

The first check wrote candidate evidence but no baseline:

```text
artifacts/tutorial/hello-dashboard/02a88f…2655/checks/missing/report.html
artifacts/tutorial/hello-dashboard/02a88f…2655/checks/missing/result.json
artifacts/tutorial/hello-dashboard/02a88f…2655/sample.tui.json
```

`result.json` is the `tuiscotti.check.v1` record for this
comparison:

```json
{"schema":"tuiscotti.check.v1","test_id":"tutorial/hello-dashboard","gate":"state","reason":"missing","candidate_sha256":"02a88f1ac58f45b7999e72f3650fb901c0c6212d7d7459e31b84f50880b52655","baseline_sha256":"missing","changed_fields":["baseline"],"changed_cells":[],"changed_cell_total":0,"title_values":null,"pixel_changed_pixels":0,"pixel_compared_pixels":0,"pixel_bounds":null}
```

## 3. Inspect the candidate evidence

Render the saved sample and build a report index over the
artifact root:

```sh
cargo run -q -p tuiscotti-cli --locked --offline -- \
  render --sample $D/samples/hello-v1.tui.json --out $D/hello.png --json
cargo run -q -p tuiscotti-cli --locked --offline -- \
  render --sample $D/samples/hello-v1.tui.json --out $D/hello.html
cargo run -q -p tuiscotti-cli --locked --offline -- \
  report --artifacts $D/artifacts --out $D/index.html
```

Observed: `render` exits 0 with `"reason":"tuiscotti-pinned-v1"`
(a state-only sample re-renders through the pinned profile),
writing `hello.png` (17453 bytes) and `hello.html` (23619 bytes);
`report` prints `report: pass` and the index at `$D/index.html`
links to `.../checks/missing/report.html`. Open the PNG and the
per-candidate `report.html` and review them the way you would
review a code diff: only what you see there is what the baseline
will freeze.

## 4. Approve explicitly, on a workstation, after review

`accept` is the only path that writes a baseline, and it refuses
before writing anything when `CI` is set in any form. There is no
bless flag and no override; do not script around this with
`env -u CI` — approvals belong on a human workstation after the
review in step 3. Confirm the refusal first:

```sh
CI=true cargo run -q -p tuiscotti-cli --locked --offline -- \
  accept --candidate $D/samples/hello-v1.tui.json \
  --candidate-sha256 02a88f1ac58f45b7999e72f3650fb901c0c6212d7d7459e31b84f50880b52655 \
  --expect-baseline missing --baselines $D/baselines --json
```

Observed (`exit=3`, zero files under `baselines/`):

```json
{"schema":"tuiscotti.cli.v1","command":"accept","status":"error","test_id":null,"reason":"approval is forbidden while CI is set","candidate_path":{"encoding":"utf8","value":"/tmp/first-success/samples/hello-v1.tui.json"},"candidate_sha256":"02a88f1ac58f45b7999e72f3650fb901c0c6212d7d7459e31b84f50880b52655","baseline_sha256":null,"output_path":null,"error_code":"CI_APPROVAL_FORBIDDEN"}
```

Then approve with `CI` unset, quoting both digests from the
`check` output (candidate digest plus the expected baseline digest,
`missing` for a first approval):

```sh
cargo run -q -p tuiscotti-cli --locked --offline -- \
  accept --candidate $D/samples/hello-v1.tui.json \
  --candidate-sha256 02a88f1ac58f45b7999e72f3650fb901c0c6212d7d7459e31b84f50880b52655 \
  --expect-baseline missing --baselines $D/baselines --json
```

Observed (`exit=0`):

```json
{"schema":"tuiscotti.cli.v1","command":"accept","status":"pass","test_id":"tutorial/hello-dashboard","reason":null,"candidate_path":{"encoding":"utf8","value":"/tmp/first-success/samples/hello-v1.tui.json"},"candidate_sha256":"02a88f1ac58f45b7999e72f3650fb901c0c6212d7d7459e31b84f50880b52655","baseline_sha256":"02a88f1ac58f45b7999e72f3650fb901c0c6212d7d7459e31b84f50880b52655","output_path":{"encoding":"utf8","value":"/private/tmp/first-success/baselines/tutorial/hello-dashboard.tui.json"},"error_code":null}
```

`accept` re-validates both digests under a lock, re-renders image
samples through the upstream renderer, and installs the baseline
atomically — or rejects with a typed code and writes nothing.

## 5. Recheck: pass

```sh
cargo run -q -p tuiscotti-cli --locked --offline -- \
  check --sample $D/samples/hello-v1.tui.json \
  --baselines $D/baselines --artifacts $D/artifacts --json
```

Observed (`exit=0`):

```json
{"schema":"tuiscotti.cli.v1","command":"check","status":"pass","test_id":"tutorial/hello-dashboard","reason":null,"candidate_path":{"encoding":"utf8","value":"/tmp/first-success/samples/hello-v1.tui.json"},"candidate_sha256":"02a88f1ac58f45b7999e72f3650fb901c0c6212d7d7459e31b84f50880b52655","baseline_sha256":"02a88f1ac58f45b7999e72f3650fb901c0c6212d7d7459e31b84f50880b52655","output_path":null,"error_code":null}
```

## 6. Change the view, keep the test ID: diff

Edit `draw_dashboard` in `crates/tuiscotti/examples/hello.rs`
(change the title to `"deploys v2"`), save the changed view under
the same test ID, and check again:

```sh
cargo run -q -p tuiscotti --example hello --locked --offline -- $D/samples/hello-v2.tui.json
cargo run -q -p tuiscotti-cli --locked --offline -- \
  check --sample $D/samples/hello-v2.tui.json \
  --baselines $D/baselines --artifacts $D/artifacts --json
```

Observed (`exit=1`):

```json
{"schema":"tuiscotti.cli.v1","command":"check","status":"different","test_id":"tutorial/hello-dashboard","reason":"state","candidate_path":{"encoding":"utf8","value":"/private/tmp/first-success/artifacts/tutorial/hello-dashboard/81625d11a38de9b6109933ab9328475551d01be29adf59a1414c7c1ff9780c93/sample.tui.json"},"candidate_sha256":"81625d11a38de9b6109933ab9328475551d01be29adf59a1414c7c1ff9780c93","baseline_sha256":"02a88f1ac58f45b7999e72f3650fb901c0c6212d7d7459e31b84f50880b52655","output_path":{"encoding":"utf8","value":"/private/tmp/first-success/artifacts/tutorial/hello-dashboard/81625d11a38de9b6109933ab9328475551d01be29adf59a1414c7c1ff9780c93/checks/02a88f1ac58f45b7999e72f3650fb901c0c6212d7d7459e31b84f50880b52655/report.html"},"error_code":null}
```

A changed sample never self-heals: only an explicit `accept`
with the new candidate digest changes the baseline. Revert the
draw-function edit when you are done experimenting.

## 7. Pixel gate: same flow, platform-qualified test ID

The `state` gate compares the snapshot only. The
`state-and-pixels` gate additionally compares the render contract
and the decoded RGBA pixels, so the test ID must end in
`/pixels-<os>-<arch>` (here `pixels-macos-aarch64`; use
`tuiscotti::render_profile::platform_key()` to build it). The
`--pixels` flag on the example does exactly that:

```sh
cargo run -q -p tuiscotti --example hello --locked --offline -- $D/samples/hello-pixels.tui.json --pixels
```

Observed:

```text
wrote /tmp/first-success/samples/hello-pixels.tui.json (479065 bytes)
```

```text
test_id: tutorial/hello-dashboard/pixels-macos-aarch64
gate: state-and-pixels
sha256: b154b62e7ba02001bbcd932fc391271be003feae9cddfc41957414b9c814d6dc
profile: tuiscotti-pinned-v1
platform_key: macos-aarch64
image: png 800 480
```

Run the same check/accept/recheck cycle. Observed exits:
`check` reports `missing` at exit 1, `accept` (with
`--expect-baseline missing` and `CI` unset) exits 0, and the
recheck exits 0 with `status: pass` and matching digests
(`candidate_sha256 == baseline_sha256 ==
b154b62e…6dc`). Pixel evidence adds the rendered image next to
the JSON records:

```text
artifacts/tutorial/hello-dashboard/pixels-macos-aarch64/b154b62e…6dc/checks/missing/actual.png
artifacts/tutorial/hello-dashboard/pixels-macos-aarch64/b154b62e…6dc/checks/missing/report.html
artifacts/tutorial/hello-dashboard/pixels-macos-aarch64/b154b62e…6dc/checks/missing/result.json
artifacts/tutorial/hello-dashboard/pixels-macos-aarch64/b154b62e…6dc/sample.tui.json
```

(A `different` pixel check additionally writes `expected.png`
and `diff.png`.) Pixel rendering is deterministic: same frame,
same `tuiscotti-pinned-v1` profile, same vendored fonts, so the
same bytes on every machine. The test ID still carries the
platform key because the gate contract is platform-qualified by
policy, not because the pixels differ.

## 8. What to version, what to throw away

Version (commit) `baselines/`: one `<test-id>.tui.json` file per
approved view. It is the reviewed expectation; CI checks against
it and never writes it.

Treat as disposable: `samples/` and `candidates/` (your local
copies of what you checked — reproducible from the draw
function), everything under `artifacts/` (per-candidate evidence:
`sample.tui.json` copies, `checks/*/result.json`,
`report.html`, `actual/expected/diff.png`), rendered `*.png` /
`*.html` from step 3, and `index.html`. Do not cache per-run
evidence between CI runs. A suggested `.gitignore` for a
downstream repo:

```gitignore
/artifacts/
/candidates/
/samples/*.tui.json
/*.png
/*.html
!/docs/*.html
```

## 9. Live sessions belong to your app, via upstream `tui-test`

tuiscotti never spawns a process and never opens a terminal: the
four CLI commands are offline. Live terminal work — child
processes, PTY sessions, input, locators, waits, live capture,
rendering, pixel comparison — is owned by upstream `tui-test`
(`tui-test-rs`) and called directly from application test code;
tuiscotti does not wrap or re-export it. The workspace depends on
the `tuiscotti-tui-test 0.2.1` registry release (renamed fork
carrying the required snapshot and offline-render APIs; see
`docs/ARCHITECTURE.md` and `docs/LIMITATIONS.md`).

Exact supported surface at `tuiscotti-tui-test 0.2.1`,
verified against its published tree:

- sessions: `tui_test::Session::new(name)`,
  `open(OpenOptions)`, `run(RunOptions)`, `close()`,
  `capture_snapshot(scope)` / `capture_snapshot_with(...)`,
  `wait_quiet(&WaitQuietOptions)`, `interrupt()`,
  `spawn_with(...)`, `write_bytes(...)`, checked input
  (`mouse_checked`, `paste`, `focus`)
- actions: `session.execute(Operation)` with the `Operation` /
  `OperationResult` model (`OpenOptions`, `RunOptions`,
  `Backend::{Alacritty, Ghostty, Rio, Xtermjs}`)
- locators: `session.get_by_text/get_by_style/get_by_link` plus
  `Locator::{wait, wait_with_timeout, wait_hidden, expect,
  expect_with, click, click_with}`
- snapshots and rendering for the offline handoff:
  `TerminalSnapshot`, `render_snapshot_png`,
  `render::pinned::{RASTER_POLICY_PINNED_V1, PinnedFontSet, ...}`,
  `pixel::compare_rgba`, `profile::Rgb`,
  `terminal::cell::Attrs`

Pseudocode (shape only — check names and signatures against the
`tuiscotti-tui-test 0.2.1` docs before compiling):

```rust,ignore
let session = tui_test::Session::new("smoke");
session.open(tui_test::OpenOptions { /* cols, rows, backend, .. */ })?;
session.get_by_text("deploys").wait()?;
let snapshot = session.capture_snapshot(tui_test::SnapshotScope::Full)?;
session.close()?;
let sample = tuiscotti::sample::Sample::with_pixels(
    &format!("my-app/smoke/pixels-{}", tuiscotti::render_profile::platform_key()),
    &snapshot,
    tuiscotti::sample::Observation::empty(),
    tuiscotti::render_profile::resolve_profile(tuiscotti::render_profile::PROFILE_NAME)?,
)?;
// then FrozenStore::check / accept exactly as in steps 2-5
```

## 10. Downstream crates: no pin needed

There is no `[patch.crates-io]` section anymore: downstream crates
depend on `tuiscotti "0.4"` plus `tuiscotti-tui-test 0.2.1`
from crates.io and resolve the same release with no extra pin.
This was verified after publication with a scratch crate outside
the workspace depending on `tuiscotti 0.4` from the registry plus the
registry release: `cargo run --offline` built and a
`FrozenStore::check` against a fresh store reported `is_pass=false` as
expected. (Against crates.io
`tui-test-rs` 0.1.1 the build fails — it lacks the fork-only APIs
tuiscotti consumes:

```text
error[E0432]: unresolved imports `tui_test::PinnedFontError`, `tui_test::RenderError`, `tui_test::render_snapshot_png`
error[E0432]: unresolved import `tui_test::pixel`
error[E0432]: unresolved imports `tui_test::SnapshotCell`, `tui_test::TerminalSnapshot`
...
```
)

So: run `cargo fetch --locked` before going offline, and re-run
steps 1-5 from this page in the downstream repo to confirm the
workflow end to end.
