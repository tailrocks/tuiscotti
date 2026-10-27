# Module

Named render profile with an immutable built-in font registry (D02, GOAL 13.2).

One logical profile exists: `PROFILE_NAME` (`tuiscotti-pinned-v1`).
`resolve_profile` maps that name to its immutable settings; every other
name is a `ProfileError::UnknownProfile` rejection. No custom font
paths and no network font loading exist on this path.

Font-byte source decision (GOAL 13.2, first branch checked 2026-10-25
against the U02+U03+U04+U05+U11 upstream patch stack): the upstream public
API supplies NO licensed bundled font bytes. The bundled `JetBrains Mono`
faces are private to the module (`REGULAR_FONTS`, `STYLED_FONTS` in
upstream `render/font.rs`) and the bundled Nerd symbol font is `pub(crate)`
(`FONT_DATA` in upstream `render/nerd_font.rs`). Nothing public returns
those bytes, so every slot below uses licensed downstream assets from
`crates/tuiscotti/fonts` (kept under version control, never fetched):

- Slots 1-2, primary text: `JetBrains Mono` Nerd Font Mono regular, bold,
  italic, and bold-italic
  (`crates/tuiscotti/fonts/JetBrainsMonoNerdFontMono-*`). The Nerd Mono
  variant keeps every glyph cell one column wide, which the terminal grid
  requires. License: `crates/tuiscotti/fonts/LICENSE-JetBrainsMono.txt`
  (OFL 1.1).
- Slot 3, symbol font: `DejaVu Sans Mono` Nerd Font Mono
  (`crates/tuiscotti/fonts/DejaVuSansMNerdFontMono-Regular.ttf`). This is
  the downstream stand-in for the upstream bundled Nerd symbol font
  (`SymbolsNerdFontMono`): it covers Private Use Area icons, powerline
  symbols, and miscellaneous symbols. The Noto symbol subsets were
  rejected for this slot because neither covers any Private Use Area
  codepoint, which would leave the recorded PUA selection rule vacuous.
  License: `crates/tuiscotti/fonts/LICENSE-DejaVuSansMono.txt`.
- Slot 4, CJK: `crates/tuiscotti/fonts/NotoSansCJKjp-subset.otf`
  (678988 bytes, SHA-256 `777bee41...d897`), the recorded subset.
  License: `crates/tuiscotti/fonts/LICENSE-Noto.txt`.

Fallback order for ordinary text (recorded in every render contract as
per-face `fallback_position` entries with their style slots):

1. The selected `JetBrains Mono` styled face.
2. The `JetBrains Mono` regular face.
3. The symbol font.
4. The pinned CJK subset.

The upstream pinned rasterizer tries faces in set order within the
requested style slot, so each style slot carries its own chain: the
styled face, then the regular face in that slot, then the symbol font in
that slot, then the CJK subset in that slot. Private Use Area icons
resolve to the symbol font first among the faces that can serve them:
the symbol font precedes the CJK subset, and every PUA icon outside
`JetBrains Mono` coverage (verified class: `U+F6C5`, present in the symbol
font but absent from `JetBrains Mono` Nerd Font Mono 3.5.1) renders from
the symbol font. No host font, replacement glyph, or unselected
synthetic style is ever substituted: a grapheme no selected face
contains (mandatory case: `U+1F980`) is a typed missing-glyph error.

Metrics: 16-pixel font, 10-by-20 cell, baseline 15 pixels from the cell
top, scale 1, zero canvas and content padding, no title bar, controls,
shadow, border, or radius. Resolved terminal colors come from the
snapshot; a cursor the snapshot does not hide draws in the fixed shown
phase.

# `PROFILE_NAME`

The single logical profile name this registry resolves.

# `FONT_SIZE_PX`

Grid font size in pixels.

# `CELL_WIDTH_PX`

Terminal cell width in pixels.

# `CELL_HEIGHT_PX`

Terminal cell height in pixels.

# `BASELINE_PX`

Text baseline measured from the top of each cell, in pixels.

# SCALE

Output scale factor.

# `CELL_WIDTH_U32`

Terminal cell width in whole pixels: the integer twin of
[`CELL_WIDTH_PX`], pinned for checked grid arithmetic without casts.

# `CELL_HEIGHT_U32`

Terminal cell height in whole pixels: the integer twin of
[`CELL_HEIGHT_PX`], pinned for checked grid arithmetic without casts.

# `FACE_INDEX`

Face index inside every asset: each file holds one face.

# `JB_REGULAR_LEN`

Pinned byte length of each asset, verified at profile build.

# `JB_REGULAR_SHA256`

Pinned SHA-256 of each asset, verified at profile build.

# `ProfileError`

Every failure a profile lookup or build can report, with sources retained.

# `UnknownProfile` {

The requested profile name is not in the built-in registry.

# requested: String,

The rejected name.

# `AssetMismatch` {

Embedded font bytes do not match their pinned length or digest.

# face: &'static str,

Which asset failed its pin check.

# detail: String,

What the pin check found.

# `FontBuild` {

The upstream validated constructor rejected committed font bytes.

# face: &'static str,

Which face failed validation.

# source: `PinnedFontError`,

The upstream rejection.

# `ProfileFace`

One face in the built-in registry, in fallback order.

# role

What this face serves (for example `"primary-bold"`).

# `bytes_sha256`

SHA-256 over the face bytes: the registry key with [`ProfileFace::index`].

# `byte_len`

Pinned byte length of the face.

# index

The face index inside its bytes (zero for every single-face asset here).

# bold

Whether this face serves the bold style slot.

# italic

Whether this face serves the italic style slot.

# Profile

One named, immutable set of render settings and font references.

# name

The logical profile name.

# faces

The registry faces in fallback order.

# `raster_policy`

The raster policy every render through this profile records.

# `render_options`

Offline render options for this profile: pinned fonts with the
profile metrics, zero chrome, and no canvas background override (the
grid fills every pixel from resolved snapshot colors).

# `platform_key`

The platform key recorded in render contracts: `<os>-<arch>` from the
Rust target names (for example `linux-x86_64`, `macos-aarch64`).

# `resolve_profile`

Resolve a profile name through the immutable built-in registry.

Only [`PROFILE_NAME`] resolves. Unknown names fail without touching
fonts, the network, or the filesystem. The profile builds once behind
a lock; no panic path exists on repeated lookups.

# Errors

Returns [`ProfileError::UnknownProfile`] for every other name, and the
build failure if committed font assets ever fail their pin checks.

# `PINNED_V1`

The single built-in profile, constructed once and never mutated.

# `FaceSpec`

One registry row: role, bytes, pinned length, pinned digest, style slot.

# `face_specs`

Build the registry in fallback order within each style slot: the styled
`JetBrains Mono` face, the `JetBrains Mono` regular face, the symbol font,
then the CJK subset.

# `sha256_hex`

SHA-256 hex over raw bytes.

# `select_face`

Verify one asset against its pins and select it for its style slot.
