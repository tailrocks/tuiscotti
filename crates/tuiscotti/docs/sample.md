# `sample` documentation

# Module

Versioned visual samples binding a snapshot to its evidence (D02, GOAL 13.1/13.3).

One sample is one UTF-8 JSON file with the schema [`SAMPLE_SCHEMA`]
(`tuiscotti.sample.v1`) and this field structure, in this order:

```text
schema
test_id
gate
snapshot            # public upstream serialization, row-major cells
render_contract     # null for a state sample without an image
image               # null for a state sample without an image
  format            # "png"
  width
  height
  png_sha256
  rgba_sha256
  png_base64
observation
```

The `snapshot` field uses the public upstream [`TerminalSnapshot`]
serialization unchanged. Types are explicit everywhere: no unstructured
maps. Unknown fields are rejected, as are unsupported schemas.

Comparison rules (comparison itself is D03; this module supplies the
primitives): the compared state is the gate, the snapshot without its
observation metadata, the render contract, and the decoded RGBA pixels.
Test IDs, file encodings, and sample-level observation are addressing
and evidence metadata: they change the candidate ID but never the
compared state. The candidate ID is the SHA-256 of the sample file bytes.

Resource limits (GOAL 8.4) apply before allocation or rendering:
encoded sample file 64 MiB (bounded reads), one sample 1,000,000 cells,
one cell string 4,096 UTF-8 bytes, decoded RGBA 256 MiB with decoded
image dimension validation before allocation, one image dimension 32,768
pixels, test ID 240 ASCII bytes with 64 ASCII bytes per segment.

# `pub const SAMPLE_SCHEMA: &str = "tuiscotti.sample.v1";`

Sample schema identifier. Only this value parses.

# `pub const CONTRACT_VERSION: u32 = 1;`

Schema version of the downstream render contract in samples.

# `pub const MAX_SAMPLE_FILE_BYTES: u64 = 64 * 1024 * 1024;`

Encoded sample file limit: 64 MiB, enforced with bounded reads.

# `pub const MAX_SAMPLE_CELLS: usize = 1_000_000;`

Cells in one sample.

# `pub const MAX_CELL_TEXT_BYTES: usize = 4_096;`

UTF-8 bytes in one cell string.

# `pub const MAX_RGBA_BYTES: u64 = 256 * 1024 * 1024;`

Decoded RGBA byte limit: 256 MiB, validated before allocation.

# `pub const MAX_IMAGE_DIM: u32 = 32_768;`

One image dimension limit: 32,768 pixels.

# `pub const MAX_TEST_ID_BYTES: usize = 240;`

Test ID length limit: 240 ASCII bytes.

# `pub const MAX_TEST_ID_SEGMENT_BYTES: usize = 64;`

One test ID segment limit: 64 ASCII bytes.

# `pub(crate) fn read_bounded_optional(`

Read one evidence file through a shared bounded read: bytes past
[`MAX_SAMPLE_FILE_BYTES`] fail as a resource limit instead of loading,
so no evidence read can exhaust memory before validation runs.
A missing file maps to `Ok(None)` so callers keep the
missing-versus-unreadable distinction; every other I/O failure is `Io`.

# Errors

Returns [`SampleError::TooLarge`] when the file passes the limit and
[`SampleError::Io`] when it cannot be read.

# `pub(crate) fn read_bounded(path: &std::path::Path) -> Result<Vec<u8>, SampleError> {`

Read one evidence file that must exist through the shared bounded read.

# Errors

Returns [`SampleError::Io`] for a missing or unreadable file and
[`SampleError::TooLarge`] when the file passes the limit.

# `pub enum SampleError {`

Every failure a sample operation can report, with sources retained.

# `TooLarge {`

The encoded file passes the 64 MiB limit.

# `what: &'static str,`

What was measured.

# `bytes: u64,`

The rejected byte count.

# `limit: u64,`

The accepted limit.

# `InvalidTestId {`

A test ID breaks its shape rules.

# `detail: String,`

What the test ID broke.

# `GateMismatch {`

A gate and its evidence disagree, or a test ID and gate disagree.

# `detail: String,`

What disagreed.

# `UnsupportedSchema {`

The schema identifier is not the supported sample schema.

# `version: String,`

The rejected schema value.

# `UnknownField {`

The encoded sample carries a field the schema does not define.

# `detail: String,`

The upstream serde report, kept verbatim.

# `InvalidFormat {`

The encoded envelope is not valid JSON for this schema.

# `source: serde_json::Error,`

The decoding failure.

# `InvalidSnapshot {`

The embedded snapshot fails upstream validation.

# `detail: String,`

The upstream validation report, kept verbatim.

# `DimensionTooLarge {`

An image dimension passes its limit.

# `width: u32,`

The rejected width.

# `height: u32,`

The rejected height.

# `limit: u32,`

The accepted per-dimension limit.

# `RgbaTooLarge {`

Decoded RGBA bytes pass the 256 MiB limit.

# `bytes: u64,`

The rejected byte count.

# `limit: u64,`

The accepted limit.

# `DimensionMismatch {`

Claimed image dimensions disagree with decoded PNG dimensions.

# `detail: String,`

What disagreed.

# `MissingDifferenceImage {`

Upstream pixel comparison reports changed pixels without a
difference image.

# `detail: String,`

What the comparison reported.

# `PngInvalid {`

The embedded PNG does not decode as 8-bit RGBA.

# `detail: String,`

What decoding found.

# `PngDecode {`

PNG decoding failed.

# `source: image::ImageError,`

The decoding failure.

# `Base64Invalid {`

Embedded base64 does not decode.

# `source: base64::DecodeError,`

The decoding failure.

# `DigestMismatch {`

A recorded digest disagrees with recomputed bytes.

# `field: &'static str,`

Which digest disagreed (`png_sha256` or `rgba_sha256`).

# `expected: String,`

The recorded value.

# `actual: String,`

The recomputed value.

# `Render {`

The upstream renderer refused the snapshot.

# `source: RenderError,`

The upstream rejection, with its source chain.

# `UnexpectedPolicy {`

The upstream pinned raster policy is not the recorded strict policy.

# `policy: String,`

The policy the render recorded.

# `UnsupportedContract {`

The downstream contract version is not the supported one.

# `version: u32,`

The rejected contract version.

# `Io {`

Filesystem failure while reading or writing a sample file.

# `source: std::io::Error,`

The filesystem failure.

# `pub enum Gate {`

The comparison policy a sample carries.

# `State,`

Compare the full canonical state only.

# `StateAndPixels,`

Compare canonical state plus render contract and decoded RGBA pixels.

# `pub const fn as_str(self) -> &'static str {`

The wire value: `state` or `state-and-pixels`.

# `pub struct SampleSchema;`

The schema identifier as an explicit type: only [`SAMPLE_SCHEMA`] parses.

# `pub struct Observation {`

Sample-level observation: evidence metadata that never joins the compared
state. Different observation gives a different candidate ID with equal
visual state.

# `pub const fn empty() -> Self {`

Empty observation.

# `pub fn noted(note: impl Into<String>) -> Self {`

Observation carrying a reviewer note.

# `pub fn note(&self) -> Option<&str> {`

The reviewer note, when present.

# `pub struct SampleImage {`

One image in a sample: encoded PNG bytes plus digests over the encoding
and over the decoded pixels.

# `pub const fn format(&self) -> ImageFormat {`

The image format; only PNG exists.

# `pub const fn width(&self) -> u32 {`

Decoded image width in pixels.

# `pub const fn height(&self) -> u32 {`

Decoded image height in pixels.

# `pub fn png_sha256(&self) -> &str {`

SHA-256 over the encoded PNG bytes.

# `pub fn rgba_sha256(&self) -> &str {`

SHA-256 over the decoded RGBA pixels (row-major, 8 bits per channel).

# `pub fn png_base64(&self) -> &str {`

The encoded PNG bytes.

# `pub enum ImageFormat {`

Image formats a sample can carry. Only PNG exists: no pixel-only or
masked variants exist in this goal.

# `Png,`

Portable Network Graphics, 8-bit RGBA.

# `pub struct ContractFont {`

One font entry in a downstream render contract.

# `pub fn digest(&self) -> &str {`

SHA-256 over the style slot, face index, and bytes of the face.

# `pub const fn face_index(&self) -> u32 {`

The face index inside its bytes.

# `pub const fn bold(&self) -> bool {`

Whether the face serves the bold style slot.

# `pub const fn italic(&self) -> bool {`

Whether the face serves the italic style slot.

# `pub const fn fallback_position(&self) -> usize {`

The position of the face in the profile fallback order.

# `pub enum CursorPhase {`

Cursor visibility recorded in a render contract.

# `Shown,`

The cursor was drawn: the snapshot does not hide it.

# `Hidden,`

The cursor was hidden or clipped out of the capture.

# `pub struct RenderContract {`

The renderer, font, geometry, and color settings that produced an image.

Structural validation (no font resolution) accepts a historical contract
that parses under this schema; rendering and approval re-rendering
resolve the profile by name instead. Style slots, fallback order, font
digests with face indices, geometry, palette digest, cursor phase, and
the platform key are all recorded here.

# `pub(crate) fn from_rendered(`

Build the recorded render contract from a resolved profile and a
fresh upstream render: the single typed constructor for recorded
render identity.

[`Sample::with_pixels`] records through it, and approval re-rendering
rebuilds the expected identity through it, so creation and
verification cannot disagree on profile, contract version, renderer
version, raster policy, fonts, geometry, scale, title bar, palette
digest, cursor phase, blink, or platform.

The pinned-profile gate lives here deliberately, not beside the
approval field comparison: the comparison checks recorded identity
against re-rendered identity, both derived from the recorded profile,
so a self-consistent non-pinned identity would pass it. Only this
constructor (and the single-profile registry behind
`resolve_profile`) pins creation and verification to
[`PROFILE_NAME`].

# Errors

Returns [`SampleError`] when the profile is not the pinned profile
or the upstream render records a foreign raster policy.

# `pub fn profile(&self) -> &str {`

The logical profile name the image was rendered with.

# `pub const fn contract_version(&self) -> u32 {`

The downstream contract schema version.

# `pub fn renderer_version(&self) -> &str {`

The renderer crate version that produced the image.

# `pub fn raster_policy(&self) -> &str {`

The raster policy version: `pinned-v1` for strict renders.

# `pub fn fonts(&self) -> &[ContractFont] {`

The pinned fonts in fallback order.

# `pub const fn cell_width(&self) -> f32 {`

The cell width in pixels.

# `pub const fn cell_height(&self) -> f32 {`

The cell height in pixels.

# `pub const fn font_size(&self) -> f32 {`

The grid font size in pixels.

# `pub const fn baseline(&self) -> f32 {`

The text baseline from the top of each cell in pixels.

# `pub const fn scale(&self) -> f64 {`

The output scale factor.

# `pub const fn title_bar(&self) -> bool {`

Whether a title bar was drawn.

# `pub fn palette_digest(&self) -> &str {`

SHA-256 over the resolved palette and default colors.

# `pub const fn cursor_phase(&self) -> CursorPhase {`

Whether the cursor was drawn.

# `pub const fn blink(&self) -> Option<bool> {`

The blink intent kept from the snapshot state.

# `pub fn platform_key(&self) -> &str {`

The `<os>-<arch>` platform the image was rendered on.

# `pub struct Sample {`

One versioned visual sample: a snapshot bound to its visual evidence.

Fields stay private; constructors and accessors are the whole surface.
Field order in the file is the declaration order below and never changes.

There is no derived `Deserialize`: the manual implementation below runs
the same complete validation as [`Sample::from_bytes`], so the serde
path can never bypass gate pairing or digest checks.

# `struct SampleWire {`

Private wire mirror with a deferred snapshot: lets classification tell
an envelope failure (unknown field, bad gate) apart from a snapshot
validation failure with one parse. The only way to a [`Sample`].

# `pub fn state(`

Build a state sample: serialize the snapshot without rendering.

# Errors

Returns [`SampleError`] when the test ID breaks its shape rules.

# `pub fn with_pixels(`

Build a state-and-pixels sample: render the immutable snapshot
through the profile, then validate the image before returning.

The snapshot is never re-captured during rendering: the caller-held
snapshot is the single input to both serialization and rendering.

# Errors

Returns [`SampleError`] when the test ID breaks its shape rules,
when the upstream renderer refuses the snapshot, or when the
returned image fails validation.

# `pub const fn schema(&self) -> SampleSchema {`

The schema identifier (always [`SAMPLE_SCHEMA`]).

# `pub fn test_id(&self) -> &str {`

The test ID this sample was constructed for.

# `pub const fn gate(&self) -> Gate {`

The comparison policy this sample carries.

# `pub fn snapshot(&self) -> &TerminalSnapshot {`

The embedded snapshot.

# `pub fn render_contract(&self) -> Option<&RenderContract> {`

The render contract, present only for `state-and-pixels`.

# `pub fn image(&self) -> Option<&SampleImage> {`

The image evidence, present only for `state-and-pixels`.

# `pub const fn observation(&self) -> &Observation {`

The observation metadata, excluded from the compared state.

# `pub fn to_bytes(&self) -> Result<Vec<u8>, SampleError> {`

Serialize to canonical file bytes: declaration field order, stable
upstream snapshot order, row-major cells. Validates before
serializing, then enforces the file limit, so only valid samples
ever reach the wire.

# Errors

Returns [`SampleError`] when the sample fails validation, when
serialization fails, or when the bytes pass the file limit.

# `pub fn from_bytes(bytes: &[u8]) -> Result<Self, SampleError> {`

Parse and validate encoded file bytes without font resolution.

Structural validation only: test ID, gate pairing, contract version,
image dimensions (before allocation), PNG decoding, and both
digests. Historical contracts pass without resolving their fonts.

# Errors

Returns [`SampleError`] for every rejected input; see its variants.

# `fn from_wire(envelope: SampleWire) -> Result<Self, SampleError> {`

Build a validated sample from one parsed wire envelope: the single
construction path shared by [`Sample::from_bytes`] and the manual
[`serde::Deserialize`] implementation, so no parse path can bypass
structural or image validation.

# Errors

Returns [`SampleError`] for every rejected input; see its variants.

# `pub fn from_file(path: &std::path::Path) -> Result<Self, SampleError> {`

Read a sample file with a bounded read: bytes past the file limit
are never loaded.

# Errors

Returns [`SampleError`] when the file is unreadable, too large, or
fails [`Sample::from_bytes`] validation.

# `pub fn save(&self, path: &std::path::Path) -> Result<(), SampleError> {`

Serialize and write one sample file through one atomic file write: a
temporary file in the destination directory, flushed and closed
before one atomic rename, like all sibling evidence. Readers never
see a partial file.

# Errors

Returns [`SampleError`] when serialization fails, when the sample
path has no parent directory or file name, or when the write fails.
A partial write is a failure, never a success value.

# `pub fn candidate_id(bytes: &[u8]) -> String {`

The candidate ID: SHA-256 over the sample file bytes.

Different observation metadata gives a different candidate ID with
equal compared state; the ID is never the visual equality rule.

# `pub fn state_digest(&self) -> Result<String, SampleError> {`

Digest over the compared state only: gate, snapshot without its
observation metadata, render contract, and decoded RGBA pixels.
Test ID, file encoding, and sample observation are excluded.

# Errors

Returns [`SampleError`] when the embedded image fails to decode.

# `pub fn validate_structure(&self) -> Result<(), SampleError> {`

Structural validation without font resolution: gate pairing, test
ID, contract version, grid limits, and cell text limits. Used both
after construction and at comparison time for historical contracts.
The pixels suffix checks against the stored contract identity, so a
sample recorded on another platform parses wherever its own ID and
contract agree; local capture still generates the current-platform
ID through [`Sample::with_pixels`].

# Errors

Returns [`SampleError`] for every structural rejection.

# `fn decoded_pixels(image: &SampleImage) -> Result<(u32, u32, Vec<u8>), SampleError> {`

Decode the embedded image with pre-allocation dimension validation.

# `fn validate_image_bytes(&self) -> Result<(), SampleError> {`

Digest and dimension agreement between the image record and its bytes.

# `fn validate_test_id(test_id: &str, gate: Gate, pixels_platform: &str) -> Result<(), SampleError> {`

Validate a test ID for its gate: the Section 14.1 grammar plus the
pixels suffix pairing against the expected platform identity. Callers
capturing locally pass the current platform; saved-sample parsing passes
the stored contract identity, so historical samples keep parsing off the
platform that recorded them.

This is the single complete test-ID validator: construction
([`Sample::state`], [`Sample::with_pixels`]) and parsing
([`Sample::from_bytes`] through [`Sample::validate_structure`]) all run
through it, and the frozen checker delegates its grammar-only check to
[`validate_test_id_grammar`], so no second shape rule can drift.

# `pub(crate) fn validate_test_id_grammar(test_id: &str) -> Result<(), SampleError> {`

Validate a test ID against the Section 14.1 grammar on every platform:

```text
segment = [a-z0-9_-]{1,64}
test_id = segment ("/" segment)*
```

Total length follows Section 8.4 (240 ASCII bytes). Absolute paths,
empty segments, backslashes, and `.`/`..` segments are rejected without
normalization. Windows device names (`con`, `prn`, `aux`, `nul`,
`com1`-`com9`, `lpt1`-`lpt9`, case-insensitive, with or without an
extension) are rejected as segments on all platforms.

# Errors

Returns [`SampleError::InvalidTestId`] for every rejected shape.

# `fn is_device_name(segment: &str) -> bool {`

True for Windows device names on any platform: `con`, `prn`, `aux`,
`nul`, `com1`-`com9`, `lpt1`-`lpt9`, case-insensitive, with or without a
dotted extension. The caller guarantees ASCII input.

# `fn check_file_size(bytes: &[u8]) -> Result<(), SampleError> {`

Enforce the encoded file limit on a loaded byte string.

# `fn check_grid_limits(cols: u32, rows: u32, cells: usize) -> Result<(), SampleError> {`

Enforce grid and cell-count limits on numbers, before any allocation.

# `fn preflight_snapshot_wire(value: &serde_json::Value) -> Result<(), SampleError> {`

Preflight the nested snapshot wire value with typed schema and limit
checks before [`TerminalSnapshot`] deserialization.

Upstream snapshot failures arrive as one untyped serde string, which
would flatten an unsupported schema or an over-limit grid into
[`SampleError::InvalidSnapshot`]. The wire numbers are checked here
instead — never by matching upstream message text — so those shapes
keep [`SampleError::UnsupportedSchema`] and the resource-limit
variants (and their `UNSUPPORTED_SCHEMA`/`RESOURCE_LIMIT` codes).
Anything this preflight does not recognize stays for
[`TerminalSnapshot`] deserialization, which still fails closed with
[`SampleError::InvalidSnapshot`].

# `fn preflight_snapshot_schema(value: &serde_json::Value) -> Result<(), SampleError> {`

Reject a nested snapshot wire schema other than the supported one.

A missing schema mirrors the upstream wire default (`0`), which the
upstream build rejects; a present non-numeric schema stays for
deserialization instead of inventing a version spelling.

# `fn preflight_snapshot_grid(value: &serde_json::Value) -> Result<(), SampleError> {`

Reject nested snapshot dimensions and cell counts past the limits.

Bounds come from the downstream limits, which match the upstream
snapshot bounds one for one.

# `fn preflight_snapshot_cells(value: &serde_json::Value) -> Result<(), SampleError> {`

Reject nested snapshot cell text and links past the byte limits.

# `fn preflight_snapshot_title(value: &serde_json::Value) -> Result<(), SampleError> {`

Reject a nested snapshot window title past the byte limit.

# `fn check_image_dims(width: u32, height: u32) -> Result<(), SampleError> {`

Enforce image dimension limits on numbers, before any allocation.

# `fn check_grid_image(`

Check the decoded image against the snapshot grid with checked
arithmetic: zero chrome means exactly `cols * cell` by `rows * cell`.

# `fn decode_png_bounded(png: &[u8]) -> Result<(u32, u32, Vec<u8>), SampleError> {`

Decode 8-bit RGBA PNG bytes with dimension validation before the pixel
buffer is allocated. The caller checks header dimensions first through
[`check_image_dims`]; this decoder reads the IHDR again from untrusted
bytes and refuses anything that changed or is not RGBA8.

# `fn sha256_hex(bytes: &[u8]) -> String {`

SHA-256 hex over raw bytes.

# `fn hex_bytes(bytes: &[u8]) -> String {`

Lowercase hex over digest bytes.
