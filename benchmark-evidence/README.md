# Compressed-baseline benchmark evidence

These archives preserve reviewable evidence for the benchmark follow-ups
linked from `docs/benchmarks/`.

- `tuiscotti-compression-evidence-slim-v3-20261109.tar.zst` — slim archive of the 1,872-pair compression evidence, with selected inputs, row measurements, treatment records, aggregate output, and replay helpers. It omits 280,800 derived codec payloads (hashes are retained), 4,500 byte-reconstructible worker logs, and 273 unpaired `p03` JSON plus 273 snapshot-transport files. The archive README documents restoration and log reconstruction; some source and command records may contain original absolute paths. Its logical size is 70,891,144 bytes. A separate disposable bare Git 2.54.0 database with exactly this archive as one blob measured a 70,898,173-byte pack and 70,899,329 bytes for pack plus index and reverse index; this is a local storage measurement, not a GitHub estimate or sample-storage cost. The retained [Git cost summary](tuiscotti-v3-git-cost-20261109/GIT-COST-SUMMARY.json) has SHA-256 `72c2ee1f33e7fb89fe63b5f1f695dfbfd396f7bfab2d8f06fee4f0af05097463`; [commands](tuiscotti-v3-git-cost-20261109/COMMANDS.md) have SHA-256 `14573699140351cd33bc02387194cab460317e0f90788a7d1a7a100f1ae6306f`. See [the state-codec evidence record](../docs/benchmarks/interleaved-state-codecs.md). SHA-256: `f1c1f947cf60228cba5592f92ac67aec626cfebad823829089f48962805d05b6`.
- `tuiscotti-main-cli-check-accept-20261108.tar.gz` — 20-sample native macOS
  arm64 CLI check and valid-accept measurements. Source binary identity and
  limitations are recorded in the archive and in the linked docs. SHA-256:
  `49800930f1b851f367e88268ffe02804399bb19daeaeb44d263bdd98d420763a`.
- `tuiscotti-mainline-git-history-20261108.tar.gz` — metadata for two actual
  TCB mainline Git transitions, including commit/tree/blob identity, commands,
  logs, summaries and checksums. It excludes sample binaries and pack outputs.
  SHA-256:
  `517d1ae7053b7cc1e32483f0d727d0cead7186cc97153b7d977254ede6e4f97a`.
- `tuiscotti-arm64-render-capture-20261108.tar.gz` — full TCB PB render-read
  results plus two selected functional capture runs. The later run binds the
  full source diff and functional test executable SHA-256 before/after, and
  retains session identities and raw sampled process RSS. It excludes rendered
  PNGs and binaries, and omits 40 checkpoint JSON/text payloads (19,820,379
  uncompressed bytes). SHA-256:
  `e9296579e7f523a6cab17786cca02c3c0c67691f28b065625571944c066efcd9`.
- `tuiscotti-paired-field-oracle-20261109.tar.gz` — paired historical sample
  field comparisons and mutation controls. SHA-256:
  `e0619d68da8a6ddeef57bd5dd3c27d30b766ca108cb859e75878febcafcaa51d`.
- `tuiscotti-pixel-same-snapshot-evidence-20261109.tar.gz` — one fresh native
  macOS arm64 Showcase snapshot with paired old/new Tuiscotti writers/readers,
  metadata and image digests, four reader mutation controls, and provenance.
  Six compiled executables are omitted and ledgered for rebuild; old/new
  Tuiscotti source commits must be fetched from the recorded public Git
  repository. SHA-256:
  `40e5f96c0a997aa41ba65da76b99f803937d0f65ab8acb3b470d7de63af34192`.
- `tuiscotti-selected-history-evidence-20261109.tar.gz` — one selected-path
  comparison of reconstructed JSON and Protobuf Git histories from five fresh
  captures. It includes capture comparisons, measurement inputs, logs and pack
  outputs; executables and full source trees are omitted, and it does not claim
  a byte-identical rebuild. SHA-256:
  `a5b0e432f55d39fc2d74a7be6826826bcfa8d7ae23f2752516b10e825f1a6305`.
- The [interleaved pixel codec results](../docs/benchmarks/interleaved-pixel-codecs.md)
  cover 15 codec settings on ten native macOS arm64 pixel samples across five
  passes. The [matrix archive](../docs/benchmarks/evidence/interleaved-pixel-codecs-20261109.tar.zst)
  is 30,550,663 bytes (SHA-256
  `31808a14ca18831064ada5cc5fcd3ca0ecc4f4a40b9d65c5964916409afd6830`). Its
  separate [same-snapshot pixel evidence archive](../docs/benchmarks/evidence/same-snapshot-pixel-10scenario-20261109.tar.zst)
  is 37,801,078 bytes (SHA-256
  `f5706b93f30f92bb62cf59c37f6d127b01c607d44ca921ac59d97260ac80cf11`).
  Together they add 68,351,741 bytes to checkout distribution. This is file
  size, not a Git pack measurement.

- The [public-writer replay](../docs/benchmarks/current-main-public-writer-replay.md)
  records five native arm64 runs of all 2,109 valid TCB PB files and the original
  20 state/pixel benchmark files against main `14e5e015`. Its
  [archive](tuiscotti-public-writer-main-replay-20261109.tar.zst) is 6,485,560 bytes,
  SHA-256 `9c00c526a770327120bbfe840a81cf2cf5064ec0007fc7cbd8d818c06ebfa62d`.
  This is distribution file size, not a Git pack measurement.

The archives are retained evidence, not product inputs. Some include research
replay helpers; no standalone research executable sources are added to the
repository tree. See the linked benchmark records for measurement scope and
remaining gaps.
