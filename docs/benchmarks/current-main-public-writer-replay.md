# Current-main public-writer replay

This native macOS arm64 follow-up tests the public binary reader and writer at Tuiscotti commit `14e5e015226921ad6d4aa9c57c6976a8404e0509`, including the refreshed dependency graph. It adds parity and resource evidence; the [level-22 decision remains provisional](../DECISIONS.md#18-binary-sample-persistence-zstandard-level-22-provisional).

## Inputs and checks

The frozen TCB corpus at `534e0ff5af75b840d93038dd02ece0839cf4f71b` has 2,117 committed PB files. The timed cohort contains all 2,109 valid files: 5,589,541 bytes, all state-gated, with 2,041 distinct embedded test IDs. Counts are file occurrences, including protected roots and fixtures, rather than unique captures. The eight deliberately invalid coverage fixtures were excluded from timing and separately rejected by the current reader.

A separate cohort contains the original ten state and ten pixel benchmark files. Its bytes match the inputs in the already published [CLI check/accept evidence archive](../../benchmark-evidence/tuiscotti-main-cli-check-accept-20261108.tar.gz), SHA-256 `49800930f1b851f367e88268ffe02804399bb19daeaeb44d263bdd98d420763a`. These are preserved captures, not fresh terminal captures from current main.

An external release-mode harness called `Sample::from_bytes`, `Sample::to_bytes`, and `Sample::from_bytes` again. Every round trip required complete `Sample` equality, equal state digests, byte-identical containers, preserved render contracts, and independently decoded PNG/RGBA hashes where present. Five passes alternated the two cohorts' order. All 10,545 valid-corpus and 100 original state/pixel round trips passed. Source, lockfiles, executable and input-byte guards passed before and after execution; no baseline or approval operation ran.

The helper was built through pinned MBX and Rust 1.99.1. Its native normal/build dependency tree matched the workspace tree, including package versions and enabled features. The archive preserves the clean source commit/tree, manifests, locks, graph comparison, helper source and exact executable. The inventory identity is a frozen path; embedded test IDs were reported, not independently inferred from filenames.

## Observed time and memory

All times below are seconds. API values sum the per-file calls in one pass; columns show medians over five passes.

| Cohort | Process wall median (range) | API encode wall / CPU | Initial API decode wall / CPU | Maximum process RSS, bytes |
| --- | ---: | ---: | ---: | ---: |
| 2,109 valid TCB files | 145.383 (125.974–162.314) | 102.474 / 97.786 | 17.827 / 17.103 | 64,913,408 |
| Original 20 state/pixel files | 2.262 (1.411–2.874) | 1.291 / 1.271 | 0.283 / 0.279 | 87,785,472 |

Whole-process CPU medians were 138.680 and 2.120 seconds respectively. API clocks exclude input reads, hashes and output writes. Whole-process figures also include a second verification decode, equality/hash/PNG checks and audit JSON generation; they are not end-to-end application timings.

The predeclared limits were 3,600 seconds per process, a nominal 0.2-second RSS poll with a 1,000,000 KiB guard, a 1 GiB posthoc `ru_maxrss` ceiling, and 72 hours cumulative. Every process stayed below these observed limits; measured process wall times summed to 736.330 seconds. Polling and posthoc high-water checks are not a hard operating-system memory limit. Inputs were hashed before each pass, so this is not cold-cache evidence. Host load varied; this single-setting run does not support relative codec rankings. Nearest-rank p95 for five observations is the maximum, not a stable tail estimate.

This does not measure fresh capture, renderer work, full `check`/`accept`/`report` flows or Git history. It does not resolve the known glyph-rendering failures or establish another platform's behavior. The [state codec](interleaved-state-codecs.md), [pixel codec](interleaved-pixel-codecs.md), and [selected history](same-capture-selected-history.md) experiments retain their separate scope and limitations.

## Evidence and replay

The [evidence archive](../../benchmark-evidence/tuiscotti-public-writer-main-replay-20261109.tar.zst) is 6,485,560 bytes, SHA-256 `9c00c526a770327120bbfe840a81cf2cf5064ec0007fc7cbd8d818c06ebfa62d`. This is distribution file size, not a Git pack measurement. It includes all 2,117 TCB PB files, the original 20 files, ten measured processes' complete results/time/RSS logs, eight invalid-fixture controls, build/source records, and the exact measured helper executable. Full source/dependency caches, compiler/SDK and tool installations are omitted and identified in its omission ledger; no bit-identical source rebuild is claimed.

From the repository root:

```sh
set -e
ARCHIVE=benchmark-evidence/tuiscotti-public-writer-main-replay-20261109.tar.zst
printf '9c00c526a770327120bbfe840a81cf2cf5064ec0007fc7cbd8d818c06ebfa62d  %s\n' "$ARCHIVE" | shasum -a 256 -c -
SCRATCH="$(mktemp -d /private/tmp/tuiscotti-public-writer-evidence.XXXXXX)"
zstd -dc "$ARCHIVE" -o "$SCRATCH/evidence.tar"
tar -xf "$SCRATCH/evidence.tar" -C "$SCRATCH"
(cd "$SCRATCH/tuiscotti-public-writer-main-replay-20261109" && python3 -B verify.py)
```

The bundle's README gives the guarded native arm64 replay command. Replay uses a fresh output root and the archived executable, verifies the preserved inputs, and requires semantic and byte-check results to match the original first pass. New timing values need not match the archived measurements. The measured summary SHA-256 is `b6464b0b89add324fe4ccf621b71515177ddf49264729d841588a1aa69c19245`; statistics SHA-256 is `d4f317f706458738d01dc30bb2a75ec7b6d52b536d597cb08ac3b4daf839a669`.
