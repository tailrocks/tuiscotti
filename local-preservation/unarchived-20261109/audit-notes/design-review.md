# Independent design and oracle review

Reviewer: gpt-6.1-sol, medium. Verdict: REJECT. The proposal is not an implementation contract yet.

1. Allocation accounting lacks a numerical ceiling, lifetimes, collection capacity rules, and a schema descriptor shared with the scanner.
2. Three PNG entry points use unlimited initial decoder limits. A replacement must bound ancillary metadata, decoder scratch and dimensions consistently without changing accepted behavior.
3. The Zstandard runtime and enforcement operations for one frame, checksum, dictionary, window, exact lengths, and output bound are not selected.
4. The oracle records contract and pixel cases as TSV rather than reusable binary-side assertions. Test-ID mismatch, mismatch ordering, complete diagnostic projections, and approval change summaries remain uncovered.
5. The private state-digest projection is not specified or covered by rich old-projection bytes and hashes, including color, option, float, palette, title, cursor and contract cases.
6. Error variants and check/accept/render/report CLI mappings remain unspecified.
7. The schema inventory appears complete, but no reviewed proto, generation strategy, shared descriptor, constructor mapping, complete-field round trips, or cross-platform golden output exists.

Approval currently rereads and hashes candidate and baseline under lock and installs original candidate bytes. Preserve this flow and prove actual installed bytes equal the candidate, including valid noncanonical binary inputs.

Evidence: `.audit/schema-proposal.md`, `crates/tuiscotti/src/sample.rs`, `check.rs`, `approve.rs`, `render.rs`, and `crates/tuiscotti-cli/tests/compressed_oracle.rs`. Findings remain open until fixed and independently rereviewed.

## Narrow follow-up

Sol medium found no established conflict with the spec. The spec requires typed rejection of resource-limit claims, not recovery from every ambient out-of-memory condition. Infallible bounded allocations alone do not require a fork. A no-fork path may use direct PNG decoding with bounded preflight and skip unused ICC extraction, but it must prove acceptance and RGBA equivalence for valid and malformed metadata, chunk order, APNG and interlaced inputs, and bound uncounted buffers. An `image::with_limits` value alone is insufficient because ICC can consume the shared budget before row decoding.

A second narrow Sol medium review interprets the aggregate budget as covering allocation requests, vector growth, simultaneously live buffers, and library-owned objects. Allocator bookkeeping, rounding, fragmentation, and ambient OOM are outside that deterministic bound. Peak RSS remains separately measured. The proposed PNG counter is not a total budget; an allocation ledger and acceptance-equivalence tests remain mandatory.
