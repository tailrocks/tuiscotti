# input

Bounded sample-file reads shared by the commands taking a sample path.

Reads stop past [`MAX_SAMPLE_FILE_BYTES`]: oversized input reports a
typed failure instead of allocating without bound.

## `ReadFailure`

How a bounded sample read failed.

The file passes the encoded-sample limit.

The rejected byte count (limit plus one).

The file could not be opened or read.

The filesystem failure.

The CLI error code for this failure: both shapes exit 3.

The diagnostic for stderr and the JSON `reason` field.

## `read_sample_bytes`

Read a sample file with a bounded read.
