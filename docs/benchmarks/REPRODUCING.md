# Reproducing the compression evidence

The recorded compression evidence is macOS arm64 only. A relocatable copy of
its source, paired captures, raw Protobuf inputs, prior harness, run records,
and replay helpers is kept in the user's workspace at:

```text
.audit/benchmark-evidence/tuiscotti-repro-bundle-20261105.tar.zst
SHA-256: 27d6c4fbfc63cf99a4ceb6ab6334c4fdb517554a06193ff722fea7ce2c2c85b9
```

This archive and the related `.audit/benchmark-evidence/final-source/` records
are local untracked evidence, not repository files. A fresh clone does not
contain them. These results cannot be described as self-contained in the
repository. The archived captures and raw inputs allow replay without
rebuilding the reference applications; generating new captures still requires
the pinned downstream reference applications/worktrees and producer programs.

## Five-pass interleaved state-codec evidence

The slim v3 archive is available at [`benchmark-evidence/tuiscotti-compression-evidence-slim-v3-20261109.tar.zst`](../../benchmark-evidence/tuiscotti-compression-evidence-slim-v3-20261109.tar.zst) (70,891,144 bytes; SHA-256 `f1c1f947cf60228cba5592f92ac67aec626cfebad823829089f48962805d05b6`). Verify and extract it from the repository root:

```sh
shasum -a 256 benchmark-evidence/tuiscotti-compression-evidence-slim-v3-20261109.tar.zst
evidence_dir="$(mktemp -d "${TMPDIR:-/tmp}/tuiscotti-codecs.XXXXXX")"
zstd -dc benchmark-evidence/tuiscotti-compression-evidence-slim-v3-20261109.tar.zst | \
  tar -xf - -C "$evidence_dir"
```

The archive stores evidence as content-addressed objects. `BUNDLE-MANIFEST.json` lists the original roots and maps the aggregate to `aggregate/real-aggregate-01/aggregate.json` with its expected SHA-256. After extraction, follow `README.md` and `ARCHIVE-COMMANDS.md`; `archive-tools/restore_bundle.py` restores all listed files into two separate chosen subtrees and verifies object hashes. The original root prefixes are `/private/tmp/tuiscotti-interleaved-aggregate-design-20261109` and `/Users/donbeave/Projects/tailrocks/tuiscotti-benchmark-closure-20261108/paired-oracle-20261108/fresh-capture-prototype/coverage-run`; the manifest also contains the standalone source `/private/tmp/build_tuiscotti_archive.py`. These are provenance strings, not durable paths. The record's restore example maps the prefixes to fresh output roots and maps the standalone file explicitly. The full restore may require substantial disk space. The archive README also documents byte-verified reconstruction of the 4,500 omitted worker logs. This slim archive omits 280,800 derived compressed payloads, while preserving their hashes, and 273 unpaired `p03` JSON plus 273 snapshot-transport files; the `p03` artifacts are not reconstructible. Some archived source and command records may contain original absolute paths. The archive supports inspection of preserved inputs, measurements, and aggregate records; the omitted payload bytes are not available for direct inspection. The archive has a logical size of 70,891,144 bytes. An independent disposable bare Git 2.54.0 (Apple Git-157) database containing only that blob measured a 70,898,173-byte pack and 70,899,329 bytes for pack plus index and reverse index. This local measurement is not a GitHub server estimate or a sample-storage measure; its complete summary and commands are linked from the [state-codec evidence record](interleaved-state-codecs.md). The level-22 decision remains provisional as described in [the state-codec evidence record](interleaved-state-codecs.md).

The Zstandard matrix encode timing uses one-shot `ZSTD_compress2`; the shipping writer uses pledged 64 KiB streaming. Although all 1,872 state writer frames matched byte-for-byte, those encode times do not measure shipping-writer latency. Codec-call timing is not end-to-end application timing.

## Verify and extract

Run from the repository root in the same workspace where the archive is
preserved:

```sh
shasum -a 256 .audit/benchmark-evidence/tuiscotti-repro-bundle-20261105.tar.zst
mkdir -p /private/tmp/tuiscotti-repro-20261105
tar --zstd -xf .audit/benchmark-evidence/tuiscotti-repro-bundle-20261105.tar.zst \
  -C /private/tmp/tuiscotti-repro-20261105
cd /private/tmp/tuiscotti-repro-20261105/tuiscotti-repro-bundle-20261105
shasum -a 256 -c REPRO-BUNDLE-SHA256SUMS
(cd source && shasum -a 256 -c BUNDLE-CONTENTS.sha256)
```

The archive SHA-256 above identifies the complete relocatable bundle. Its
internal manifests verify all extracted files and the preserved original
benchmark bundle. Extraction and replay put generated artifacts under `/private/tmp`; mise trust state and tool-managed caches may also be written outside it.

## Replay the recorded codec smoke

Requirements: macOS, mise, MBX 1.21.1, Rust/Cargo and locked dependencies
from the included candidate copy, Python 3, and Cargo Nextest. Starting at the
extracted bundle root:

```sh
BUNDLE_ROOT="$(pwd -P)"
mise trust "$BUNDLE_ROOT/source/candidate-copy/mise.toml"
export TUISCOTTI_BENCH_INPUT="$BUNDLE_ROOT/source/captures/new/samples"
export TUISCOTTI_BENCH_OUTPUT="$BUNDLE_ROOT/replay/replay-smoke"
export TUISCOTTI_BENCH_LEVELS=22
export CARGO_TARGET_DIR="$BUNDLE_ROOT/replay/target"
(
  cd "$BUNDLE_ROOT/source/candidate-copy" && \
    mbx nextest run --locked --offline -p tuiscotti --lib \
      -E 'test(=export_local_captures_at_explicit_zstd_levels)' --no-capture
)
mise trust --untrust "$BUNDLE_ROOT/source/candidate-copy/mise.toml"
```

This test-only probe processes 20 archived captures, checks raw Protobuf
against archived bytes, encodes and decodes level-22 containers, and validates
state digests. It writes no baselines and does not run approval. The bundle's
recorded smoke completed successfully after fresh extraction.

## Replay the level-22 Git history comparison

This regenerates only the level-22 synthetic history outputs in the bundle's
replay area. It requires mise, MBX 1.21.1, Rust/Cargo, locked dependencies,
and Python 3. From the extracted bundle root:

```sh
BUNDLE_ROOT="$(pwd -P)"
mise trust "$BUNDLE_ROOT/source/candidate-copy/mise.toml"
export TUISCOTTI_BENCH_ROOT="$BUNDLE_ROOT/source"
export TUISCOTTI_BENCH_OUTPUT_ROOT="$BUNDLE_ROOT/replay"
export TUISCOTTI_BENCH_LEVELS=22
export CARGO_TARGET_DIR="$BUNDLE_ROOT/replay/target"
python3 "$BUNDLE_ROOT/replay-tools/export-history-variants.py"
python3 "$BUNDLE_ROOT/replay-tools/git-history.py" --level 22 \
  --proto-root "$BUNDLE_ROOT/replay/history/encoded" \
  --out "$BUNDLE_ROOT/replay/history/results-relocated/level-22"
mise trust --untrust "$BUNDLE_ROOT/source/candidate-copy/mise.toml"
```

The helpers create disposable Git repositories under the selected replay
output directory. The recorded relocation replay reproduced the archived
JSON and Protobuf/Zstandard history heads, commit counts, checkout and pack
sizes, and pack hashes. The full noisy codec matrix is preserved as raw
records but was not rerun by this relocation check; host contention makes its
timing unsuitable for ranking codecs.

## Final-source follow-up

The separate local `.audit/benchmark-evidence/final-source/` record documents
five release-mode public writer runs against the same 20 archived samples on
the final review source. Each run passed all 20 round trips; the 100 resulting
files matched the archived level-22 outputs byte for byte. The process-wall
median was 3.16 s (range 3.12–4.63 s), with peak RSS 341,000,192 bytes. These
are feasibility measurements under a contended macOS host, not speed claims.
The record also includes five CLI checks on one already-approved pixel sample,
one render, and a report over an empty artifact directory. It explicitly does
not claim a 20-sample CLI check, accept measurement, or realistic non-empty
report measurement.

That follow-up's test-only harness archive is
`.audit/benchmark-evidence/final-source/benchmark-harness.tar.zst` (SHA-256
`a5a44dfe07a40b2c36f46cc5d1f4b79ccf4dc6d5e6ab9a903f7e15ba86942df8`). Its
run script and raw inputs/results are in that same local evidence directory.
The harness is not part of the product and must not be copied into shipping
source. Its recorded invocation and toolchain are in that directory's
`README.md` and `pinned-toolchain-summary.txt`.

## Limits

The local `.audit/benchmark-evidence/tuiscotti-final-git-evidence-20261106.tar.zst`
archive (SHA-256 `e35f7749d9583feb1a85fdc96e2e982ea516885ed5390a53a7cd00ed63d163ed`)
contains the final-source Git-history replay, its inputs, source, results, and
checksums. Extract it and follow `evidence/README.md` to reproduce the two
disposable histories.

The archives preserve captured outputs and paired evidence, but capture
reproduction requires external pinned reference applications and worktrees.
No native Linux or cross-platform determinism result is available. Benchmark
replay does not establish behavior on Linux or independently recapture the
applications.

## CLI check and acceptance follow-up

The archive
[`tuiscotti-main-cli-check-accept-20261108.tar.gz`](../../benchmark-evidence/tuiscotti-main-cli-check-accept-20261108.tar.gz)
(SHA-256
`49800930f1b851f367e88268ffe02804399bb19daeaeb44d263bdd98d420763a`)
preserves 20 exact level-22 sample containers, five full `check` passes, five
full valid `accept` passes, and raw JSON/time output for every invocation. The
release CLI was built from merged Tuiscotti commit
`6d01628170de8bed3f00f52713f3a510b782e41e` with
`MBX_GC_AUTO=false cargo build --release --locked --offline -p tuiscotti-cli`;
its SHA-256 is
`4c39bc547a0444e5b41d3ac9f5ddb2aaddcb7b9dca30913d5771208b2a15712c`.
That measured commit was the then-main head. The repository main head has since
advanced; this record does not claim a build from the current head. The archive
README describes the workload and scope.

This follow-up closes the 20-capture CLI check and valid-accept feasibility
measurements for native macOS arm64. It does not establish the full downstream
corpus, real approval review, report or capture timing, Git migration history,
Linux behavior, or cross-platform determinism. The harness records process
launches and JSON verification as part of each full-pass wall time.

## Actual TCB Git history evidence

[The mainline Git history record](real-tcb-mainline-history.md) gives exact
TCB commits, selected current functional-root tree sizes, repack settings,
full-history pack sizes, one-commit thin-pack sizes, and scope limits. The
metadata archive
[`tuiscotti-mainline-git-history-20261108.tar.gz`](../../benchmark-evidence/tuiscotti-mainline-git-history-20261108.tar.gz)
contains command and tree/object records; it omits sample and pack binaries.
The actual mainline has only one post-migration PB baseline update commit, so
these data do not satisfy the repeated equivalent update-history comparison.

## Paired historical field comparisons

The [paired sample field comparison](paired-field-comparison.md) records
1,868 protected historical-root JSON/PB pairs, a separate 161-pair format
migration, and 20 archived state/pixel pairs. The package
[`tuiscotti-paired-field-oracle-20261109.tar.gz`](../../benchmark-evidence/tuiscotti-paired-field-oracle-20261109.tar.gz)
(SHA-256
`e0619d68da8a6ddeef57bd5dd3c27d30b766ca108cb859e75878febcafcaa51d`)
contains both workers' source and lockfiles, per-pair comparison rows and
manifests, the 20 archived JSON/PB inputs, six old-reader mutation controls,
and the replay commands. It excludes compiled executables and full TCB Git
repositories; the TCB manifests bind to public commit trees and blob IDs.
The 51 unequal historical state digests are differences between captured
samples that need separate investigation; they are not by themselves evidence
of a codec regression. The original comparator argv was not retained, so the package
labels its commands as replay instructions rather than original invocations.

## Full-corpus render-read and selected capture sample

The
[`render-read and capture supplement`](render-read-capture-supplement.md)
records a serial render-read pass over all 2,117 committed TCB PB paths and a
separate three-test capture-timing sample. Its compact archive,
[`tuiscotti-arm64-render-capture-20261108.tar.gz`](../../benchmark-evidence/tuiscotti-arm64-render-capture-20261108.tar.gz)
(SHA-256
`e9296579e7f523a6cab17786cca02c3c0c67691f28b065625571944c066efcd9`),
contains per-sample render results, input hashes and Git-blob verification, runner logs, capture timing
records, raw timestamped RSS samples, the CLI binary SHA and a reference to its companion source/build
record, capture source identities, and baseline manifests. The
capture environment file lists explicit overrides only; inherited launcher
environment values were not retained. It excludes rendered PNGs and binaries, and omits 40 checkpoint JSON/text payload files (19,820,379 uncompressed bytes).

The render pass attempted 2,117 files: 2,055 succeeded, eight deliberately
truncated fixtures returned `INVALID_SAMPLE`, and 54 valid samples returned
fail-closed `MISSING_GLYPH` errors. The 54 are broken down by code point in
the supplement; they were not omitted. The sample run preserves normal
functional frozen checks but modifies the disposable test-support source to
time `capture_snapshot`. Neither workload establishes full paired
JSON/Protobuf results, complete capture cost, or completion of the
multi-revision Git-history requirement.

## Same-snapshot pixel control

[The same-snapshot pixel control](same-snapshot-pixel-control.md) records one
fresh macOS arm64 Showcase capture compared through old/new Tuiscotti writers
and readers, with four mutation controls. Its archive is
[`tuiscotti-pixel-same-snapshot-evidence-20261109.tar.gz`](../../benchmark-evidence/tuiscotti-pixel-same-snapshot-evidence-20261109.tar.gz)
(SHA-256
`40e5f96c0a997aa41ba65da76b99f803937d0f65ab8acb3b470d7de63af34192`). Verify
the archive with `shasum -a 256`, extract it, then run
`python3 publication/verify-evidence.py` from the extracted root. The package
omits six compiled executables and the Tuiscotti source tarballs; the ledger
contains binary build records, and the exact public Git source commit/tree IDs
must be fetched and checked for rebuild. This is a single-scenario correctness
result, not a corpus-wide, dependency-change, timing, or cross-platform result.

## Ten-scenario interleaved pixel codec matrix

The [interleaved pixel codec results](interleaved-pixel-codecs.md) record 15
codec settings over five passes on ten decoded pixel samples. The
[matrix archive](evidence/interleaved-pixel-codecs-20261109.tar.zst) preserves
all five raw passes, the aggregate, the failed one-shot writer-frame audit,
and replay instructions. It is 30,550,663 bytes with SHA-256
`31808a14ca18831064ada5cc5fcd3ca0ecc4f4a40b9d65c5964916409afd6830`.

The separate [same-snapshot pixel evidence archive](evidence/same-snapshot-pixel-10scenario-20261109.tar.zst)
preserves the ten captures, paired decoded projections, and exact reference
and helper executables. It is 37,801,078 bytes with SHA-256
`f5706b93f30f92bb62cf59c37f6d127b01c607d44ca921ac59d97260ac80cf11`. The two
archive files total 68,351,741 bytes of checkout distribution overhead. This
is their combined file size, not a Git pack measurement.

## Same-capture selected history

[The same-capture selected history experiment](same-capture-selected-history.md)
compares reconstructed JSON and Protobuf Git histories over three selected
logical paths using five fresh captures. One repack produced 16,870-byte JSON
and 15,134-byte Protobuf packs (22 objects each). This is a single selected-path
storage result, not a timing comparison, and it does not measure pixel or
palette updates. The captures are reconstructed from fresh values; they are
not the original captured values or full original commits.

The archive
[`tuiscotti-selected-history-evidence-20261109.tar.gz`](../../benchmark-evidence/tuiscotti-selected-history-evidence-20261109.tar.gz)
has SHA-256
`a5b0e432f55d39fc2d74a7be6826826bcfa8d7ae23f2752516b10e825f1a6305`. The
benchmark page gives extraction and checksum commands. Executables and full
source trees are omitted, and the effective environment and complete immutable
build dependency closure were not recorded, so the archive does not establish a
byte-identical rebuild.
