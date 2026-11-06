# Compressed Protobuf baselines

## Status and scope

This is a specification for future implementation. The shipping format is still
`tuiscotti.sample.v1` JSON. **MUST** identifies a release requirement; an open
choice below is not permission to select a default without evidence.

The target storage format is Protobuf compressed with Zstandard. New samples and baselines MUST
use binary files. Remove JSON sample and baseline reading and writing in the
same release. There is no deprecation period or JSON compatibility mode.

The change covers saved samples, baseline storage, regeneration, and the existing
Rust and offline CLI consumers. Baselines are regenerated, not converted. It
does not add a terminal engine, renderer,
visual review application or GitHub publishing. Downstream repository upgrades
are a separate rollout after this change merges. Existing JSON sample evidence
cannot be loaded by the new version. JSON result records, receipts, configuration,
CLI output, and the private state-digest projection remain supported. Updating
existing evidence-loading paths for binary
samples is in scope; designing new review tools is not.

The boundaries in [ARCHITECTURE.md](ARCHITECTURE.md), the comparison and approval
contract in [SNAPSHOTS.md](SNAPSHOTS.md), and the test-ID grammar remain binding.
`FrozenStore::check` MUST remain read-only with respect to baselines. Only
`ApprovalStore::accept` / `tuiscotti accept` may write active baselines, after
re-verification, and they MUST refuse whenever `CI` is set, including an empty
value. No ambient bless setting or migration exception may bypass that rule.

## Motivation and evidence

The existing proposal recorded 1,978 tracked baseline files containing
2,043,838,901 bytes at commit
`60b5fb2762d91a3549bd4ac63b1d77d534ee53d2` of the
[terminal-components-baseline repository](https://github.com/tailrocks/terminal-components-baseline).
Those files occupy about 1.9 GiB in a checkout. One sampled 1,126,784-byte JSON
capture became 10,316 bytes with gzip. These are motivating observations from
the proposal, not measurements of the proposed format or a reproducible
corpus-wide benchmark.

Repeated cell fields suggest compression can reduce checkout bytes. Git already
compresses objects and deltas, so checkout savings MUST NOT be presented as Git
history savings. Protocol Buffers and compression MUST be measured separately. Compressed JSON is
a benchmark control, not a proposed storage format. Zstandard is the required
codec. Its compression level and measured savings remain open.

## Logical sample and losslessness

The public validated domain remains `Sample` and upstream `TerminalSnapshot`.
Protobuf messages MUST be private persistence types, not replacement public
terminal types. The Protobuf mapping MUST use public upstream accessors and validated
construction/deserialization; it MUST NOT depend on upstream private layout,
reimplement terminal capture, or re-export live APIs.

Every field in the current sample domain MUST survive a domain → binary → domain
round trip for codec verification. This does not require a baseline converter.
The following inventory is the minimum schema coverage, not a list
of fields eligible for omission:

| Object | Fields and distinctions to preserve |
|---|---|
| Sample | Logical schema, `test_id`, gate, snapshot, optional render contract, optional image, observation. |
| Snapshot | Upstream schema, `cols`, `rows`, scope, full `u64` viewport origin, row-major cells, all 256 ordered palette entries, default foreground/background/cursor, cursor, optional title, metadata. |
| Cell | UTF-8 text, recorded `u16` width, foreground/background color kinds and values, bold, dim, italic, inverse, invisible, strike, blink, underline style/color, optional link URI and link ID. |
| Color | Default, indexed 0–255, and explicit RGB remain different even if they paint identically; RGB channels retain their exact bytes. |
| Cursor | `col`, `row`, `in_capture`, visible, shape, RGB color, optional blink intent. Coordinates and clipping intent MUST NOT be recomputed. |
| Snapshot metadata / provenance | `captured_at_ms`, revision, backend, session, source; retain the values in the sample being encoded; do not substitute local values. |
| Render contract | Profile, contract version, renderer version, raster policy, ordered fonts, cell width/height, font size, baseline, scale, title bar, palette digest, cursor phase, optional blink, platform key. |
| Contract font | Digest, face index, bold, italic, fallback position and ordering. |
| Image | PNG format, dimensions, both digest values, exact encoded PNG bytes; decoding those bytes MUST yield identical RGBA. |
| Sample observation | Optional note, including absent versus present empty text. |

Absent, present-empty, present-zero, and present-false MUST remain distinct where
the domain distinguishes them. Preserve all `u64` values without a signed or
floating-point intermediate. Preserve exact accepted `f32`/`f64` values,
including signed zero; do not round geometry or coerce `f64` to `f32`. Do not
normalize Unicode, collapse whitespace, resolve indexed colors to RGB, trim
blank cells, infer widths, reorder fonts, or regenerate PNGs during encoding or decoding.

Full preservation MUST be tested using a complete field projection, plus bitwise
checks for floating-point fields and exact PNG bytes. `TerminalSnapshot::PartialEq`
and state equality exclude metadata, so equality alone is insufficient proof. The existing
`SampleImage::png_base64` view MUST reconstruct the same validated value from
stored PNG bytes.

## Container and wire versions

A binary file MUST contain exactly one header followed by exactly one compressed
Protobuf message. It MUST NOT be an archive, a stream of independent samples,
or a wrapper carrying JSON as the principal snapshot payload.

The proposed 32-byte header is fixed for container version 1:

| Offset | Length | Value |
|---|---:|---|
| 0 | 8 | Magic bytes `54 55 49 53 43 50 42 00` (`TUISCPB` followed by NUL). |
| 8 | 2 | Container version, unsigned little-endian; initially 1. |
| 10 | 2 | Codec identifier, unsigned little-endian; `1` means Zstandard. |
| 12 | 4 | Protobuf message schema version, unsigned little-endian; initially 1. |
| 16 | 8 | Exact decompressed message length, unsigned little-endian. |
| 24 | 8 | Exact compressed payload length, unsigned little-endian. |

Container version, Protobuf schema version, the logical
`tuiscotti.sample.v1` identifier, upstream snapshot schema, render-contract
version, and codec identity are independent. A matching logical schema MUST NOT
make an unknown container or message version acceptable. Version 1 has no flags,
external dictionary, optional header extension, or timestamp. Codec ID 0 is
reserved for uncompressed benchmark/control data and MUST NOT become an
undocumented production fallback. Zstandard framing, checksum requirements, encoder settings and compatibility
policy MUST be recorded in
[DECISIONS.md](DECISIONS.md) before format release.

Readers MUST identify binary data by magic and validate the whole header. JSON
samples, unknown binary data, truncated magic/header, and unsupported
schemas/codecs MUST fail closed. None may become a comparison pass. Extension
alone MUST NOT decide decoding.

The implementation MUST commit a reviewed `.proto` schema and tag inventory.
Top-level tags are: logical schema 1, test ID 2, gate 3, snapshot 4, render
contract 5, image 6, observation 7. Nested messages MUST explicitly cover the
inventory above. Colors MUST use a tagged union; optional strings and booleans
MUST carry presence. Required domain scalars MUST carry presence and reject
absence instead of inheriting a Protobuf zero default. Enum zero MUST be an
unspecified sentinel and reject; unknown enum values and unset/conflicting
unions MUST reject. Integer narrowing MUST be checked.

Tags and enum numbers MUST never be reused; removed identifiers MUST be reserved.
Version 1 readers MUST reject unknown fields at every nesting level and duplicate
singular fields, including repeated assignments of the same union alternative.
A generic decoder's normal unknown-field skipping or last-value-wins behavior is
insufficient. Adding a field requires a new supported message version and an
explicit compatibility decision. Readers MUST NOT silently discard newer data
and subsequently approve the reduced sample.

Writers MUST emit fields in ascending tag order, ordered collections in their
domain order, canonical varints, and one pinned representation for repeated
scalars. Maps are forbidden in canonical messages. Readers may accept legal
field order and packed/unpacked repeated-scalar forms, but MUST reject invalid
wire types, overflows, invalid UTF-8, malformed lengths, forbidden duplicates,
and groups. Generated-code/runtime versions and compression settings MUST be
pinned. Repeated encoding of the same complete sample, including provenance,
MUST produce identical bytes across supported platforms. Protobuf's name alone
does not establish canonical encoding; golden bytes MUST establish it.

## Comparison and digest contracts

The binary implementation MUST produce the same pass/failure category,
mismatch reason, changed fields/cells, pixel counts and bounds as the current
implementation for equivalent logical inputs. A pre-change test oracle may
establish those results without adding JSON support to the new implementation. The
existing ordering of failures MUST remain: test ID, gate policy, canonical
snapshot state, render contract, image dimensions, decoded pixels. Preserve the
current approval change-summary behavior separately; do not use the format
change to alter existing comparison policy.

State comparisons include the complete canonical snapshot and exclude snapshot
metadata and sample observation. `state-and-pixels` additionally compares the
existing render contract and decoded RGBA. Encoded PNG or compressed-file bytes
MUST NOT replace pixel equality. Gate changes MUST remain policy failures; the
`pixels-<os>-<arch>` suffix MUST continue to match the recorded contract platform,
so historical samples can parse on another platform.

The following identities MUST remain separate:

| Identity | Required behavior |
|---|---|
| Candidate / baseline SHA-256 | Hash the exact saved file bytes, including binary header and compressed stream. Regeneration changes this identity even when comparison passes. |
| `Sample::state_digest` | Keep the current `tuiscotti-state-v1` algorithm and bytes, independent of persistence encoding. Retain its existing metadata exclusions and contract/pixel inputs. Do not silently replace it with a Protobuf hash. |
| PNG / RGBA digests | Preserve and verify the current hashes over exact PNG bytes and decoded RGBA respectively. Compression is outside both scopes. |

When checks save candidate evidence, they MUST preserve the exact input bytes
and report their actual hash. Ordinary acceptance MUST re-read and hash both current files while
holding the stable approval lock, reject stale values, resolve the candidate's
profile, verify render identity and pixels as today, then atomically install the
exact candidate bytes. Consequently accepted baseline SHA-256 MUST still equal
candidate SHA-256. Acceptance MUST reject a JSON candidate. Existing evidence and approval invocations remain bound to their
original bytes; do not rewrite old digests or receipts to imply that they
approved newly generated bytes.

## Bounded and safe decoding

The input is untrusted in every Rust, CLI, check, accept, render and
existing report-loading path. All paths MUST share the same validation and
limits; a secondary decoder MUST NOT bypass them.

| Resource | Version 1 requirement |
|---|---|
| Complete encoded input | At most 64 MiB, including binary header, using bounded reads. |
| Decompressed Protobuf | At most 64 MiB; stream into a bounded sink and check actual output, not only the claimed length. |
| Decoder nesting | At most 32 message levels; recursive groups are forbidden. |
| Codec working window | At most 32 MiB; reject frames requesting a larger window before allocating it. |
| Grid | Nonzero dimensions, at most 32,768 per dimension and 1,000,000 cells, checked multiplication, exact row-major cell count. |
| Cell and links | At most 4,096 UTF-8 bytes per text, link URI and link ID. |
| Title | At most 65,536 UTF-8 bytes. |
| Test ID | Existing grammar, at most 240 ASCII bytes total and 64 per segment, including platform/device-name restrictions. |
| Image | At most 32,768 pixels per dimension and 256 MiB decoded RGBA; checked arithmetic and validation of actual PNG dimensions before pixel-buffer allocation. |

The 64 MiB message cap MUST be checked against all supported legacy samples
before release. If a valid legacy sample exceeds it after encoding, this blocks
release until the cap and memory budget are explicitly revised; silently
excluding previously supported values is unacceptable.

The decoder MUST validate header lengths with checked arithmetic, enforce exact
payload and output lengths, consume exactly one codec frame, and reject trailing
bytes, concatenated frames, truncation, corrupt checksums and partial output.
It MUST reject external dictionaries and never read dictionary/font resources
from an input-controlled path or the network. A compression-ratio heuristic
MUST NOT replace absolute output and window bounds: valid repetitive grids can
have very high ratios.

Before owned message allocation, a bounded wire scan MUST validate nesting,
field lengths, element counts, and relevant numeric limits, including cell count
and per-field text limits. It MUST account for accumulated packed and unpacked
repeated fields, not just individual segments. Font lists, metadata strings,
notes and PNG bytes have no new smaller per-field semantic limit; they remain
bounded by the message cap and a checked aggregate allocation budget. The
implementation MUST document and test that budget, including message objects,
PNG decoding, codec state and temporary buffers. Merely limiting compressed
input or trusting a generated decoder is insufficient. Boundary values and
limit-plus-one failures MUST be tested without actually allocating the rejected
claimed sizes.

Format/decompression failures MUST be typed sample errors with source details;
unknown versions/codecs MUST be distinguishable from corrupt encoding and
resource limits. Keep existing public error classifications and CLI meanings:
`UNSUPPORTED_SCHEMA`, `RESOURCE_LIMIT`, image errors, and I/O errors must not
collapse into a comparison mismatch. Approval continues its documented
`INVALID_SAMPLE` mapping for malformed candidates. A rejection MUST NOT create
or replace a baseline or leave a partial accepted file.

## Paths, compatibility, and rollout

The baseline path MUST be `<baseline-root>/<test-id>.tui.pb`. Test-ID grammar,
directory structure, and the gate-to-platform suffix pairing remain unchanged.
The file contains the compressed, versioned container specified above. Checks
and acceptance MUST use only this path and MUST NOT fall back to `.tui.json`.
A JSON file at an input path MUST fail as an unsupported sample. An absent binary
baseline is missing, even if an old JSON file is present. Documentation MUST
warn that leaving old JSON files in a baseline tree does not make them active.

Acceptance MUST compare the expected digest with the binary baseline under the
stable approval lock and atomically install the exact binary candidate bytes.
It MUST NOT rename or delete old JSON files as part of acceptance. Checks never
upgrade a file. Candidate and evidence filenames MUST reflect the binary format.

Ship the reviewed schema, golden fixtures, measured Zstandard settings, binary-only
readers and writers, and regenerated downstream workflow as a breaking change.
Remove JSON sample serialization, deserialization, and their public APIs rather
than retaining a compatibility release. Old binaries cannot read binary samples;
new binaries cannot read JSON samples. Rollback requires the old Tuiscotti version
and the original files and paths.

## Downstream repository rollout

Every repository that uses Tuiscotti JSON baselines MUST be inventoried and
migrated. The inventory MUST identify each baseline root, consumer dependency,
CLI installation method, CI invocation, and any scripts or checks that assume
`.tui.json`. It MUST include
[terminal-components-baseline](https://github.com/tailrocks/terminal-components-baseline)
and record evidence for any other repositories found. Do not assume that the
local checkouts are the complete inventory.

The rollout MUST include these requested targets and recheck their current heads
before making changes:

| Target | Initial inspection |
|---|---|
| [terminal-components-baseline main](https://github.com/tailrocks/terminal-components-baseline) | Commit `2e9e60f53988837c6e1728c71565543edc797d3a` tracks 154 `.tui.json` files. |
| [terminal-components-baseline PR #2](https://github.com/tailrocks/terminal-components-baseline/pull/2) | Head `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2` tracks 1,986 `.tui.json` files, including fixtures. |
| [jackin PR #1264](https://github.com/jackin-project/jackin/pull/1264) | Head `52fabbcca9e9c83c2f712ed9ababdb7e0bb76d87` pins Tuiscotti; its complete Git tree has no tracked `.tui.json` files. Inspect its baseline integration and upgrade its dependency and consumers as needed. |

Counts describe tracked paths at the inspected commits, not necessarily active
baselines. The migration inventory MUST distinguish active baselines from test
fixtures and other JSON, and update fixtures whose behavior depends on the old
format. Remove JSON sample fixtures unless they verify explicit rejection.

After the implementation PR merges, pin each downstream repository to the full
commit SHA of the merged Tuiscotti change before regenerating its baselines. Pin
the CLI used for acceptance and checks to the same revision. Commit the updated
dependency lockfile and the new binary baselines together with changes to
scripts and CI. Do not use a floating branch or a pre-merge commit as the final
pin. Verify that the repository has no active `.tui.json` baselines after
migration and that its normal checks pass against the new files. Preserve
historical receipts unchanged. Inventory each repository's receipt and check
scripts before regeneration. Update current approval metadata to refer to the
new `.tui.pb` paths and exact file-byte digests. Historical receipts do not
approve the new files.

Once the merged revision and downstream migrations are verified, release a new
Tuiscotti version containing that change. Upgrade each downstream repository
from the temporary Git revision to the published version, update its lockfile
and CLI installation, and rerun its normal validation. Record the release
version and the downstream upgrade status in the rollout inventory. A downstream
repository is complete only after it uses the release and its checks pass.
The implementation, merge, release, and downstream changes are future work;
this specification does not itself authorize publishing or alter those repositories.

## Regeneration and approval

Generate fresh samples with the new Tuiscotti version and the repository's normal
capture workflow. Do not implement or run a JSON-to-Protobuf baseline converter.
Each candidate carries the snapshot, render contract, image and provenance from
the new capture. A regenerated sample is not guaranteed to equal the old sample.

Review each new capture against the prior approved evidence using the
repository's review process. The new Tuiscotti version MUST NOT read the old
JSON baseline. If a comparison is needed, use evidence produced by the old
version before upgrading or a separate human review; do not add a converter or
cross-format reader to the new version. Capture or rendering changes, including changes in
provenance, MUST NOT be described as lossless serialization changes. Preserve
historical receipts and their byte digests unchanged.

Approve each binary candidate into a separate, empty staging baseline root using
`ApprovalStore::accept` or `tuiscotti accept` with the candidate's exact digest and
`missing` as the expected baseline. Acceptance retains its workstation-only CI
refusal, mandatory re-render verification and atomic single-file write. It MUST
NOT modify the old baseline root. After all required samples are reviewed, the operator installs each approved
candidate into the destination root through acceptance, verifies its exact bytes,
and removes superseded JSON files as an explicit repository change. This repository change is not an atomic whole-tree
operation and MUST NOT be presented as one. Preserve the old Git revision for
rollback, and do not claim that its receipts approve the new bytes.

Verify the staged tree contains the expected test IDs and no duplicate paths.
Run the normal checks against the staged tree before replacing the old files,
then run them again against the resulting repository tree. If capture, review,
acceptance or validation fails, leave the original baseline tree in place and
resolve the failure before replacement.

## Concurrency and filesystem invariants

All ordinary accepts MUST retain the stable
`.tuiscotti-approval.lock`, existing timeout behavior, stale-candidate and
stale-baseline checks, symlink protections, owner-only exclusive temporary
files and atomic single-file replacement. The lock MUST never be unlinked.
Regeneration and staging MUST NOT bypass the approval lock or write to the
original baseline root.

Concurrent checks of one test ID MUST preserve every binary candidate, including
candidates with equal state but different byte hashes. Gate data
and expected-baseline evidence MUST not overwrite one another. A concurrent
accept/check MUST expose either the complete old or complete new baseline, never
a partial decompression stream. Acceptance with digests observed before any
baseline change MUST fail `STALE_BASELINE`; changing candidate bytes MUST fail
`STALE_CANDIDATE` when the old candidate digest is supplied.

## Rust API and CLI requirements

Existing constructors, accessors, validation, `FrozenStore::{check, check_file}`,
`ApprovalStore::accept`, offline rendering and artifact loading MUST accept the
same validated domain values. `Sample::{from_bytes, from_file}` and all sample
writers MUST support only the compressed Protobuf format. `candidate_id` MUST
keep hashing the supplied bytes without decoding. Remove public JSON sample
serialization and deserialization, including any serde implementation that
provides it. Keep wire types private. `state_digest` MUST retain its existing
algorithm and canonical projection to preserve comparison identity. An internal
projection used solely for that digest is not a supported JSON sample format.
Error types may gain variants without changing established success semantics.

The existing `check`, `accept`, `render` and `report` command arguments and exit
codes MUST remain valid. `--json` stays the `tuiscotti.cli.v1` result schema with
file-byte digests; do not change a field's meaning to a semantic hash. Format
errors exit 3, argument errors 2, differences/missing baselines 1, successful
operations 0. This change MUST NOT add a JSON-to-Protobuf conversion command or
API. Existing capture and acceptance APIs supply the regeneration workflow.

## Benchmark and decision method

The implementation MUST produce a reproducible measurement artifact before
choosing Zstandard settings or claiming benefits. Specify the pinned real corpus
commit and file hashes, include both gates and representative platform contracts,
and stratify by grid size, text/style diversity, Unicode/wide cells, hyperlinks,
metadata and PNG share. Measure the full available corpus as well as per-stratum
results; do not extrapolate one highly repetitive sample. Keep synthetic limit
and incompressible inputs separate from representative results.

Compare at least current canonical JSON, compressed JSON, raw Protobuf and
compressed Protobuf. Test high-compression settings of Zstandard, XZ/LZMA2,
and Brotli, plus gzip as a control. Test multiple relevant settings and record
unsupported candidates and reasons. Use
identical logical samples and unchanged embedded PNG bytes. Include sparse-cell
edits, style/palette changes, metadata-only changes, pixel-heavy changes,
additions/deletions and a real sequence of baseline update commits. Do not create
fake updates by changing only compression settings.

Use the old Tuiscotti version in a separate benchmark worktree or process for
historical JSON controls. Produce Protobuf samples through fresh captures using
the new version, with pinned inputs and rendering settings. Verify equivalence
of the compared logical fields and PNG bytes for paired captures; report any
unpaired or changed captures separately. Do not add a JSON reader or converter
to the new library or CLI for the benchmark.

Record all of the following with reproducible commands and raw data:

- Exact checkout bytes, file counts, migration duplication and retained-original
  costs, per-file distribution, and total size.
- Encode/decode wall time, CPU time, peak process memory, codec window, and
  end-to-end `check`, `accept`, render-read and capture costs. Isolate unchanged
  renderer/PNG work so a small codec difference is not attributed to the renderer.
- Independent Git object databases populated with the same logical update
  sequence: loose/object counts, packed size and deltas after identical documented
  repack settings, clean clone/checkout costs where measured. Report before/after
  migration history as well as fresh-history behavior; recompressing new files
  cannot erase already committed JSON history.
- Hardware, OS/architecture, filesystem/storage, Rust/build profile, dependency
  versions, corpus order, concurrency, warm/cold-cache definitions, repetitions,
  median/tail/dispersion and measurement uncertainty. Verify outputs are consumed
  and all runs succeed; exclude setup/network time consistently.

Before measurement, declare an execution-time and peak-memory budget for the
actual workload and selection criteria for checkout size, Git storage, and processing time.
Choose the strongest measured Zstandard setting that meets the stated resource
limits. Report the smallest result across all candidates even if a different
codec produces it or it fails those limits. Publish the
size/time/memory/Git tradeoffs and the chosen point rather than selecting a
level from checkout size alone. Report compressed JSON as a control even if it
performs better. Compressed Protobuf remains the target. Unmeasured results
MUST remain explicitly unknown.

## Acceptance criteria

Implementation is ready only when all of these are demonstrated:

1. Reviewed schema/tag registry, bounded decoder design, golden container/message
   bytes, deterministic cross-platform encoding and a benchmark-backed pinned
   Zstandard settings are committed with the implementation.
2. Full-field round trips cover every color/enum/attribute, all palette entries,
   width/continuation edge cases, optional presence, full-range integers,
   floating-point bits, metadata/provenance, observations and exact PNG bytes.
   JSON samples are rejected by every sample-reading entry point.
3. Binary checks match recorded pre-change outcomes for passes, missing
   baselines, test-ID/policy/state/contract/dimension/pixel differences,
   diagnostic details and state digests. PNG encodings with equal RGBA remain
   pixel-equal; metadata-only changes remain state-equal with distinct file-byte
   digests. No new runtime JSON reader is used to establish equivalence.
4. Corruption and adversarial tests cover every header boundary, unsupported
   version/codec/field/enum, duplicate singular/union fields, truncated and
   overflowing varints/lengths, invalid UTF-8, concatenation/trailing bytes,
   oversized windows, decompression bombs, malformed PNGs and all limit boundaries.
   Fuzzing the container/wire path demonstrates bounded failure without panic.
5. Real filesystem/process tests demonstrate no baseline writes from check or
   regeneration, CI refusal before approval writes, stale digest behavior,
   concurrent candidate preservation, cross-process accept locking, symlink
   rejection and complete old/new reads during replacement. Acceptance still
   copies exact candidate bytes and retains mandatory render verification.
6. A representative regeneration captures fresh binary samples, reviews them
   against prior approved evidence, approves them into a separate staging root,
   and validates the replacement tree. An isolated rehearsal tree satisfies
   this implementation gate; actual downstream replacement is a later rollout.
   Historical receipts remain unchanged. Failures leave the original baseline
   tree in place until explicit replacement.
7. [API.md](API.md), [SNAPSHOTS.md](SNAPSHOTS.md), [CLI.md](CLI.md), applicable module
   docs and README examples describe the binary format, removal of JSON support,
   limits and regeneration/rollback behavior in the same change.
   New decisions go in [DECISIONS.md](DECISIONS.md); frozen migration history stays
   unchanged. Relative links resolve.
8. Required repository gates pass: locked dependency fetch, format check, offline
   workspace Clippy, offline `cargo nextest run`, library doctests and warning-free
   workspace documentation. No cargo workspace features or forbidden direct
   terminal dependencies are introduced. Integration tests use nextest.

## Decisions still required

Before the binary-only release, resolve and record:

- Zstandard framing, level, checksum and window settings, deterministic encoder
  guarantees and dependency/runtime choice, supported by the benchmark artifact.
- Complete nested Protobuf tag/enum inventory, checked integer widths and the
  generated-code/build strategy, including whether a system `protoc` is needed.
- The aggregate decoder memory budget and proof that the selected message limit
  preserves all valid legacy samples; benchmark budgets and selection criteria must be set before collecting
  selection results.
- Breaking API changes and release version for removing JSON sample support.
- Operator replacement and rollback procedure for each downstream repository.
  Ordinary acceptance remains the only Tuiscotti baseline-writing operation.

These unresolved choices block a format release, not completion of this
specification. Implementation, corpus migration and publication are separate work.
