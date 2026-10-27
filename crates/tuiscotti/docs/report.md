# Module

Static HTML comparison reports (D03, GOAL 14.3).

[`render_report`] builds a static HTML page from saved comparison data:
the test ID, mismatch reason, and digests; expected, actual, and
difference images when present (as relative sibling links); cell
coordinates and changed state fields for state differences; and an
explicit note when no expected image exists.

The output carries no code from samples and no web server or script:
text and attribute values are escaped, every href segment is
percent-encoded, cross-drive targets become `file://` URLs, and no
absolute developer path is ever written.

[`write_report_index`] builds the static evidence index over an
artifact root; its links resolve from the index parent, so the index
may live anywhere.

# `ReportError`

Every failure a report render or save can report.

# `Io(std::io::Error),`

The report file cannot be written.

# `ReportedCell`

One changed cell for the report: coordinates, changed field names, and
the compared cell text on both sides (truncated for display).

# new

Record a changed cell at grid coordinates with its changed fields
and the compared display text.

# col

The grid column of the changed cell.

# row

The grid row of the changed cell.

# fields

Changed field names for this cell, in fixed order.

# `actual_text`

Display text of the candidate cell.

# `baseline_text`

Display text of the baseline cell.

# `PixelSummary`

Compared-pixel summary for the report.

# pub changed: u64,

Changed pixel count.

# pub compared: u64,

Compared pixel count.

# pub bounds: [u32; 4],

Smallest changed rectangle as `(min_x, min_y, max_x, max_y)`.

# `ReportInput`

Input for [`render_report`]: plain borrowed data, no filesystem reads.

# `pub test_id: &'a str,`

The compared test ID.

# pub gate: &'a str,

The gate wire value (`state` or `state-and-pixels`).

# pub reason: &'a str,

The mismatch reason wire value.

# `pub candidate_sha256: &'a str,`

SHA-256 over the candidate sample bytes.

# `pub baseline_sha256: &'a str,`

SHA-256 over the baseline bytes, or `missing`.

# `pub changed_fields: &'a [String],`

Changed top-level state fields, in fixed order.

# `pub changed_cells: &'a [ReportedCell],`

Changed cells with coordinates, row-major, bounded.

# `pub changed_cell_total: usize,`

The exact changed-cell count, without the report bound.

# `pub pixels: Option<PixelSummary>,`

Pixel summary when pixels were compared.

# `pub has_expected_image: bool,`

Whether `expected.png` sits beside this report.

# `pub has_actual_image: bool,`

Whether `actual.png` sits beside this report.

# `pub has_diff_image: bool,`

Whether `diff.png` sits beside this report.

# `pub title_values: Option<(&'a str, &'a str)>,`

Compared window titles as `(actual, baseline)` when the title moved.

# `IndexOutcome`

One completed evidence index: the static page linking every saved
check record.

# records

The indexed check record count, in deterministic sort order.

# `IndexError`

Every failure an evidence index write can report.

# Read {

The artifact root cannot be walked for check evidence.

# `artifacts: PathBuf,`

The artifact root that failed to list.

# `source: std::io::Error,`

The filesystem failure.

# Write {

The static index page cannot be written.

# `path: PathBuf,`

The index path that failed.

# `source: std::io::Error,`

The filesystem failure.

# code

The stable Section 16.3 machine code: every index failure is `IO`.

# `write_report_index`

Walk the artifact root for check `result.json` records (schema
`tuiscotti.check.v1`), sort them deterministically, and write one
self-contained HTML page linking each check directory's own
`report.html`. Links resolve from the index parent, so the index may
live anywhere; only forward slashes are emitted. Nothing here
modifies approvals: the baseline root is never opened.

# Errors

Returns [`IndexError`] when the artifact root cannot be walked or
the index page cannot be written.

# `IndexedRecord`

One indexed check record: identity plus its relative check directory.

# `collect_records`

Collect check records under `artifacts`, sorted by test ID, candidate
digest, and baseline digest for deterministic output.

# `read_record`

Read one check record; returns `None` for files that are not check
evidence (foreign files never break the index).

# `render_index`

Render the static index page: escaped text, links relative to the
index parent, no absolute developer paths and no scripts.

# `push_index_line`

Append one `\n`-terminated line to an index buffer.

# `relative_href`

Href from the index parent to an evidence file, with forward
slashes on every platform. Both sides resolve lexically against the
process directory, so no filesystem access decides the spelling.
Every segment is percent-encoded, so `#`, `?`, `%`, spaces, and
non-UTF8 names survive the browser intact. Targets without a relative
spelling (a different drive root) become `file://` URLs.

# absolutize

Join a relative path onto the process directory; absolute paths
pass through unchanged.

# `normalize_lexical`

Resolve `.` and `..` lexically, without touching the filesystem.

# `relative_lexical`

Lexical path from `base` to `target`, joined with forward slashes;
`None` when the two sides share no comparable root (drive-letter
splits on Windows).

# `encode_href_segment`

Percent-encode one href segment from its raw bytes: unreserved marks
and sub-delimiters travel literally, everything else (including `%`,
`#`, `?`, spaces, and non-UTF8 bytes) travels as uppercase `%XX`.

# `file_url`

`file://` URL for a target with no relative spelling: every segment
travels percent-encoded, so the fallback stays a valid browser URL
instead of a bare path that reads as a scheme.

# `render_report`

Render the static HTML report for one comparison as a string.

# `write_report`

Render and save the static HTML report through one atomic file write: a
temporary file in the destination directory, flushed and closed before
one atomic rename, like all sibling evidence.

# Errors

Returns [`ReportError`] when the report path has no file name or the
report file cannot be written.

# `TempFile`

A temporary file that removes itself on drop unless committed. Writes go
to the destination directory, flush before the handle closes, and reach
their name through one atomic rename.

# `MAX_TEMP_CREATE_ATTEMPTS`

Staging attempts before an occupied directory reports `AlreadyExists`.

# create

Create a uniquely named temporary file in a directory.

Exclusive creation with owner-only mode: the call never follows a
symlink (a planted link at the fresh name fails creation), and the
file is never truncated or reused. Occupied names retry with a fresh
counter value; every other I/O failure returns immediately.

# `create_retry`

Stage through caller-supplied fresh names; the closure hook exists so
tests can force the collision path deterministically.

# `create_once`

Open one staging name exclusively: `create_new` fails when anything
(file, directory, or symlink) already occupies the path, so creation
never follows a link. All later bytes travel through the held
handle, and the final rename moves the staged path itself.

# commit

Flush, close, and atomically rename the temporary file.

The staged path is retained until the rename succeeds: a failed
rename removes the staged file and reports I/O, so no `.tmp-*`
residue survives the error branch.

# `atomic_write`

Write bytes through a temporary file with flush, close, and atomic
rename. Every evidence write, including the HTML report, goes through
this helper so readers never see a partial file.

# `escape_html`

Escape text and attribute values for HTML: no code from a sample runs.

# `push_header`

Write the document head: escaped title, no script, no absolute path.

# `push_summary`

Write the outcome summary: reason, gate, and both digests escaped.

# `push_state`

Write changed fields, values, and cell coordinates with an exact total.

# `push_title_values`

Write the compared window titles when the title moved.

# `push_cells`

Write the changed-cell table with coordinates and display text.

# `push_pixels`

Write the compared-pixel summary when pixels were compared.

# `push_images`

Write image links with relative sibling names, or explicit missing notes.

# `push_image`

Write one image row: a relative link when present, an explicit note when
no expected image exists or an image does not apply to this gate.

# `push_footer`

Close the document: no JavaScript application follows.
