# Embedded PNG decoder sources

This module contains source files from the crates.io archives `png 0.18.1` and
`fdeflate 0.3.7`, both licensed `MIT OR Apache-2.0`. The corresponding upstream
license texts are included in `../../../licenses/PNG-LICENSE-MIT`,
`../../../licenses/PNG-LICENSE-APACHE`, `../../../licenses/FDEFLATE-LICENSE-MIT`,
and `../../../licenses/FDEFLATE-LICENSE-APACHE`.

The source was copied from the local Cargo registry archives, with SHA-256 digests:

- `png-0.18.1.crate`: `60769b8b31b2a9f263dae2776c37b1b28ae246943cf719eb6946a1db05128a61`
- `fdeflate-0.3.7.crate`: `1e6853b52649d4ac5c0bd02320cddc5ba956bdb407c4b75a2c6b75bf51500f8c`

Changes from upstream include source-module path qualification for embedding; making
all upstream public items crate-private; disabling optional `unstable` and benchmark-only
code paths; adapting one test-only `rng.gen` call to edition-2024 raw-identifier syntax;
and replacing the boxed row-transform closures and fdeflate primary-table boxes with
closed inline enums/arrays. The PNG streaming decoder and fdeflate Huffman table growth
also use checked, fallible exact reservations and carry allocation failures as typed
decoder errors. PNG encoder and metadata-writing implementations are compiled only in
unit-test builds; the shipped embedded module retains decoder behavior only. The
ordinary checksum validation path remains enabled. These edits
are local patches over the exact archived sources, not pristine file copies.

The published PNG crate archive omits its external test fixtures. They are included
under `../../../tests/` from the matching upstream commit identified by the published
crate's `.cargo_vcs_info.json`: `image-rs/image-png` commit
`2a3f980245e3ae38b82ade96533e7b450e8477bb` (2026-02-14, release 0.18.1 merge).
Fixture checksums and license provenance are in `../../../tests/PNG-FIXTURES.md`.
The embedded code remains private to `tuiscotti`; public errors are wrapped by the
parent crate so private source types do not appear in its public API.
