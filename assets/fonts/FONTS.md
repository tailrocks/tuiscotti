# Fonts: licensing and coverage

One profile exists: `tuiscotti-pinned-v1` (`crates/tuiscotti/src/profile.rs`).
It embeds exactly six font files from `crates/tuiscotti/fonts/` via
`include_bytes!`, so the compiled library always carries its own fonts.
There are no custom font
paths, no `--font-file` flag, no host-font lookup, and no network font
loading anywhere on this path (`crates/tuiscotti-cli/src/cli.rs` defines
only `check`, `accept`, `render`, and `report`; `render` selects output
by the `--out` extension, `.png` or `.html` — there is no `--format`
flag).

## The six embedded files

| File | Bytes | SHA-256 (pinned in `profile.rs`) | Role |
|---|---|---|---|
| `JetBrainsMonoNerdFontMono-Regular.ttf` | 2,573,248 | `f2a5ea6cfab397445ffab00c0370927b66d61e560a05db5db271b42006381c1a` (`JB_REGULAR_*`) | Primary text, regular |
| `JetBrainsMonoNerdFontMono-Bold.ttf` | 2,577,016 | `bfcf9a917276ffc058867d87cbc8a5b2f1ab0f4b710e9170dc02763ccb80bd4b` (`JB_BOLD_*`) | Primary text, bold |
| `JetBrainsMonoNerdFontMono-Italic.ttf` | 2,575,368 | `31efd6ead98746f5b0afa1ee6dba60267ad48db36428360bee327bec10621f97` (`JB_ITALIC_*`) | Primary text, italic |
| `JetBrainsMonoNerdFontMono-BoldItalic.ttf` | 2,578,280 | `9dba502e00e35209f6ed2a151c7376c051657b067cdebbc6e52d06cb9002cf31` (`JB_BOLD_ITALIC_*`) | Primary text, bold-italic |
| `DejaVuSansMNerdFontMono-Regular.ttf` | 2,814,292 | `9b55ade625f2d3f2a273ed16d9db2d924ad6236222920f9fd1324941cdc3c712` (`SYMBOL_*`) | Slot 3 symbol font: PUA icons, powerline, misc symbols |
| `NotoSansCJKjp-subset.otf` | 678,988 | `777bee41f0c6076c00ad919384359a6e396b8822cf9056041fca8fcf2759d897` (`CJK_*`) | Slot 4: kana, fullwidth forms, JIS X 0208 level-1 kanji |

The Nerd Mono variant keeps every glyph cell one column wide, which the
terminal grid requires. The profile refuses to build on any length or
digest mismatch, so a corrupt or swapped asset fails loudly instead of
shifting pixels silently.

The two `NotoSansSymbols{,2}-subset.ttf` files next to this document are
reference-only: they are not embedded, not pinned, and not on the
fallback chain. They were rejected for the symbol slot because neither
covers any Private Use Area codepoint, which would have left the PUA
selection rule vacuous.

## Fallback order

Recorded in every render contract as per-face `fallback_position`
entries. Within each style slot the upstream pinned rasterizer tries,
in order:

1. The selected `JetBrains Mono` styled face.
2. The `JetBrains Mono` regular face.
3. The symbol font (`DejaVu Sans Mono` Nerd Font Mono).
4. The pinned CJK subset.

`cell.mods` selects the style slot: bold to Bold, italic to Italic,
both to BoldItalic. Within the slot the rasterizer tries the faces in
listed order and uses the first face containing the grapheme; an icon
uses the symbol font only when the earlier faces cannot supply it.

## Missing glyphs are errors, not tofu

No host font, replacement glyph, or unselected synthetic style is ever
substituted: a grapheme no selected face contains (mandatory case:
`U+1F980` 🦀) is a typed missing-glyph error. Renders surface it as
code `MISSING_GLYPH` (`crates/tuiscotti/src/render.rs`); approvals
surface `ApproveError::MissingGlyph` with the same code and write no
baseline. The `live_workflow` functional test renders a state sample
carrying `U+1F980` and requires exit 3 with `error_code`
`MISSING_GLYPH` and no output file. There is no `.fidelity.json`
sidecar, no `fallback_glyphs` or `approximate` report fields, and no
tofu substitution: unrenderable text fails the command instead.

Uncovered by every face, and therefore rejected this way: Hangul, JIS X
0208 level-2 kanji, and color emoji. The CJK subset is extensible by
re-cutting it and updating the `CJK_LEN` / `CJK_SHA256` pins, but wider
coverage is a deliberate follow-up, not a silent fallback.

## Metrics (pinned in `Profile`)

16-pixel font, 10-by-20 cell, baseline 15 pixels from the cell top,
scale 1, zero canvas and content padding, no title bar, controls,
shadow, border, or radius. All four primary faces measure identically.

## License (why vendoring is allowed)

JetBrains Mono is **SIL Open Font License 1.1**, and the Nerd Fonts
patch re-releases under the same OFL — see
`crates/tuiscotti/fonts/LICENSE-JetBrainsMono.txt`. The Noto CJK subset
is likewise **SIL OFL 1.1** (`LICENSE-Noto.txt`). The DejaVu file
remains under the Bitstream Vera license
(`LICENSE-DejaVuSansMono.txt`). Each license text ships next to its
faces, as redistribution requires.
