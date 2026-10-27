# module approve

Baseline approval with digests and frozen checking (D04, GOAL Section 15).

[`ApprovalStore::accept`] runs one explicit approval transaction for one
test ID: it refuses under `CI`, validates both digests, locks the stable
per-root file, reads the candidate once, validates the candidate test ID
unconditionally, arbitrates digests against the baseline read under the
same lock, re-renders image samples with render-identity verification,
then stages the baseline and commits it with one atomic rename. The
change summary rides in the returned outcome; no approval-success record
is persisted, so the rename is the final fallible filesystem operation.
The baseline root is never trusted blindly: the expected previous digest
(or the literal `missing` for a first approval) must match the digest of
the bytes actually present, and the candidate digest must match the bytes
actually approved.

What approval does and does not do:

- It validates the candidate with [`Sample::from_bytes`], which checks
  the schema, test ID, gate pairing, snapshot, and, for a
  `state-and-pixels` sample, the recorded PNG and RGBA digests against the
  decoded image bytes. Frame and image agreement is structural.
- For an image sample it then re-renders the candidate snapshot through
  the upstream renderer under the candidate's SPECIFIED profile (steps
  10-12): an unresolvable profile rejects with
  [`ApproveError::UnknownProfile`], a renderer refusal rejects with
  [`ApproveError::RenderFailed`] (or [`ApproveError::MissingGlyph`] when
  no specified face holds mandatory text). The expected render contract
  is rebuilt through the same typed constructor as sample creation and
  every material field (fonts, renderer, geometry, policy — but not the
  platform key) is compared pre-commit; differing recorded identity, and
  decoded pixels that differ from the candidate pixels, reject with
  [`ApproveError::InconsistentSample`]. State-only samples skip steps
  10-12 ("if the sample has an image").
- The candidate test-ID grammar validates unconditionally, before any
  baseline path is derived or read: a present baseline never skips it.
  Rejected IDs are never normalized into a path, and the derived
  store-relative path accepts only normal components. When a baseline is
  present, no test-ID or gate cross-comparison runs in the change
  summary: the present-baseline `FrozenStore` comparison classifies
  observed evidence shapes (snapshot state, render contracts, decoded
  pixels) without comparing ID or gate enums. The operation targets the
  candidate test ID; the baseline path derives from that ID.
- It carries the change set in the returned outcome from that same
  comparison (state first, then render contract, then decoded RGBA pixels
  through upstream `compare_rgba`), over the stored bytes only. The
  smallest change set wins; no caller-supplied flags steer it.
- It reads the baseline bytes exactly once per transaction, directly
  from the store under the held lock. The candidate bytes are read once
  and reused by digest, validation, comparison, and upsert, so the
  candidate reference stays consistent throughout.

Transaction order (GOAL Section 15, numbered steps):

1. Refuse when the `CI` variable is set, before any write.
2. Validate the digest arguments (missing, malformed, and zero digests
   are rejected as `INVALID_SAMPLE`) and store paths.
3. Make the baseline root when missing.
4. Acquire the operating-system lock on the stable approval file.
5. Read the candidate bytes one time with the file-size limit.
6. Compare their digest with the mandatory candidate digest.
7. Validate the candidate schema and snapshot.
8. Validate the candidate test ID unconditionally (never skipped, never
   normalized into a path).
9. Derive the store-relative baseline path (normal components only) and
   read the baseline from the store under the same lock.
10. Compare the baseline digest with the mandatory baseline digest.
11. For an image sample, resolve the candidate's specified render profile.
12. Rebuild the expected render identity, re-render the candidate
    snapshot through the upstream renderer, and compare the recorded
    identity and the new decoded pixels with the candidate record.
13. Validate the store-relative paths for the baseline's parent directories.
14. Make the baseline's parent directories under the locked baseline root.
15. Write the approved sample bytes unchanged to a same-directory temporary file (exclusive, owner-only, no symlinks).
16. Flush the temporary file.
17. Close the temporary file and prepare the outcome.
18. Atomically rename the temporary file over the baseline (same-directory renames cannot EXDEV, so no copy fallback). This is the final fallible filesystem operation: nothing is persisted after the commit.
19. Release the lock (the stable lock file persists).
20. Return the approved test ID, sample digest, and change summary.

Locking: one lock per baseline root, held from step 4 through step 19,
across the baseline read, digest comparison, re-render, and atomic
replacement. The lock file `<root>/.tuiscotti-approval.lock` is stable:
it is created once when missing, opened (never recreated) by every
holder, and kept after release. Mutual exclusion is the OS file lock on
the open handle, so holders serialize across processes as well as
threads, and a dead process leaves a stale file that never blocks. The
wait stops after 30 seconds with a typed timeout.

Frozen checking: [`FrozenStore`] has no approval method and gains none
here; checking never writes the baseline root. Approval under a frozen
store is refused in two shapes: a read-only baseline tree fails the lock
or the write with a typed I/O error and writes nothing, and any set `CI`
variable (any value, including empty) refuses approval before any write
with [`ApproveError::CiApprovalForbidden`]. Each approval call is its own
explicit opt-in through its digest arguments; nothing unfreezes a root
persistently.

Error codes (GOAL Section 16.3): there is no `INVALID_DIGEST` code.
Malformed digest arguments report `INVALID_SAMPLE`; every other
failure reports its Section 16.3 code, mapped per failure shape in
[`ApproveError::code`] (unsupported schemas, resource limits, image
errors, unknown profiles, render failures, missing glyphs, stale
digests, inconsistent samples, lock timeouts, store paths, and I/O
each keep their own code instead of flattening to `INVALID_SAMPLE` or
`INVALID_IMAGE`). Process exit codes are owned by the CLI boundary,
which maps the argument-error shape to 2 and every operational
failure to 3.

Volatile-data exclusion: the outcome carried by a completed approval
holds no absolute paths, process IDs, timestamps, host names, or session
names. It holds only the test ID, gate, digests, and the compared change
set, all of which recompute deterministically from the approved bytes.

The offline CLI calls this approval path:

```text
tuiscotti accept --candidate PATH --candidate-sha256 SHA256 --expect-baseline SHA256_OR_MISSING --baselines DIR
```

No command equivalence is promised here: D05 owns argument parsing, JSON
output, and process exit codes. The stable machine codes from
[`ApproveError::code`] are the mapping inputs for that command.

Design notes: approval reuses the frozen differ through `FrozenStore`
instead of duplicating it, so check and
accept cannot disagree on the change set. Small byte-level helpers
(digest shape, root resolution, symlink guard, SHA-256) mirror `check.rs`
without sharing code, because only lock and comparison helpers are
shared across the two files.

# enum `ApproveError`

Every failure an approval transaction can report, with sources retained.

# variant `ApproveError::CiApprovalForbidden`

The `CI` variable is set, so approval is refused before any write.

# variant `ApproveError::InvalidArgument`

A digest argument is missing, malformed, or a zero placeholder.
Digest arguments are sample-selection input, so these report
`INVALID_SAMPLE`; the CLI boundary maps them to the argument-error
exit code 2.

# variant field `ApproveError::detail`

Which argument failed and how.

# variant `ApproveError::StaleCandidate`

The candidate bytes disagree with the supplied candidate digest.

# variant field `ApproveError::expected`

The supplied candidate digest.

# variant field `ApproveError::actual`

The digest of the bytes actually read.

# variant `ApproveError::StaleBaseline`

The present baseline disagrees with the expected previous digest.

# variant field `ApproveError::expected`

The expected previous digest, or `missing`.

# variant field `ApproveError::actual`

The digest of the bytes actually present, or `missing`.

# variant `ApproveError::LockTimeout`

The approval lock stayed held past the acquisition deadline.

# variant `ApproveError::UnknownProfile`

The candidate names a render profile outside the D02 registry, so
steps 10-12 cannot resolve it.

# variant field `ApproveError::requested`

The rejected profile name.

# variant field `ApproveError::observed_baseline`

The baseline digest observed under the lock, or `None` when the
failure precedes the baseline read.

# variant `ApproveError::RenderFailed`

The upstream renderer refused the candidate snapshot during the
step-11 re-render.

# variant field `ApproveError::source`

The upstream rejection, with its source chain.

# variant field `ApproveError::observed_baseline`

The baseline digest observed under the lock, or `None` when the
failure precedes the baseline read.

# variant `ApproveError::MissingGlyph`

No specified font can render mandatory text in the candidate
snapshot during the step-11 re-render.

# variant field `ApproveError::source`

The upstream missing-glyph rejection.

# variant field `ApproveError::observed_baseline`

The baseline digest observed under the lock, or `None` when the
failure precedes the baseline read.

# variant `ApproveError::InconsistentSample`

The step-12 comparison found re-rendered pixels that differ from
the stored candidate pixels.

# variant field `ApproveError::detail`

What disagreed (dimensions and compared byte counts).

# variant field `ApproveError::observed_baseline`

The baseline digest observed under the lock, or `None` when the
failure precedes the baseline read.

# variant `ApproveError::InvalidStorePath`

A baseline or evidence path is empty, unresolvable, or escapes its
root through a symlink.

# variant field `ApproveError::detail`

What the path broke.

# variant field `ApproveError::observed_baseline`

The baseline digest observed under the lock, or `None` when the
failure precedes the baseline read.

# variant `ApproveError::Check`

The inner frozen comparison, candidate validation, or record
serialization failed.

# variant field `ApproveError::source`

The frozen-store failure, with its source chain.

# variant field `ApproveError::observed_baseline`

The baseline digest observed under the lock, or `None` when the
failure precedes the baseline read.

# variant `ApproveError::Io`

A filesystem failure outside sample validation.

# variant field `ApproveError::source`

The filesystem failure.

# variant field `ApproveError::observed_baseline`

The baseline digest observed under the lock, or `None` when the
failure precedes the baseline read.

# fn code

The stable machine code for this failure, shared with the CLI map.

Every Section 16.3 code is producible here and maps 1:1 with its
failure shape: malformed digest arguments are `INVALID_SAMPLE`
(there is no `INVALID_DIGEST` code), and sample, image, profile,
render, stale, lock, path, and filesystem failures keep their own
codes instead of flattening to `INVALID_SAMPLE` or `INVALID_IMAGE`.

# fn `observed_baseline`

The digest the baseline root actually held when this failure was
reported: the present digest for a stale baseline (or `missing`
when no baseline file was present), the digest read under the lock
for every failure after the baseline read (`missing` when no
baseline file was present there either), and `None` when the
failure precedes the baseline read and carries no observed
baseline.

# fn `with_observed_baseline`

Attach the digest read under the lock to a failure produced after
the baseline read, so the rejection reports what the root actually
held instead of no observed baseline.

# fn `sample_code`

The Section 16.3 code for one validated sample failure.

Resource exhaustion (`TooLarge`, `DimensionTooLarge`, `RgbaTooLarge`)
is `RESOURCE_LIMIT`; an unknown sample schema is `UNSUPPORTED_SCHEMA`;
PNG, digest, and dimension disagreements are `INVALID_IMAGE`; renderer
refusals are `RENDER_FAILED` except a missing glyph, which is
`MISSING_GLYPH`; filesystem failures are `IO`. Everything else —
test IDs, gates, formats, snapshots, contracts, and evidence shapes —
is `INVALID_SAMPLE`.

# struct `AcceptOutcome`

One completed approval transaction: the reviewed candidate is now the
approved baseline for its test ID.

The change summary rides in the outcome itself: no approval-success
record is persisted beside the baseline, so there is no fallible
evidence write after the atomic replacement.

# fn `test_id`

The approved test ID.

# fn gate

The gate both the candidate and the new baseline carry.

# fn `candidate_sha256`

SHA-256 over the approved candidate bytes.

# fn `previous_baseline_sha256`

The previous baseline digest from the prepared transaction, or
`missing` for a first approval.

# fn `baseline_sha256`

SHA-256 over the written baseline bytes; always the candidate digest,
because approval writes the reviewed bytes unchanged.

# fn `baseline_path`

The upserted baseline file.

# fn reason

The structured change reason for this approval: `Missing` for a
first approval, the frozen-comparison reason when the candidate
differs from the previous baseline, or `None` when the candidate
already equals it.

# fn `changed_fields`

Changed fields recorded for this approval, in comparison order:
`["baseline"]` for a first approval, empty when nothing changed.

# struct `ApprovalStore`

An explicit approval handle over one baseline root.

The handle never approves through frozen comparison: checking stays on
[`FrozenStore`], and only this type writes baselines. Roots that cannot
be written (read-only trees) fail at lock acquisition with a typed I/O
error; a set `CI` variable refuses every call before any write.

# fn new

Open an approval handle over a baseline root.

The root resolves through its nearest existing ancestor. It is not
created here; a missing root is created during [`ApprovalStore::accept`].

# Errors

Returns [`ApproveError`] for an empty or unresolvable root.

# fn `baseline_root`

The canonical baseline root this handle approves into.

# fn accept

Approve one candidate for one test ID.

The candidate digest and the expected previous digest (a digest or
the literal `missing`) are mandatory; missing, malformed, and zero
digests are rejected. The candidate file is read-only input: it is
read once and never written. The baseline root gains exactly the
upserted baseline; the change summary rides in the returned outcome
instead of a persisted report, so the atomic replacement is the final
fallible filesystem operation. Refusals (CI, stale digests, corrupt
input, lock failures) write nothing anywhere.

# Errors

Returns [`ApproveError`] for CI refusal, digest rejections, stale
candidates or baselines, invalid samples or paths, lock timeouts, and
I/O failures.

# fn `accept_core`

Run the approval transaction mechanics without the CI gate: steps
2-20 (digest validation, root creation, lock, read, arbitration,
re-render, atomic upsert).

This private method is called by [`ApprovalStore::accept`] after the CI
check succeeds. The functional suite exercises approval through the
shipped CLI. Successful approval uses a child process with `CI` unset.

# Errors

Returns [`ApproveError`] for digest rejections, stale candidates or
baselines, invalid samples or paths, lock timeouts, and I/O
failures.

# fn `accept_locked`

Run the locked transaction: the single candidate read, test-ID
validation, digest arbitration, re-render with identity verification,
change summary, and the commit-last atomic upsert.

# fn `baseline_relative`

Derive the store-relative baseline path for a candidate test ID.

Every relative component must be [`Component::Normal`]: root, prefix,
current-directory, and parent-directory components are rejected outright,
never normalized, so no test ID can escape the baseline root through the
derived path.

# fn `read_baseline_bytes`

Read the baseline file under the held lock through the shared bounded
read: `None` when absent, a resource limit past the size bound, an I/O
error for any other read failure. Bytes past the limit never load, so a
hostile baseline cannot exhaust memory before validation runs.

# fn `write_baseline`

Steps 13-18: validate paths, make parent directories, stage the
approved bytes, prepare the outcome, and commit the baseline.

The approved bytes reach their name through one atomic rename, and that
rename is the final fallible filesystem operation of the transaction: no
approval-success record is persisted after the commit, so a refusal can
never report an error after a commit.

# fn `rerender_check`

Steps 10-12: for an image sample, resolve the candidate's specified
render profile, rebuild the expected render identity through the same
typed constructor as sample creation, re-render the snapshot through the
upstream renderer, and compare the expected identity and the new decoded
pixels with the candidate record.

State-only samples carry no image and skip all three steps. An unknown
profile rejects with [`ApproveError::UnknownProfile`]; a renderer
refusal rejects with [`ApproveError::RenderFailed`] (or
[`ApproveError::MissingGlyph`]); a recorded identity that disagrees with
the re-rendered identity, or pixels that differ from the candidate
pixels, reject with [`ApproveError::InconsistentSample`].

# fn `contract_identity_change`

The first material render-contract field where the recorded candidate
identity differs from the freshly re-rendered identity, or `None` when
they agree.

Every material field compares: profile, contract version, renderer
version, raster policy, fonts with face indices and style slots,
geometry, scale, title bar, palette digest, cursor phase, and blink. The
platform key is addressing metadata, not rendered identity: a foreign
platform alone never rejects reproducible pixels.

# struct `DecodedPng`

Decoded PNG bytes with dimensions for pixel comparison.

# field `DecodedPng::width`

Decoded image width in pixels.

# field `DecodedPng::height`

Decoded image height in pixels.

# field `DecodedPng::pixels`

Row-major 8-bit RGBA pixels.

# fn `decode_rendered_png`

Decode freshly rendered PNG bytes: failures are render failures, since
the upstream renderer just encoded these bytes.

# fn `decode_stored_png`

Decode the stored candidate PNG bytes: failures are image errors. These
bytes already passed [`Sample::from_bytes`] validation, so failure here
is unreachable outside memory corruption.

# fn `decode_png`

Decode 8-bit RGBA PNG bytes with their dimensions.

# fn `ci_is_set`

True when the `CI` variable is present with any value, including empty.

Any presence freezes approval: there is no override, and ambient
`BLESS`, `UPDATE`, and `INSTA_UPDATE` variables are ignored outright
(they are never read).

# fn `require_no_ci`

Refuse approval while the `CI` variable is set, before any write.

# fn `require_digest`

Validate one mandatory digest argument: missing, malformed, and zero
placeholder digests are rejected as invalid sample-selection input.
Only the expected baseline digest may carry the literal `missing`.

# fn `is_hex_digest`

True for 64 lowercase hex characters: a SHA-256 digest shape.

This mirrors the frozen comparison helper so approval validates digest
arguments without depending on comparison internals.

# fn `sha256_hex`

SHA-256 hex over raw bytes: the candidate and baseline keying.

# fn `read_candidate_bytes`

Read the candidate file once through the shared bounded read: the file
is read-only input, and bytes past the sample limit fail as a resource
limit before validation runs. An unreadable candidate names its path in
the error.

# fn `resolve_baseline_root`

Resolve a baseline root: anchor a relative root at the working
directory, then canonicalize it, or its nearest existing ancestor with
the missing remainder appended lexically. A `..` remainder is rejected
rather than resolved.

# fn `ensure_no_symlink`

Reject symlinks in every existing prefix of a store-relative path, so a
link can never redirect a baseline or evidence write outside its root.

# struct `TempFile`

A temporary file that removes itself on drop unless committed. Writes go
to the destination directory, flush before the handle closes, and reach
their name through one atomic rename.

# fn create

Create a uniquely named temporary file in a directory.

Exclusive creation with owner-only mode: the call never follows a
symlink (a planted link at the fresh name fails creation), and the
file is never truncated or reused.

# fn commit

Flush, close, and atomically rename the temporary file.

Both paths share one directory, so the rename cannot cross devices:
there is no copy fallback (a copy would not be atomic), and any
rename failure removes the temporary file and reports I/O.

# struct `ChangeSet`

The change summary between the candidate and the previous baseline, in
comparison order: the smallest difference the frozen comparison reports.
The summary rides in the returned outcome; it is never persisted.

# fn empty

An empty change set: the candidate already equals the baseline.

# fn missing

The change set for a first approval: no baseline to compare, so
the single changed field is the missing baseline itself.

# fn `from_present`

The change set from a present-baseline comparison, verbatim. The
comparison never ran test-ID or gate cross-comparison; a pass is an
empty change set.
