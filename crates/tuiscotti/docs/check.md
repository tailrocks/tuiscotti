# module check

Frozen comparison and candidate evidence (D03, GOAL 14).

[`FrozenStore`] compares a candidate [`Sample`] against the approved
baseline for its test ID without changing the baseline root. On any
mismatch it saves the candidate bytes and the derived evidence under the
artifact root, keyed by the compared baseline digest, and returns a
[`CheckFailure`] with the candidate path and both digests.

Comparison order (GOAL 14.2): read and validate the actual sample, read
and validate the baseline when present (`Missing` without making the
baseline directory otherwise), require equal test IDs and gates, compare
canonical snapshot state first, then the render contract, then upstream
`compare_rgba` over DECODED RGBA pixels for `state-and-pixels`. Equal
pixels never hide a state difference: state is compared first, and the
`state` gate ignores image and render metadata entirely.

Store note: these stores hold local project data. They are not a hostile
multi-user filesystem service. Store-relative write paths are still
checked for symlinks so a link can never redirect evidence outside the
store roots, and equal or contained baseline and candidate roots are
rejected before any comparison.

# const `CHECK_SCHEMA`

Schema identifier for `result.json` files written beside the evidence.

# const `MISSING_BASELINE`

Literal baseline digest recorded when a check finds no baseline file.

# const `MAX_REPORTED_CELLS`

Cap on changed cells kept in a failure: the total is exact, the kept
detail is row-major deterministic and bounded for report size.

# enum `CheckError`

Every failure a frozen check or evidence write can report.

# variant `CheckError::InvalidTestId`

A test ID breaks the Section 14.1 grammar.

# variant field `CheckError::detail`

What the test ID broke.

# variant `CheckError::InvalidStorePath`

Baseline and artifact roots are equal, contained, or a
store-relative path leaves its root through a symlink.

# variant field `CheckError::detail`

What the path broke.

# variant `CheckError::Sample`

The candidate or baseline sample fails [`Sample`] validation.

# variant `CheckError::Pixel`

Upstream pixel comparison rejects decoded image buffers.

# variant `CheckError::EvidenceCorrupt`

Saved candidate or evidence bytes fail re-validation.

# variant field `CheckError::detail`

Which saved file failed and how.

# variant `CheckError::Image`

A difference image fails PNG encoding.

# variant field `CheckError::source`

The PNG encoding failure.

# variant `CheckError::Io`

A filesystem failure outside sample validation.

# variant field `CheckError::source`

The filesystem failure.

# variant `CheckError::Report`

A static HTML report fails to render or save.

# enum `MismatchReason`

Structured mismatch reason with one value per failure class (GOAL 14.2).

# variant `MismatchReason::Missing`

No baseline file exists for the test ID.

# variant `MismatchReason::TestId`

Candidate and baseline carry different test IDs.

# variant `MismatchReason::Policy`

Candidate and baseline carry different gates.

# variant `MismatchReason::State`

Canonical snapshot state differs.

# variant `MismatchReason::Contract`

Render contracts differ with equal state.

# variant `MismatchReason::Dimensions`

Decoded image dimensions differ with equal state and contract.

# variant `MismatchReason::Pixels`

Decoded RGBA pixels differ with equal dimensions.

# fn `as_str`

The wire value recorded in `result.json` and the HTML report.

# struct `CheckSuccess`

A passing comparison: the candidate digest and the compared baseline
digest (`missing` never appears here: a missing baseline fails).

# fn `test_id`

The compared test ID.

# fn `candidate_sha256`

SHA-256 over the candidate sample file bytes.

# fn `baseline_sha256`

SHA-256 over the compared baseline file bytes.

# struct `CheckFailure`

A failing comparison with the saved candidate and evidence locations.

# fn `test_id`

The compared test ID.

# fn gate

The gate both samples carry.

# fn reason

The structured mismatch reason.

# fn `candidate_sha256`

SHA-256 over the candidate sample file bytes.

# fn `baseline_sha256`

SHA-256 over the compared baseline bytes, or `missing`.

# fn `candidate_path`

Where the candidate bytes were saved (never a shared mutable file).

# fn `evidence_dir`

The `checks/<baseline_sha256_or_missing>` directory for this run.

# fn `report_path`

The static HTML report for this comparison.

# fn `changed_fields`

Changed top-level state fields, in fixed order.

# fn `changed_cells`

Changed cells with coordinates, row-major, bounded (see the total).

# fn `changed_cell_total`

The exact changed-cell count, without the report bound.

# fn `pixel_changed`

Changed pixel count from upstream `compare_rgba` (zero when pixels
were not compared).

# fn `pixel_compared`

Compared pixel count (zero after a dimension mismatch).

# fn `pixel_bounds`

Smallest changed rectangle as `(min_x, min_y, max_x, max_y)`.

# enum `CheckOutcome`

The outcome of [`FrozenStore::check`]: a pass or a saved failure.

# variant `CheckOutcome::Pass`

The candidate equals the baseline under its gate.

# variant `CheckOutcome::Failure`

The candidate differs; evidence is saved under the artifact root.

# fn `is_pass`

True only for an equal comparison.

# struct `FrozenStore`

A read-only comparison handle over a baseline root and an artifact root.

Checking never writes to the baseline root: no baseline file, no
directory, no lock file. Failure evidence goes to the artifact root.
There is no approval method here by design (D04 owns approval).

# enum `PresentComparison`

What D04 approval records for a present baseline: pass, or the change
summary for the approval outcome.

No test-ID or gate cross-comparison shapes this value: the
[`FrozenStore::check`] `TestId`/`Policy` arms never run here. Only snapshot state, render
contracts, and decoded pixels do. No evidence files are written; the
caller already holds the approval lock over both inputs.

# variant `PresentComparison::Pass`

The candidate equals the baseline under the compared evidence.

# variant `PresentComparison::Failure`

The candidate differs; the reason and fields mirror [`CheckFailure`].

# variant field `PresentComparison::reason`

The structured mismatch reason (`State`, `Policy`, `Contract`,
`Dimensions`, or `Pixels`; never `Missing` or `TestId`).

# variant field `PresentComparison::changed_fields`

Changed top-level fields, in fixed order.

# fn `from_state`

Wrap one state diff under its reason.

# fn new

Open a frozen store over two roots.

Both roots resolve through their nearest existing ancestors, then
equal roots and contained roots are rejected in either direction.
The artifact root is created when missing; the baseline root is
never created here (a missing baseline root makes every check
`Missing`).

# Errors

Returns [`CheckError`] for empty, equal, contained, or
unresolvable roots, and for artifact-root creation failures.

# fn `baseline_root`

The canonical baseline root.

# fn `artifact_root`

The canonical artifact root.

# fn check

Compare candidate sample bytes against the stored baseline.

On failure the candidate and derived evidence are saved and a
[`CheckOutcome::Failure`] is returned. The baseline root is never
written. A missing baseline file is a `Missing` failure without
making the baseline directory.

# Errors

Returns [`CheckError`] for invalid samples, test IDs, store paths,
corrupt saved evidence, or I/O failures.

# fn `check_file`

Read a sample file with the shared bounded read, then
[`FrozenStore::check`] it.

# Errors

Returns [`CheckError`] when the file is unreadable or the check fails.

# fn `compare_with_baseline`

Compare against present baseline bytes after both sides validate.

# fn `compare_pixels`

Compare render contracts, then decoded RGBA pixels. State is already
equal here, so equal pixels can never hide a state difference.

# fn `compare_present`

Compare a validated candidate against known-present baseline bytes
for the D04 approval change summary.

Both samples are already parsed and validated by the caller; this
only diffs. A state-gate candidate passes on equal snapshots (the
`state` gate ignores image and render metadata entirely). A
baseline without image evidence records the gate shape without
comparing gate enums. Otherwise render contracts then decoded pixels
decide, exactly as in [`FrozenStore::check`]: equal snapshots with
differing contracts report `Contract`, mirroring the frozen
comparison, so check and accept cannot disagree on the change set.

# fn `fail_without_baseline`

Record a `Missing` failure: candidate plus evidence, no baseline dir.

# fn `baseline_path`

Resolve the baseline file for a test ID inside the baseline root.

# fn `missing_baseline_path`

Resolve a not-yet-existing baseline path: the nearest existing
ancestor must stay inside the baseline root.

# const `MAX_DISPLAY_CHARS`

Maximum display characters kept per compared value in reports.

# const `CELL_FLAGS`

Cell attribute flags compared field by field, in fixed report order.

# struct `StateDiff`

Pure state-comparison detail shared by every failure record and report.

# fn named

A non-state reason with one named field and no cell detail.

# fn pixels

A pixel reason: no state fields changed, only pixel counts.

# fn `is_equal`

True when neither a field nor a cell changed.

# struct `DiffPng`

Decoded difference image in row-major RGBA order with its dimensions.

# enum `PixelVerdict`

The pixel comparison verdict: equal, dimension-mismatched, or different.

# variant `PixelVerdict::Equal`

Decoded RGBA pixels are equal.

# variant `PixelVerdict::Dimensions`

Decoded image dimensions differ.

# variant `PixelVerdict::Different`

Pixels differ with counts, bounds, and a difference image.

# fn `decide_pixels`

Decode both images and run upstream `compare_rgba` over DECODED RGBA,
never PNG bytes. State and contract are already equal at this point.

# fn `validate_test_id`

Validate a test ID against the Section 14.1 grammar on every platform
(see [`crate::sample`] for the shape rule).

The grammar lives in one place: this check delegates to the sample
module's complete validator, so construction, parsing, and checking
cannot disagree on the shape.

# Errors

Returns [`CheckError::InvalidTestId`] for every rejected shape.

# fn `diff_snapshots`

Compare two snapshots through canonical equality, collecting changed
top-level fields (fixed order) and changed cells (row-major).

# fn `display_value`

Truncate a compared value to a fixed display length on a char boundary.

# fn `diff_cursors`

Compare cursor state field by field; an absent cursor difference keeps
the field list stable.

# fn `diff_cells`

Compare the union of occupied cells; cells outside the shared area
count as changed. Iterating `max(cols) x max(rows)` would visit every
cell outside both grids (up to 1B entries at the dimension limit) and
report each empty `(None, None)` pair as a changed cell, so each row
only covers the columns occupied on either side and empty pairs are
skipped. The total stays exact while the kept detail is row-major
deterministic and bounded for report size.

# fn `changed_pair`

The changed-cell record for one occupied coordinate, or `None` for an
empty `(None, None)` pair and for an equal pair.

# fn `diff_cell`

Compare two cells through read-only accessors, in fixed field order.

# fn `resolve_root`

Resolve a store root: anchor a relative root at the working directory,
then canonicalize it, or its nearest existing ancestor with the missing
remainder appended lexically. A `..` remainder is rejected rather than
resolved.

# fn `ensure_no_symlink`

Reject symlinks in every existing prefix of a store-relative path, so a
link can never redirect a write outside its store root.

# fn `sha256_hex`

SHA-256 hex over raw bytes.

# fn `decode_sample_png`

Decode an embedded sample PNG with dimension validation before the pixel
buffer is allocated. Returns raw bytes, width, height, and RGBA pixels.

# fn `check_png_dims`

Enforce image dimension limits on numbers, before any allocation.

# fn `byte_total`

Checked `width * height * 4` for an RGBA8 buffer length.

# fn `encode_diff_png`

Encode a difference image as PNG bytes for evidence.

# fn `write_failure`

Save the candidate and derived evidence, then describe the failure.

Evidence is keyed by the compared baseline digest (`missing` when
none): `artifacts/<id>/<candidate>/checks/<baseline>/`. Saved
candidate and evidence bytes are re-validated before reuse; corrupt
or digest-mismatched files are a typed error, never a silent
overwrite. The baseline is never written here.

# fn `candidate_dir`

Resolve the candidate directory, rejecting symlink redirection.

# fn `relative_dir`

The store-relative check directory for one baseline comparison.

# fn `save_candidate`

Save the candidate bytes once; re-validate them on later checks.

# fn `save_expected`

Save the compared baseline bytes (`expected.tui.json`); never read a
newer baseline during later reporting, never overwrite another
digest's evidence.

# fn `save_images`

Save `expected.png`, `actual.png`, and `diff.png` for pixel gates.
State gates carry no images; a missing baseline has no expected one.
A baseline without an image (a state-only sample at a pixel
candidate's baseline path) likewise yields no `expected.png`: the
failure still records with full evidence instead of erroring.

# struct `SavedImages`

Which sibling images a report may link to.

# fn none

No images: state gates and missing baselines.

# fn `validate_saved_report`

Re-validate a saved report before reuse: the existing path must be a
regular file and its bytes must equal the deterministically generated
expected HTML. Any disagreement is corrupt evidence, never silently
trusted and never overwritten: conflicting immutable evidence fails the
check instead.

# fn `validate_saved_sample`

Re-validate saved sample bytes before reuse: they must parse and their
digest must match the key that stored them.

# fn `sample_png_bytes`

Raw PNG bytes embedded in validated sample bytes.

# fn `save_png`

Save PNG bytes, validating any present file by decoding it.

# fn `save_png_bytes`

Save encoded difference bytes, validating any present file by decoding.

# struct `CheckSchema`

The schema identifier as an explicit type: only [`CHECK_SCHEMA`] parses.

# struct `CellRecord`

One changed-cell record in `result.json`: coordinates plus field names.

# struct `BoundsRecord`

One pixel-bounds record in `result.json`.

# struct `TitleValues`

Compared window titles in `result.json`, display-truncated.

# struct `ResultRecord`

The versioned comparison record saved as `result.json` beside the
evidence. Evidence keying, report rendering, and re-validation all read
this record back; unknown fields and unsupported schemas are rejected.

# fn `for_failure`

Build the record for one failure.

# fn save

Save the record, or re-validate a present one against this failure.

# fn `validate_saved`

Re-validate a saved record: schema, identity, reason, and digests.

# fn `report_input`

The report input for this failure.

# fn `is_hex_digest`

True for 64 lowercase hex characters: a SHA-256 digest shape.

# const `APPROVAL_LOCK_FILE`

File name of the per-root approval lock inside the baseline root.

The lock file is STABLE: it is created once when missing and kept after
every release. It is never unlinked, never truncated, and never created
anew at a held path. Mutual exclusion is the OS file lock
(`flock`/`LockFileEx` through `fs2`) on the open handle, held from lock
acquisition (transaction step 4) through the baseline read, digest
comparison, and atomic replacement, and released at step 19 when the
guard drops. A stale file from a dead process never blocks: the file
carries no lock state, so the next holder opens it and locks it
promptly. Locks serialize holders across processes as well as threads.

# const `APPROVAL_LOCK_TIMEOUT_SECS`

How long approval waits for a held lock before it reports a timeout.

# const `APPROVAL_LOCK_POLL`

Poll interval while approval waits for a held lock to release.

# struct `ApprovalFileLock`

A held per-root approval lock.

The guard owns the open handle on the stable lock file: dropping it
releases the OS lock while the file persists for the next holder.

# enum `ApprovalLockError`

Why approval lock acquisition failed.

# variant `ApprovalLockError::Timeout`

Another holder kept the OS lock past the acquisition deadline.

# variant `ApprovalLockError::Io`

The lock file could not be opened or created.

# fn acquire

Acquire the approval lock for a canonical baseline root.

The stable file is opened or created (never exclusively: a held path
is opened, not recreated), then the OS lock is taken with
non-blocking attempts until `timeout`. A directory at the lock path,
a read-only root, or a missing root fails fast with
[`ApprovalLockError::Io`] without waiting. This call never creates
directories; the root must already exist.

# fn `open_lock_file`

Open or create the stable lock file without truncating it.

Plain creation (not `create_new`): an existing file — held or stale — is
opened as is, so a held path is never recreated and a stale file keeps
no lock state. Owner-only permissions apply when the file is created.
