# Embedded PNG upstream test fixtures

The fixtures in this directory were copied from `tests/` in the exact
`png 0.18.1` source commit recorded by the published crate's
`.cargo_vcs_info.json`:

- Repository: <https://github.com/image-rs/image-png>
- Commit: `2a3f980245e3ae38b82ade96533e7b450e8477bb`
- Commit date: 2026-02-14
- Commit subject: `Merge pull request #676 from Shnatsel/release-0.18.1`
- Upstream license: MIT OR Apache-2.0; see `../licenses/PNG-LICENSE-MIT` and
  `../licenses/PNG-LICENSE-APACHE`.

`PNG-FIXTURE-SHA256SUMS.txt` contains the SHA-256 digest of each copied fixture,
relative to this directory. The three fdeflate chunking-regression `.zz` fixtures
are separately copied from the published `fdeflate 0.3.7` crate to
`../src/sample/embedded_png/tests/`; their source archive digest is recorded in
`../src/sample/embedded_png/PROVENANCE.md`.
