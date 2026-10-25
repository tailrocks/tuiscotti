# Limitations

Honest gaps at the D07 head. Labels: **tested** (ran green),
**compiles-only**, **not run**, **unsupported** (explicitly out of
scope, fails closed where applicable).

## Platform matrix

| Platform | Status | Evidence |
|---|---|---|
| macOS (aarch64) | tested | Full guard nextest green at the D07 head (recorded in the D07 report). |
| Linux (x86_64) | tested in CI | CI lanes run `ubuntu-24.04`; the D07 head regenerates per-crate lanes from the two remaining members. |
| Windows (ConPTY) | not run | No Windows lane exists and no Windows test process has ever executed. Windows is never green until a CI lane runs it. |

Unix-only live behavior: `session_migration` (direct upstream
`tui_test` sessions over PTY) is `#![cfg(unix)]`. Pure-view capture,
samples, checks, approvals, and all four CLI commands are platform
portable. Unix file locking (`fs2`) guards approval; on other
platforms the OS equivalent applies.

## Supported fonts

One pinned profile, `tuiscotti-pinned-v1`: 16 px font, 10×20 cells,
baseline 15 px, scale 1, zero padding, no title bar or decorations.
Licensed downstream bytes under `crates/tuiscotti/fonts` (never fetched):

- Primary text: JetBrains Mono Nerd Font Mono (regular, bold,
  italic, bold-italic) — one column per glyph, as the grid requires.
- Symbols: DejaVu Sans Mono Nerd Font Mono (PUA icons, powerline,
  miscellaneous symbols).
- CJK: Noto Sans CJK JP subset.

Fallback order per style slot: styled face, regular face, symbol
font, CJK subset. No host fonts, no synthetic styles, no
replacement glyphs: a grapheme no selected face contains is a typed
missing-glyph error (`MISSING_GLYPH`), and unknown profile names are
rejected (`UNKNOWN_PROFILE`).

## Unsupported graphics and semantic features

- Sixel and Kitty graphics inspection: unsupported.
- Color emoji and codepoints outside the vendored faces: a typed
  missing-glyph error, never tofu substitution.
- Pixel-only gates, tolerances, masks, and automatic redaction:
  unsupported. The two gates are `state` (canonical state only) and
  `state-and-pixels` (state plus render contract and decoded RGBA).
- Scrollback reflow on resize: unsupported (upstream emulator
  constraint).
- Semantic-provider and manual-clock frameworks: unsupported
  downstream. Application semantics and clocks live in application
  test helpers, never inferred from terminal pixels; no tuiscotti
  API drives an arbitrary child through a manual clock.
- Video encoding, GIF/APNG export, and generic terminal diagnostics:
  unsupported (upstream recording APIs, if any, are outside this
  workflow).
- Approval under `CI`, bless flags, auto-accept, globs, and frozen
  or read-only stores that accept: unsupported — approval is one
  candidate, one test ID, two explicit digests, and refuses under
  `CI` with no override.

## Scope

- Live sessions, input, locators, waits, capture, rendering, and
  pixel comparison are upstream `tui-test` behavior with upstream
  limits; tuiscotti asserts nothing about them beyond converting
  their snapshots into samples.
- Hidden text is conceal, not redaction — samples retain the
  original symbols. Never capture real secrets.
- The offline commands never execute sample contents; only live
  application tests spawn processes, through upstream APIs.
