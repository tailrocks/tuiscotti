# Snapshot and approval semantics

Three gates, one rule: **actuals are written before any assertion,
approvals change only by explicit review.** CI must never approve by
itself — there is no bless flag, env var, or accepting CI mode.

## Sample (`sample.rs`)

A sample is versioned canonical JSON: the test ID, the gate
(`state` or `state-and-pixels`), the snapshot payload, the render
contract (profile + font digest), and SHA-256 digests sealing each
part. `state` compares the full canonical state; `state-and-pixels`
additionally compares the render contract and decoded RGBA pixels. A
gate change is a policy mismatch — never weaken a gate after a failure.
State-only samples carry no pixel requirement.

## Frozen check (`check.rs`)

`FrozenStore::check` / `check_file` reads a saved sample and a baseline
tree and reports a `CheckOutcome`: pass, missing baseline, or differ.
The check never writes to the baseline tree — evidence lands in a
separate artifacts directory, which may live outside the baseline root.
Contained roots, symlink escapes, and read-only baselines are rejected
with typed errors. Concurrent checks for the same test ID keep every
candidate; gate data never clobbers.

## Approval (`approve.rs`)

`ApprovalStore::accept` runs the full transaction: hold the OS lock on
the stable `.tuiscotti-approval.lock` (kept after release, never
unlinked), read the candidate, compare digests (a changed candidate is
`STALE_CANDIDATE`, a changed baseline `STALE_BASELINE`), then —
mandatory — resolve the candidate's profile and re-render through the
upstream renderer. Unknown profiles reject with `UNKNOWN_PROFILE`;
pixels that differ from the stored candidate pixels reject with
`INCONSISTENT_SAMPLE`. First approval for a test validates the
candidate test ID; a present baseline skips the ID comparison. Temp
files use exclusive creation with owner-only mode and never follow
symlinks; the baseline is replaced atomically. `CI` in the environment
refuses approval (`CI_APPROVAL_FORBIDDEN`). Malformed samples map to
`INVALID_SAMPLE`.

## Report (`report.rs`)

`report` builds a static index from saved candidate evidence. It
captures no state and changes no approvals.

## CLI mapping

`tuiscotti check` applies the frozen check to a saved sample.
`tuiscotti accept` applies the approval transaction (no `--all`, no
glob, no CI override). `tuiscotti render` writes `.png` or `.html` from
a saved sample (other extensions rejected). `tuiscotti report` writes
the static index. Exit codes: 0 pass, 1 missing/different, 2 invalid
arguments, 3 file/format/profile/rendering/approval/I/O failure.
Machine-readable `--json` output uses the `tuiscotti.cli.v1` schema;
see the [CLI guide](CLI.md).
