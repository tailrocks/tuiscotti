# Proposed version 1 wire schema

This is unreviewed. Do not implement against it until the allocation budget, codec runtime, error mapping, and independent review are resolved.

All singular fields are required with explicit presence unless marked optional. Strings, messages, and bytes use wire type 2. Unsigned integers, booleans, and enums use wire type 0. Floats use wire type 5 and doubles wire type 1. Repeated messages preserve domain order.

- Sample: 1 schema string, 2 test ID string, 3 gate enum, 4 snapshot, 5 optional render contract, 6 optional image, 7 observation.
- Snapshot: 1 schema u32, 2 columns u32, 3 rows u32, 4 scope enum, 5 viewport origin u64, 6 repeated cell, 7 repeated RGB, 8 default foreground RGB, 9 default background RGB, 10 default cursor RGB, 11 cursor, 12 optional title, 13 metadata.
- RGB: 1 red, 2 green, 3 blue, each u32 checked to u8.
- Color: oneof 1 empty default-color message, 2 indexed u32 checked to u8, 3 RGB. Exactly one assignment is required.
- Cell: 1 text, 2 width u32 checked to u16, 3 foreground color, 4 background color, 5 bold, 6 dim, 7 italic, 8 inverse, 9 invisible, 10 strike, 11 blink, 12 underline enum, 13 underline color, 14 optional link URI, 15 optional link ID.
- Cursor: 1 column u32, 2 row u32, 3 in-capture, 4 visible, 5 shape enum, 6 color RGB, 7 optional blink.
- Metadata: 1 captured-at u64, 2 revision u64, 3 backend, 4 session, 5 source enum.
- Render contract: 1 profile, 2 contract version u32, 3 renderer version, 4 raster policy, 5 repeated font, 6 cell width float, 7 cell height float, 8 font size float, 9 baseline float, 10 scale double, 11 title bar bool, 12 palette digest, 13 cursor phase enum, 14 optional blink, 15 platform key.
- Font: 1 digest, 2 face index u32, 3 bold, 4 italic, 5 fallback position u64 checked to usize.
- Image: 1 format enum, 2 width u32, 3 height u32, 4 PNG digest, 5 RGBA digest, 6 PNG bytes.
- Observation: 1 optional note.

Enums reserve zero as unspecified and reject it and unknown values. Gate: state 1, state-and-pixels 2. Scope: viewport 1, active screen 2, full history 3. Underline: none 1, single 2, double 3, curly 4, dotted 5, dashed 6. Cursor shape: block 1, underline 2, bar 3. Source: live 1, offline 2. Cursor phase: shown 1, hidden 2. Image format: PNG 1. Verify these against upstream before review.

Writer emits required values including zero, present optionals including empty, tag order ascending, shortest varints, exact float bits, and collection order. Reader validates the 32-byte specified header, one Zstandard frame, checksums, no dictionary, window bound, exact compressed and decompressed lengths, and no trailing data. Scan the decompressed bytes with an explicit bounded stack before owned allocations. Check descriptor-driven fields, cardinality, oneof, wire type, lengths, UTF-8, numeric widths, counts, mandatory presence, and resource limits. Use public upstream constructors. Unknown versions and codecs, malformed data, resources, image failures, and I/O retain distinct classifications.

Open: complete aggregate allocation budget including PNG metadata and scratch, codec state, message and domain allocations, base64, the digest projection, and runtime overhead; exact Protobuf and Zstandard dependencies and generation strategy; validated error mapping; golden bytes and complete pre-change oracle; independent Sol medium review. The current PNG constructor has unlimited initial metadata parsing and the PNG crate allocation limit does not cover every internal buffer. The proposed borrowed direct mapper avoids a generated owned intermediate but requires a single schema source and verified scanner alignment. Cross-platform determinism remains unverified.

## Concrete schema and scanner proposal

The complete unreviewed proto3 inventory is in `.audit/tuiscotti-v1.proposal.proto`. Singular scalars have explicit presence. Required fields are every singular except sample contract/image, cell link URI/ID, cursor blink, snapshot title, contract blink, and observation note. Color requires exactly one alternative. Enum zero is unspecified and rejected. Keep every assigned number and name reserved after retirement. Readers reject unknown fields, duplicate singulars and repeated union assignments, invalid wire types, malformed values and missing required fields. Writers emit ascending tags and shortest varints; readers accept legal field order and valid nonminimal varints, preserving repeated encounter order.

Use the reviewed proto and checked-in descriptor as the only field registry. A pinned offline regeneration tool emits scanner metadata and a typed borrowed traversal interface requiring a handler for every field and union alternative. It rejects unclassified presence and constraint policy; mapper and scanner use the same generated descriptor. A regeneration check compares generated artifacts. Normal builds consume checked-in generated artifacts without `protoc` or network. Exact generator, pinned descriptor-generation tooling, and offline regeneration proof remain open.

Proposed interfaces are `decode_sample(bytes) -> Result<Sample, SampleError>`, a strict `scan_sample(protobuf, budget) -> Result<ValidatedSample, DecodeError>` which borrows bytes and cannot be constructed externally, and `map_sample(validated, budget) -> Result<Sample, SampleError>` which uses validated borrowed ranges and checked exact reservations. Field paths use a bounded stack of static names and repeated indices plus absolute decompressed offset. Header/frame offsets use file offsets. Missing field errors point to the enclosing end; malformed values and duplicates point to their offending byte.

Snapshot reconstruction uses upstream public `SnapshotCell::new`, `SnapshotCursor::new`, `SnapshotParts`, and `TerminalSnapshot::try_new`; verify upstream `SNAPSHOT_SCHEMA_VERSION` before construction. The snapshot schema has no accessor, so encode the pinned upstream constant. Local Sample, RenderContract, Font and Image private fields require crate-private construction, never re-render during decode. PNG exact bytes are retained and the public base64 view is reconstructed. Validate all narrowing, finite resource bounds, palette cardinality, grid and image constraints, UTF-8, and upstream invariants. Preserve accepted float bit patterns and do not introduce unrequested limits on fonts, metadata or notes. Aggregate memory and PNG acceptance evidence remain unresolved.

## Generator proposal after review

Use exact `protoc` 36.2 to regenerate a checked-in descriptor from the proto. The proposal was compiled locally with `mise exec protoc@36.2 -- protoc --proto_path=.audit --descriptor_set_out=/private/tmp/tuiscotti-v1-proposal.desc .audit/tuiscotti-v1.proposal.proto`; its descriptor SHA-256 is `1ec2a3f875e918676e68a566c60beb236007d7809f9204777ee2a9ef7a5ab4a7`. This only establishes syntax for this provisional path. The final stable path, archive checksums and offline regeneration must be established and reviewed.

Use a non-shipping `tools/schema-gen/` Cargo workspace with its own lockfile, outside the root workspace. Pin `prost = 0.14.4`, `prost-types = 0.14.4`, `toml = 1.2.0` and serde for the trusted descriptor and strict policy parser only. Normal builds use checked-in generated Rust and do not run the generator or protoc. After warming both lockfiles and installing the pinned compiler, offline regeneration must rebuild the descriptor, compare it, regenerate Rust from descriptor plus exhaustive policy, and compare artifacts. Run the tooling through the mise mbx Cargo wrapper. Verify the isolated manifest and offline compiler on supported regeneration hosts before adoption.

The checked-in strict TOML policy has an explicit version and records every fully qualified message, field tag/name/kind/cardinality/oneof, presence classification, domain handler and constraint identifiers, plus every enum number/name and acceptance status. Real oneofs require exactly one alternative. Distinguish synthetic proto3 optional oneofs. `deny_unknown_fields`, duplicate-key rejection, exact descriptor/policy key equality, and a closed constraint catalog prohibit missing, extra or unclassified fields. Attach phase-labeled cross-field constraints to their owning messages. Preserve an append-only retired-name/number inventory and compare it with proto reserved declarations. See the field constraint inventory in the audit trail and review it before encoding policy.

Generate scanner dispatch and per-message decode and encode orchestration from this same descriptor and policy. Generate typed mapper and writer traits with one required method per field and oneof alternative, no defaults or blanket fallback. Implement both traits manually. A regenerated added field then makes both implementations fail compilation until handled. Enum mappings use exhaustive matches. Generation and workspace compilation both gate changes. Tests and review must still prove the methods map the correct values.

Strict wire rules: varints use at most 10 bytes, with the tenth byte no greater than 1; accept legal nonminimal forms while rejecting truncation and overflow. Reject field zero, field numbers above Protobuf maximum, invalid wire types and groups. Reject values above u32 before narrowing, bool values other than 0/1, RGB/index above u8, cell width above u16, and u64-to-usize overflow. Enforce required presence, known nonzero enum values, duplicate singular and union rejection and all semantic constraints before owned allocation where applicable. Writer uses canonical shortest varints and ascending field order.

Policy constraints (base type checks are explicit on every matching field):

- Sample: schema UTF8 and SAMPLE_SCHEMA_EXPECTED; test_id UTF8 and TEST_ID_GRAMMAR_240_64; gate ENUM_NONZERO_KNOWN; message GATE_EVIDENCE_PAIR.
- Snapshot: schema UINT32 and SNAPSHOT_SCHEMA_EQ_1; cols/rows UINT32 and GRID_DIM_1_32768; scope ENUM_NONZERO_KNOWN; viewport_origin UINT64; title UTF8 and TITLE_UTF8_65536; message GRID_PRODUCT_CELLS_EXACT_MAX_1000000, PALETTE_COUNT_256 and UPSTREAM_SNAPSHOT_INVARIANTS.
- Rgb channels: UINT32 and RGB_U8. Color indexed: UINT32 and INDEX_U8; message exactly_one. DefaultColor has no fields.
- Cell: text UTF8 and CELL_TEXT_UTF8_4096; width UINT32 and CELL_WIDTH_U16; each attribute bool BOOL_0_1; underline ENUM_NONZERO_KNOWN; link_uri/link_id UTF8 and LINK_UTF8_4096.
- Cursor: col/row UINT32; in_capture/visible/blink BOOL_0_1; shape ENUM_NONZERO_KNOWN.
- Metadata: captured_at_ms/revision UINT64; backend/session UTF8; source ENUM_NONZERO_KNOWN.
- RenderContract: profile/renderer_version/raster_policy/palette_digest/platform_key UTF8; contract_version UINT32 and CONTRACT_VERSION_EXPECTED; cell_width/cell_height/font_size/baseline FLOAT32_BITS; scale FLOAT64_BITS; title_bar/blink BOOL_0_1; cursor_phase ENUM_NONZERO_KNOWN.
- Font: digest UTF8; face_index UINT32; bold/italic BOOL_0_1; fallback_position UINT64 and USIZE_CHECKED.
- Image: format ENUM_NONZERO_KNOWN; width/height UINT32 and IMAGE_DIM_1_32768_RGBA_MAX_268435456; digests UTF8; message IMAGE_DIGEST_AND_GRID.
- Observation note: UTF8. Message-valued and raw bytes fields have explicit empty field constraints and their own applicable message or aggregate constraints. AGGREGATE_BUDGET applies to all variable-size allocation and codec/PNG scratch. The generator must reject omissions and invalid phase assignments.

## Independent sub-contract verdict

Sol medium accepted the schema sub-contract as an implementable design. Apply `UPSTREAM_SNAPSHOT_INVARIANTS` through the pinned upstream constructors and retain existing sample/image validation. This verdict does not establish an implemented policy or generator, stable descriptor path, compiler provenance, offline regeneration, adversarial tests, goldens, round trips, or the aggregate memory budget.

Storage decision: decoded `SampleImage` continues to own the canonical base64 string, not an additional raw PNG buffer. The mapper borrows exact PNG bytes from the decompressed message for validation and digest checks, then creates the base64 value once with checked exact reservation. The public `png_base64()` view remains. The codec must not rerender or transform the PNG. This specifies the lifetime assumed by the proposed allocation ledger; acceptance of all formerly valid base64 spellings needs separate verification because the binary format stores bytes, not text.

Base64 source audit: the currently pinned base64 0.23.1 STANDARD engine requires canonical padding and rejects nonzero trailing bits and nonalphabet bytes. Thus for every parsed SampleImage string accepted by existing validation, `STANDARD.encode(STANDARD.decode(s)?) == s`. JSON token escape spelling is not part of the returned string. This is source evidence for reconstructing the same `png_base64()` value; test the binary implementation as well.
