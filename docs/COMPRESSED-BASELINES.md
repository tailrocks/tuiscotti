# Compressed Protobuf baselines

## Motivation

Tuiscotti stores complete terminal samples as JSON. A terminal cell repeats its
text, colors, and style fields in every frame. In the
[terminal-components-baseline repository](https://github.com/tailrocks/terminal-components-baseline),
1,978 tracked baseline files contain 2,043,838,901 bytes at commit
`60b5fb2762d91a3549bd4ac63b1d77d534ee53d2`. The checkout occupies about
1.9 GiB for those files. Git already compresses the data, so a smaller checkout
does not necessarily mean a smaller Git repository or history.

A few sampled captures compress well. One 1,126,784-byte JSON capture became
10,316 bytes with gzip. This is a single example, not a corpus-wide result or
a measurement of the proposed format.

## Proposal

Store Tuiscotti samples as Protocol Buffers and compress the encoded bytes.
Choose the compression algorithm and settings after measuring representative
samples and realistic baseline updates. Favor high compression, while reporting
the effects on execution time, checkout size, and Git storage. Do not assume
that compressing each file improves Git's own compression or delta storage.

The decoded sample must preserve every field and the existing comparison
semantics, including cells, dimensions, palette, cursor, gate, observation, and
provenance. Preserve the existing separation between checking and acceptance.
A check must never write a baseline. Acceptance must remain an explicit,
atomic operation and must refuse to run in CI.

Provide a migration path for existing JSON baselines. Prove that migration
preserves decoded sample values and comparison outcomes, including mismatches.
Account for file-byte digests, approval receipts, provenance, and baseline
paths when the serialized representation changes. Define versioning and
failure behavior for malformed, unsupported, or oversized input. Read
[SNAPSHOTS.md](SNAPSHOTS.md) before changing the load-bearing baseline layout
or gate-to-suffix rules. Record format decisions in [DECISIONS.md](DECISIONS.md).

The scope is the sample and baseline format, its migration, and the changes
needed for existing Tuiscotti APIs and CLI commands to use it. Visual review
reports, GitHub publishing, and changes to terminal-components-baseline are
outside this task.

## Prompt to continue

Implement compressed Protobuf sample and baseline storage in Tuiscotti using
the motivation and constraints above. First inspect the current sample schema,
serialization, comparison, acceptance, CLI, and repository instructions.
Design the format and migration around the existing public contracts. Measure
representative real captures, including Git storage over realistic changes,
before choosing the compression settings. Add meaningful tests for lossless
round trips, migration, comparison behavior, malformed input, and read-only
checks. Update the applicable API, snapshot, CLI, migration, and README
documentation. Run the repository's required validation and report the
results and any compatibility decisions. Do not build a visual review tool.
