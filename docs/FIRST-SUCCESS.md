# First success: create and approve a binary baseline

This walkthrough creates a sample from a Ratatui view, checks it against an empty baseline root, reviews its evidence, approves it, and checks it again. The commands show the workflow. File sizes, digests, absolute paths, and rendered image details depend on the run.

The maintained example lives at `crates/tuiscotti/examples/hello.rs`. It saves one state-only sample by default. Pass `--pixels` to save a `state-and-pixels` sample.

## Prepare the workspace

Use the pinned Rust toolchain. Fetch dependencies before running commands offline:

```sh
cargo fetch --locked
cargo build --locked --offline
```

Create separate directories for samples, baselines, and evidence:

```sh
D=/tmp/first-success
mkdir -p "$D/samples" "$D/baselines" "$D/artifacts"
```

## Create a sample

```sh
cargo run -q -p tuiscotti --example hello --locked --offline -- \
  "$D/samples/hello.tui.pb"
```

The example writes one versioned sample in the binary container. `Sample::to_bytes` encodes the logical `tuiscotti.sample.v1` value as Protobuf and compresses it with Zstandard. The container header and compressed bytes are part of the file digest.

Get the digest from the file you created. On macOS, use `shasum -a 256`; on Linux, use `sha256sum`:

```sh
shasum -a 256 "$D/samples/hello.tui.pb"
```

Keep this value for the `accept` command. Do not copy a digest from another run.

## Check the sample

```sh
cargo run -q -p tuiscotti-cli --locked --offline -- \
  check --sample "$D/samples/hello.tui.pb" \
  --baselines "$D/baselines" --artifacts "$D/artifacts" --json
```

The first check reports `missing` and exits with code 1. It writes candidate evidence under the artifact root, including a `sample.tui.pb` copy and a JSON `result.json` record. It does not write a baseline.

The `tuiscotti.cli.v1` output and `tuiscotti.check.v1` result record are JSON metadata. They remain JSON while sample and baseline payloads use the binary format.

## Review the evidence

Render the sample and build the report index:

```sh
cargo run -q -p tuiscotti-cli --locked --offline -- \
  render --sample "$D/samples/hello.tui.pb" --out "$D/hello.png" --json
cargo run -q -p tuiscotti-cli --locked --offline -- \
  report --artifacts "$D/artifacts" --out "$D/index.html"
```

Review the image and the candidate report before approval. The `report` command reads JSON result metadata and links to the binary sample evidence.

## Approve the candidate

Compute the candidate digest from the exact file bytes. Then pass that digest and the expected prior baseline value to `accept`:

```sh
CANDIDATE=$(shasum -a 256 "$D/samples/hello.tui.pb" | cut -d' ' -f1)
cargo run -q -p tuiscotti-cli --locked --offline -- \
  accept --candidate "$D/samples/hello.tui.pb" \
  --candidate-sha256 "$CANDIDATE" \
  --expect-baseline missing --baselines "$D/baselines" --json
```

`accept` re-reads both files under the stable approval lock, checks both digests, verifies image render identity and pixels when present, and atomically installs the candidate's exact bytes. The baseline digest therefore equals the candidate digest.

Approval refuses whenever `CI` is set, including an empty value. There is no override. A rejection does not create or replace a baseline.

## Check again

```sh
cargo run -q -p tuiscotti-cli --locked --offline -- \
  check --sample "$D/samples/hello.tui.pb" \
  --baselines "$D/baselines" --artifacts "$D/artifacts" --json
```

The check now reports `pass`. The baseline path is `<baseline-root>/<test-id>.tui.pb`.

## Check a changed view

Edit the example view while keeping its test ID, save a second sample, and check it:

```sh
cargo run -q -p tuiscotti --example hello --locked --offline -- \
  "$D/samples/hello-changed.tui.pb"
cargo run -q -p tuiscotti-cli --locked --offline -- \
  check --sample "$D/samples/hello-changed.tui.pb" \
  --baselines "$D/baselines" --artifacts "$D/artifacts" --json
```

A changed snapshot reports `different`. It does not update the baseline. Review the new evidence and use `accept` with the new candidate digest and the current baseline digest if the change is intended.

## Use the pixel gate

The `state` gate compares snapshot state. The `state-and-pixels` gate also compares the render contract and decoded RGBA pixels. Its test ID ends in `/pixels-<os>-<arch>`:

```sh
cargo run -q -p tuiscotti --example hello --locked --offline -- \
  "$D/samples/hello-pixels.tui.pb" --pixels
```

The example uses `tuiscotti::render_profile::platform_key()` to build the suffix. PNG bytes and decoded RGBA have separate digests. Pixel comparison uses decoded RGBA, not compressed sample bytes or PNG byte equality.

## Keep baselines and evidence separate

Commit approved `.tui.pb` files under the baseline root. Treat temporary samples, candidate evidence, reports, and rendered images as disposable unless your project has a specific reason to retain them.

Old `.tui.json` files are not active baselines. The binary reader rejects JSON and does not convert it. Regenerate samples from their source and review them before approval. Keep historical receipts as historical records; they do not approve regenerated bytes.

A sample tree can contain unrelated JSON files. The CLI reads JSON result metadata and output records by their own schemas; it does not treat those files as sample payloads.

## Keep live sessions upstream

Tuiscotti does not launch processes or open terminal devices. Its four CLI commands are offline. Application tests use upstream `tui_test::Session` for live sessions, input, locators, waits, capture, rendering, and pixel comparison.
