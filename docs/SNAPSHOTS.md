# Snapshot and approval semantics

The sample file is the approved input to a frozen check. A check never writes a baseline. Only an explicit, workstation approval can replace one.

## Binary sample files

`Sample::to_bytes` writes one versioned Protobuf message in a Zstandard-compressed binary container. `Sample::from_bytes` validates and reads it. Persistence message types stay private. The logical sample identifier remains `tuiscotti.sample.v1`.

Baseline files use `<baseline-root>/<test-id>.tui.pb`. Candidate evidence uses `sample.tui.pb` and `expected.tui.pb`. The header and compressed payload are part of the sample file bytes. The SHA-256 digest covers both.

Render contract geometry and scale must be finite; readers and writers reject NaN and infinity. Finite values retain their exact floating-point bits.

The reader accepts only the supported binary container. JSON samples, unknown formats, unsupported versions, malformed messages, and resource-limit violations fail closed. The reader does not fall back to `.tui.json`. An old JSON file beside a missing `.tui.pb` does not count as a baseline. The reader does not convert old samples. Regenerate samples from their source, then review them before approval.

## Comparison gates

The `state` gate compares canonical snapshot state. It excludes snapshot metadata and sample observation. The `state-and-pixels` gate also compares the render contract and decoded RGBA pixels. PNG bytes identify the saved image. PNG or compressed-file byte equality does not replace pixel comparison.

A gate change is a policy mismatch. Keep the `pixels-<os>-<arch>` suffix paired with the recorded render-contract platform. Historical samples can parse on another platform.

The state digest keeps the existing `tuiscotti-state-v1` algorithm. It remains independent of persistence encoding and keeps its metadata exclusions. Candidate and baseline SHA-256 values cover exact saved file bytes. PNG and RGBA digests keep their existing scopes.

## Frozen checks

`FrozenStore::check` and `check_file` read the candidate and baseline. They return a `CheckOutcome`: pass, missing baseline, or different sample. A check never writes to the baseline root. It writes candidate evidence and JSON result metadata under the separate artifact root.

Result records, approval receipts, configuration, CLI output, and the private state-digest projection remain separate from binary sample persistence. `result.json` remains JSON. It records digests of the exact candidate and baseline bytes.

## Approval

`ApprovalStore::accept` reads both files while holding `.tuiscotti-approval.lock`. The lock stays in place after release. The transaction checks the candidate digest and expected prior baseline digest. It validates the binary sample, resolves its render profile, and verifies render identity and pixels. If both digests match, approval atomically installs the exact candidate bytes. The accepted baseline digest therefore equals the candidate digest.

The transaction rejects JSON candidates, stale digests, unknown profiles, invalid samples, and mismatched render identity or pixels. A rejection does not create or replace a baseline. Approval refuses if `CI` is set to any value, including an empty value. The refusal occurs before writes. There is no override.

## Reports and CLI

`tuiscotti report` reads JSON result metadata and links to binary candidate evidence. It does not decode sample payloads or change approvals. `check`, `accept`, and `render` share the bounded binary sample decoder.

CLI `--json` output keeps the `tuiscotti.cli.v1` schema. Exit code 0 means pass or successful operation. Exit code 1 means `check` found a missing or different baseline. Exit code 2 means invalid arguments. Exit code 3 means an operational failure. See the [CLI reference](CLI.md).
