# Compressed-baseline benchmark evidence

These two archives preserve reviewable evidence for the benchmark follow-ups
linked from `docs/benchmarks/`.

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
  PNGs and binaries, and omits 40 checkpoint JSON/text payloads (19,820,379 uncompressed bytes). SHA-256: `e9296579e7f523a6cab17786cca02c3c0c67691f28b065625571944c066efcd9`.

The archives are retained evidence, not product inputs. The first contains
research replay helpers; no standalone research executable sources are added
to the repository tree. See the linked benchmark records for measurement
scope and remaining gaps.
