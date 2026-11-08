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
