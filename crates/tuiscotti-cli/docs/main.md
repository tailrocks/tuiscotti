# main

`tuiscotti`: offline visual-snapshot commands.

Exactly four commands, all offline:

```text
tuiscotti check --sample PATH --baselines DIR --artifacts DIR
tuiscotti accept --candidate PATH --candidate-sha256 SHA256 --expect-baseline SHA256_OR_MISSING --baselines DIR
tuiscotti render --sample PATH --out PATH
tuiscotti report --artifacts DIR --out PATH
```

`check` applies the frozen comparison without starting an application
or writing a baseline. `accept` runs the explicit digest-arbitrated
approval transaction. `render` extracts or re-renders a `.png`, or
writes a static `.html` page, from a saved sample. `report` builds a
static HTML index from saved candidate evidence.

Exit codes follow this contract: 0 for a pass or a successful
offline operation, 1 for a missing baseline or a differing sample, 2
for invalid arguments (clap usage errors stay at 2 with no JSON
result), and 3 for file, format, profile, rendering, approval, or I/O
failures. Error codes travel in the `tuiscotti.cli.v1` result object
with `--json`, or on stderr otherwise. Exit 2 covers
both argument-error shapes: `render`'s `--out` extension check and
`accept`'s digest checks run as clap parsers, so rejections exit 2
with no JSON result; every sample-content failure, including corrupt
samples, exits 3.

The requested result goes to stdout (one JSON object plus a newline
with `--json`, one summary line otherwise). Diagnostics go to stderr.
Stdout writes tolerate a closed pipe: a vanished reader preserves the
operation result, never a panic.

## `StdoutFlush`

How a buffered stdout flush ended.

The bytes reached stdout; the caller returns its outcome code.

The reader went away; the caller returns its outcome code.

Stdout failed another way; the caller exits 3.

## `write_stdout`

Write a complete report to stdout without panicking on a closed pipe.

Pure-report commands buffer their output and flush once here: a
broken pipe is [`StdoutFlush::BrokenPipe`] (the reader went away;
the outcome code is preserved), any other error is reported on
stderr.
