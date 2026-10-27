# `json_out`

Section 16.3 JSON result: one object plus a newline on stdout.

The emission rules live here so every command shares them: stdout
carries exactly one JSON line (or one human summary line without
`--json`), stderr carries diagnostics, and argument-parsing errors
never reach this module because clap exits before dispatch.

## `CLI_SCHEMA`

Schema identifier for every CLI result object.

## `EncodedPath`

A native path encoded without loss, per Section 16.3.

Correctly encoded Unicode paths use `utf8`. Other Unix paths carry
the native bytes as standard padded Base64 (`unix-bytes-base64`).
Other Windows paths carry native UTF-16 code units as little-endian
bytes in standard padded Base64 (`windows-utf16le-base64`). Decoding
always recovers the exact native path.

Encode a native path without loss and without unwrapping UTF-8.

## `CliResult`

One Section 16.3 result object, fields in schema order.

`baseline_sha256` is a 64-character lowercase digest, `"missing"`,
or null when the command has no baseline result. `reason` carries
the mismatch wire value for `different`, the render profile for a
rendered state-only sample, a diagnostic otherwise, and null for a
plain pass.

Build a result; statuses are `pass`, `missing`, `different`, `error`.

The one-line human summary for runs without `--json`.

## `emit`

Emit a result and return the process exit code.

stdout carries exactly one JSON line with `--json`, else the human
summary; a closed pipe preserves the operation result (the reader
went away, but the outcome still happened), any other stdout
failure exits 3. Callers print their own stderr diagnostic before
this on non-zero outcomes.

## Tests (removed)

Utf-8 paths encode losslessly under the `utf8` spelling.

Non-UTF8 Unix paths encode the native bytes as padded Base64 and
decode back to the exact native path.

The JSON object carries every schema field in schema order.
