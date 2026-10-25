# Fonts: licensing and coverage

## Layout (B7: profile bytes ship inside the crate)

The six faces the pinned profile embeds — four JetBrains Mono Nerd Font
Mono faces, DejaVu Sans Mono Nerd Font Mono, and the Noto Sans CJK JP
subset — live with their license texts in `crates/tuiscotti/fonts/`, so
`cargo package` ships them inside the `tuiscotti` library package. The
two Noto symbol subsets below are reference-only (not embedded by the
profile) and stay here alongside this file.

## Vendored assets (default family)

JetBrainsMono **Nerd Font Mono** — four faces, each ~2.5 MB:

- `JetBrainsMonoNerdFontMono-Regular.ttf` — geometry + hash authority
- `JetBrainsMonoNerdFontMono-Bold.ttf`
- `JetBrainsMonoNerdFontMono-Italic.ttf`
- `JetBrainsMonoNerdFontMono-BoldItalic.ttf`

- Original typeface: <https://www.jetbrains.com/lp/mono/>
- Patch project: <https://github.com/ryanoasis/nerd-fonts> (Mono variant:
  every glyph cell 1-wide)

`DejaVuSansMNerdFontMono-Regular.ttf` (2.8 MB, in
`crates/tuiscotti/fonts/`) is the profile's wired symbol face (slot 3):
it serves Private Use Area icons, powerline symbols, and miscellaneous
symbols the primary family lacks.

## Vendored fallback faces (per-glyph chain)

Glyphs the primary family lacks are served per-glyph by pinned Noto subsets
(per-face `fallback_position` entries in the render contract; registry
order in `Profile::faces()` in `crates/tuiscotti/src/profile.rs`), tried
in this order after the primary styled + regular faces:

| File | Source | Subset covers | Size |
|---|---|---|---|
| `NotoSansSymbols2-subset.ttf` | Noto Sans Symbols 2 Regular | U+25A0–25FF Geometric Shapes (◐ ● ◆), U+2600–26FF Misc Symbols (★ ☕ ⚠), U+2700–27BF Dingbats (❤ ✔), U+2B00–2BFF Misc Symbols & Arrows (⬤ ⭐) | 87 KB |
| `NotoSansSymbols-subset.ttf` | Noto Sans Symbols (variable → wght 400 instance) | U+2600–26FF misc symbols unique to v1 (⚷ ⚙ ♻ ☰-class) — v1/v2 are disjoint in this block (probed) | 27 KB |
| `NotoSansCJKjp-subset.otf` | Noto Sans CJK JP Regular | U+3000–30FF (CJK punctuation, hiragana, katakana), U+FF01–FF65 (fullwidth forms), JIS X 0208 level-1 kanji (2965, incl. 東京日本語中文) | 679 KB |

Each face's SHA-256 is pinned in code (`JB_*_SHA256`, `SYMBOL_SHA256`,
`CJK_SHA256` in `crates/tuiscotti/src/profile.rs`) and verified when the
pinned profile builds; a mismatch refuses to render. Fallback faces draw
single glyphs in their own weight, centered and clipped inside the cell
box the primary geometry pins — they never move the cell grid.

Committed bytes are hash-pinned in code
(`crates/tuiscotti/src/profile.rs`: `JB_*_LEN` / `JB_*_SHA256`,
`SYMBOL_LEN` / `SYMBOL_SHA256`, `CJK_LEN` / `CJK_SHA256`) and verified
with pure-Rust SHA-256 every time the pinned profile builds — no Python,
no fonttools, no `xtask` (deleted at D07). From a clean checkout:

```sh
cargo nextest run -p tuiscotti --test samples --locked --offline
```

The `d02_profile_*` tests resolve the pinned profile, which refuses to
build on any length or digest mismatch, and
`d02_profile_assets_ship_inside_the_crate` trips if an embedded asset or
its license ever leaves the `tuiscotti` package directory again.

Subset provenance (upstream sources the committed bytes were cut from):

- `google/fonts@8b0a1d0f` `ofl/notosanssymbols/NotoSansSymbols[wght].ttf`
- `google/fonts@7b6724ac` `ofl/notosanssymbols2/NotoSansSymbols2-Regular.ttf`
- `notofonts/noto-cjk@165c01b4` `Sans/OTF/Japanese/NotoSansCJKjp-Regular.otf`

### Rebuilding / extending the subsets

Needs `fonttools` (`pip install --user fonttools`). Instantiate the
Symbols variable font at wght=400, subset with pyftsubset, assert
required codepoints survived, then update the length and SHA-256 pins in
`crates/tuiscotti/src/profile.rs` (file size and `sha256sum` of the new
bytes) and re-run the suite above.

The exact pyftsubset invocation (same flags for all three):

```sh
pyftsubset <upstream> --output-file=<subset> --unicodes-file=<ranges> \
  --no-hinting --name-IDs=0,1,2,4,6,13,14 --name-legacy --name-languages=* \
  --no-layout-closure --drop-tables+=GSUB,GPOS
```

To extend coverage (JIS X 0208 level-2 kanji, Hangul, more symbol blocks):
widen the pyftsubset unicode ranges, rerun, then update the
`JB_*` / `SYMBOL_*` / `CJK_*` pins in
`crates/tuiscotti/src/profile.rs` and the coverage table below (pins are
asserted by `crates/tuiscotti/tests/samples.rs`). Mind repo size: a full
Noto Sans CJK face is ~16 MB — keep subsets to a few MB max.

## License (why vendoring is allowed)

JetBrains Mono is licensed under the **SIL Open Font License 1.1**, and the
Nerd Fonts patch re-releases under the same OFL. OFL permits redistribution
with software provided the license text ships alongside — hence
`crates/tuiscotti/fonts/LICENSE-JetBrainsMono.txt` next to the faces (the
canonical OFL 1.1 text, <https://openfontlicense.org>). Do not sell the
font standalone; modified versions must not use a Reserved Font Name.

The Noto faces (Symbols, Symbols 2, CJK JP) are likewise **SIL OFL 1.1** —
see `crates/tuiscotti/fonts/LICENSE-Noto.txt` (Copyright The Noto Project
Authors). Subsetting an OFL font for embedding is permitted; the subsets
keep the upstream copyright and license name entries.

The DejaVu file remains under the Bitstream Vera license
(`crates/tuiscotti/fonts/LICENSE-DejaVuSansMono.txt`): redistribution
allowed with the license text; do not use the names "Bitstream" or "Vera"
for modified versions.

The SHA-256 of each vendored face is pinned in code (`JB_*_SHA256`,
`SYMBOL_SHA256`, `CJK_SHA256` in `crates/tuiscotti/src/profile.rs`,
asserted by the `d02_profile_*` tests), and
`d02_profile_assets_ship_inside_the_crate` trips if an embedded asset or
its license ever leaves the `tuiscotti` package directory again. Any
font change fails gates loudly instead of shifting pixels silently.

## Cell metrics (pinned in `Profile`)

Raster backend is `swash` (unhinted; the previous `fontdue` backend
was removed with its unmaintained `ttf-parser`). Unhinted swash
rasters match the old geometry exactly, so the pins below are
unchanged.

At `font_px = 16` the profile pins `cell_w = 10`, `cell_h = 20`, and
baseline 15 px from the cell top (`crates/tuiscotti/src/profile.rs`).
All four primary faces measure identically. Glyphs rasterize at
`16 × scale` directly onto the final image (HiDPI, no post upscale).
Fallback faces are rasterized at the same pixel size but do NOT share
these metrics: their glyphs are centered horizontally in the cell span
and clipped to the cell rect.

## Face selection

`cell.mods` selects the primary face: bold → Bold, italic → Italic, both →
BoldItalic. Per-glyph chain: styled face → regular face → fallback faces in
chain order → tofu. The faux double-strike / shear survive only when a
non-regular face fails to parse (recorded as `faces_fell_back` in the
fidelity sidecar) or `--font-file` overrides with a single face. Fallback
faces render in their own regular weight regardless of `cell.mods`.

## Coverage reality

| Class | Status |
|---|---|
| ASCII, Latin, punctuation | Full (primary family) |
| Box drawing, blocks, Braille | Full (primary family) |
| Powerline symbols, Nerd-Font icons (PUA) | Full (primary family) |
| Combining marks | Rendered overlaid at the same origin (documented approximation) |
| Geometric Shapes ◐, Misc Symbols ★ ☕, Dingbats ❤ ✔ | Fallback: Noto Sans Symbols 2 subset (recorded in `fallback_glyphs`) |
| Misc symbols unique to v1: ⚷ (U+26B7), ⚙, ♻ | Fallback: Noto Sans Symbols subset |
| Kana, JIS X 0208 level-1 kanji (東京…), fullwidth forms | Fallback: Noto Sans CJK JP subset, correct 2-cell advance via `unicode-width` |
| Hangul, JIS X 0208 level-2 kanji | NOT covered → deterministic tofu box + fidelity report (subset extensible, see above) |
| Color emoji (e.g. U+1F980 🦀) | NOT covered → tofu + fidelity report |

Missing glyphs are detected when no face in the chain produces a non-empty
bitmap (cmap index alone is not coverage — empty Nerd-Font placeholders
must not block Noto) and drawn as an outline box; every miss is listed with position
and `U+XXXX` codepoints in `<name>.png.fidelity.json` next to each PNG
output (`render --format png` and store `actual/`/`approved/` pairs) —
exact reporting, never silent tofu. Cells served by a fallback face are
listed in the same sidecar under `fallback_glyphs` (the field is omitted
when empty). `approximate: true` marks any render with missing glyphs or
fell-back faces. `--font-file` overrides the embedded family (single face;
its hash is recorded in reports instead; the fallback chain still applies).
