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
