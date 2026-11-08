# Paired historical sample field comparisons

These native macOS arm64 runs add full typed-field comparisons for three
existing JSON/PB cohorts. They are read-only observations: each format was
opened by a separate Tuiscotti reader pinned to its source revision. The
workers hash the original input bytes and Git blob identity, project sample
fields, validate embedded PNG/RGBA bytes where present, and report the
existing state digest separately. The harness did not convert a JSON sample,
write a baseline, or invoke acceptance.

## Results

| Cohort | Paired inputs | State digest equal | Complete projection equal | Notes |
|---|---:|---:|---:|---|
| Protected historical roots: TCB `60b5fb2` JSON to `534e0ff` PB | 1,868 | 1,817 | 0 | All decoded, tree/blob and file hashes matched. The 1,817 digest-equal rows differed only in metadata. Fifty-one rows had nonmetadata differences and unequal state digests. |
| Migration: TCB `f539d9a` JSON to `1d7f63d` PB | 161 | 161 | 0 | All decoded, tree/blob and file hashes matched. Differences were capture metadata only: `captured_at_ms` in 161 rows and `revision` in 158. |
| Frozen archived capture pairs | 20: 10 state, 10 state-and-pixels | 20 | 0 | All decoded. All ten pixel samples passed PNG/RGBA/hash/dimension validation. Differences were capture metadata only. |

The first cohort compares old JSON files from TCB PR2 commit
`60b5fb2762d91a3549bd4ac63b1d77d534ee53d2` with PB files in merged commit
`534e0ff5af75b840d93038dd02ece0839cf4f71b`, at matching relative paths under
`tests/functional/baselines/pre_refactor/`. These are protected historical
roots; some have live test consumers. They are not inactive archive files.
The second cohort is the distinct old-JSON parent `f539d9a61b1fe1e9fab13f4e976c872b09b8fb9a`
to migration-PB commit `1d7f63dbb13ecb2bf3e3295e7a296dc2080a62b1` transition.
The migration cohort is not counted again as part of the 1,868 historical
roots. See [the real TCB history record](real-tcb-mainline-history.md) for
the commit-level Git measurements and their limits.

The 51 state differences in the historical cohort include nonmetadata field
changes such as cell content/color and cursor values. They establish that
those stored captures are not field-equivalent. They do not establish a
serialization defect: the samples come from different capture histories, and
no matching recapture was made to distinguish capture changes from format
loss. In 28 of those rows, the exact difference count is retained but the
displayed JSON Pointer list is capped at 1,000 paths. Every row carries a
`difference_paths_truncated` flag. The 1,817 digest-equal rows have only
metadata differences and none are truncated.

The pixel cohort comes from the frozen benchmark archive, not a new capture
from TCB `534e0ff`. Its source provenance is old producer revision
`60b5fb2762d91a3549bd4ac63b1d77d534ee53d2`, candidate source-tree SHA-256
`279db9c434436aaa2a4b36976a389a92ca7be3ec0dbf06ff9d95388ff8a8e3dd`, original
archive SHA-256
`726afbf093bcc612e055e085cd3a787f53be6a1d6e051e8cfb476235324392b6`, and
archive content-manifest SHA-256
`95056582c499be14c16637692233c033e1f6c40affbd50f77ae4da7f0151f0ef`.
The bundle includes `subset-binding.tsv` and a verifier for the 40 included
sample files plus the independent projection. The comparison program itself
checks the archive manifest's SHA and each paired input's recorded file hash,
but does not cross-check those inputs' membership in the archive manifest.

## Independent worker control

The old JSON worker's snapshot fields excluding metadata and render-contract
projection were compared with the benchmark archive's independent projection
for one pixel-bearing sample. They matched after explicit representation
normalization; the archived RGBA SHA-256 matched, and the worker verified
PNG/RGBA hashes and image dimensions. Six valid
mutations to a disposable JSON copy (cell text, cell style, palette entry,
cursor visibility, contract font size and metadata session) appeared at the
expected old-reader projection paths.

These mutations test only the old JSON reader. No PB input was edited or
encoded, and an independently generated expected projection was checked for
one old-reader sample only. This evidence does not prove observability of
every projected field in the new PB worker. The real PB reads cover the
complete 1,868 and 161 state cohorts, plus 10 state and 10 pixel samples in
the archived cohort.

## Reproduction bundle

The compact package
[`tuiscotti-paired-field-oracle-20261109.tar.gz`](../../benchmark-evidence/tuiscotti-paired-field-oracle-20261109.tar.gz)
(SHA-256 `e0619d68da8a6ddeef57bd5dd3c27d30b766ca108cb859e75878febcafcaa51d`)
contains the typed workers' source and Cargo lockfiles, comparator and
manifest scripts, full pair manifests and result rows, worker logs, the 20
archived JSON/PB inputs, candidate/content manifests, and six old-worker
mutation fixtures. It does not include compiled binaries or full TCB Git
repositories. The historical manifests bind every row to the public source
commit tree and blob ID. The archive records source and binary hashes, tool
versions, the isolated environment allowlist, and replay commands.

The original comparator shell argv for the three completed runs was not
retained as a separate file. The archive labels its fully specified
reproduction commands as reconstructed commands, not as the historical
argv. Recorded release binary hashes are tied to Tuiscotti revisions
`d8d54155947f6303a41dfd6079b53c0abc16f29a` and
`92ef2d9ee082b35f68e066c5129fedbf07675a80`. Worker process exit statuses were
zero. The comparison program exits nonzero when fields differ; the original
top-level comparator exit code was not separately retained.

## What remains open

These pairs do not complete the benchmark in
[`COMPRESSED-BASELINES.md`](../COMPRESSED-BASELINES.md). They compare stored
JSON and PB samples; they do not run a per-stratum matrix across canonical and
compressed JSON, raw and compressed Protobuf, or the other codec controls.
They do not measure repeated full-corpus encode/decode wall and CPU time or
peak RSS, per-stratum encode/decode, or a sequence of equivalent logical
updates across real Git histories. The real TCB history observations are
whole-commit pack measurements; they are not an equivalent JSON-versus-PB
update sequence. Render-read and selected capture observations are recorded
in the [render-read/capture supplement](render-read-capture-supplement.md).

Separate baseline inventory records 73 missing sidecars and 74 receipt gaps;
these comparisons neither re-audit nor resolve them. The available pairs do
not exhaust the 2,117 PB corpus, do not establish current-main recapture
equivalence for the 51 changed historical samples, and do not add Linux or
cross-platform evidence. Level 22 therefore remains provisional.
