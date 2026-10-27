# `ops_check`

`check`: frozen comparison of a saved sample, fully offline.

Reads the sample, compares it against the baseline for its test ID
through [`FrozenStore`](tuiscotti::check::FrozenStore), and reports
the outcome. Nothing here starts an application or writes a
baseline: a missing baseline is a `missing` failure at exit 1, and
failure evidence goes to the artifact root only.

The CLI error code for a frozen-check failure, reusing the library
mapping: sample shapes keep their D04 codes, bad store paths are
`INVALID_STORE_PATH`, pixel-comparison refusals are `INVALID_IMAGE`,
and corrupt evidence, image-encoding, I/O, and report failures are
`IO`. Every shape exits 3.

Emit a check error: the input path is known, the digest only when the
sample bytes were read.

Emit a passing comparison.

Emit a failing comparison: `missing` without a baseline, `different`
with the mismatch reason otherwise.

Run `check` and return the process exit code.
