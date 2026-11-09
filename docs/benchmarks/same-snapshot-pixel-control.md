# Same-snapshot pixel control

A fresh live macOS arm64 capture was used to compare the old and new Tuiscotti public `Sample` writers and readers on one `showcase-home-120x40` scenario. The captured Showcase executable was built from TCB revision `60b5fb2762d91a3549bd4ac63b1d77d534ee53d2` (tree `ab75046874d68a3d954dc2b50f440243fc81c14e`; Showcase binary SHA-256 `fb75600f77c7d5e25f12eac030009ccf07b353a42bf8c43eea117efde9bd08d4`). The old Tuiscotti reader/writer used commit `d8d54155947f6303a41dfd6079b53c0abc16f29a` (tree `1e421b2e95c368efc14293fe156bef423539481b`), and the new reader/writer used commit `92ef2d9ee082b35f68e066c5129fedbf07675a80` (tree `d8b16a10c7cfb783c55d10f68321984db14e8df8`).

Both readers produced equal complete typed projections, including snapshot metadata and render contract. The PNGs were 1200×800 and byte-identical (SHA-256 `d407b8aadffd1972369dd741ce18db5c6cc81f5f2e9b9b13c3ca52912046bb38`); the RGBA buffers were 3,840,000 bytes and byte-identical (SHA-256 `9e9c869d2da92da3f10ab0c9db8f92ab7c0f30306d3c1ef84e9eed5da9ff560c`). The independently reviewed captured TerminalSnapshot JSON SHA-256 was `ffd3135b76c0d8606dc91e637521d7cdf1b9b5f837c6979be5e9ea1fee10216a`.

Four new-reader mutation controls were checked. Metadata-only and palette changes preserved both image digests; text and style mutations changed both PNG and RGBA digests. The palette control changed four projected fields (palette digest and three palette components) while leaving image bytes unchanged. These controls and the pair establish only this single scenario and snapshot.

## Evidence and replay

The archive [`tuiscotti-pixel-same-snapshot-evidence-20261109.tar.gz`](../../benchmark-evidence/tuiscotti-pixel-same-snapshot-evidence-20261109.tar.gz) has SHA-256 `40e5f96c0a997aa41ba65da76b99f803937d0f65ab8acb3b470d7de63af34192`. Verify and extract it from the repository root:

```sh
shasum -a 256 benchmark-evidence/tuiscotti-pixel-same-snapshot-evidence-20261109.tar.gz
evidence_dir=$(mktemp -d "${TMPDIR:-/tmp}/tuiscotti-pixel.XXXXXX")
tar -xzf benchmark-evidence/tuiscotti-pixel-same-snapshot-evidence-20261109.tar.gz \
  -C "$evidence_dir"
cd "$evidence_dir/tuiscotti-pixel-control-evidence-20261109"
python3 publication/verify-evidence.py
```

The verifier checks the included evidence hashes, the full outer run-result/provenance/log/input/success-marker chain, source identities, and recomputed PNG/RGBA digests. Six compiled executables are omitted; their exact SHA-256 values and build records are in `publication/reproduction-ledger.json`. The old and new Tuiscotti source trees are also omitted: fetch the immutable commits from the recorded public repository and check their tree IDs using `publication/public-git-resolution.json`. The archive includes lockfiles and the selected TCB Showcase build inputs. Rebuilding requires the pinned Rust/MBX tools and locked dependencies; follow `publication/reproduction-ledger.json` and `publication/README.md`. A rebuilt binary with a different hash is a different result and must be reported as such.

This is a correctness control, not a timing benchmark. It is not a full corpus comparison, a new dependency evaluation, evidence for other platforms, or a baseline approval. No baseline was accepted or written. See also [paired historical field comparisons](paired-field-comparison.md) and the [evidence replay index](REPRODUCING.md).
