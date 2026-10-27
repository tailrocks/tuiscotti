# Module

tuiscotti: offline visual-regression workflow for Ratatui views.

A pure-view test renders an application view in the test process through
[`view`] (no shell, child process, PTY, daemon, or terminal session) and
checks the resulting [`sample`] against frozen baselines with [`check`].
New or changed samples pass explicit digest-arbitrated review in
[`approve`]; [`report`] renders static HTML evidence. [`error`] unifies
the fallible operations of these five modules.

Live terminal tests use `tui_test::Session` directly from application
test code. tuiscotti ships no session, locator engine, input API,
operation enum, renderer, or approval adapter, and it does not re-export
the upstream API: small conversion functions at the visual boundary
([`view::capture_view`], [`view::snapshot_from_buffer`]) are the only
bridge between a Ratatui view and the upstream snapshot model.

# `render_profile`

Render profile selecting the pinned upstream raster policy and fonts.

The registry file keeps its packet name `profile.rs`; the module is
`render_profile` so the name records that the profile describes the
upstream pinned renderer, not a downstream renderer (D07 removed the
local glyph rasterizer and general image renderer).
