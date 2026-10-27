# `ops_accept`

`accept`: the explicit D04 approval transaction, fully offline.

Approves one candidate for one test ID with the mandatory digests.
There is no `--all`, no glob, and no CI override. Digest shapes and
the `missing` literal are validated by clap parsers before this runs,
so rejections there exit 2 with no JSON result; every failure below
is operational and exits 3. The failure JSON reports the observed
baseline digest (`missing` when no baseline file was present, null
when the failure carries no observed baseline), never the requested
one.

The process exit code for an approval failure: `2` for malformed
digest arguments, `3` for every operational failure. The library
keeps the typed errors and their Section 16.3 codes; this mapping is
the CLI boundary.

Run `accept` and return the process exit code.
