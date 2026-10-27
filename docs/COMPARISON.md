# Comparison with related projects

tuiscotti does the offline half of visual testing; upstream
`tui-test` does the live half. Application tests call upstream
directly — tuiscotti wraps none of it.

| Capability | tuiscotti | upstream `tui-test` |
|---|---|---|
| Pure-view capture (Ratatui view, in-process, no subprocess) | yes (`view::capture_view`, `snapshot_from_buffer`) | no live session needed on this path |
| Versioned samples (`tuiscotti.sample.v1`) | yes (`sample::Sample`, two gates) | snapshot model owned upstream, consumed here |
| Frozen check (read-only baselines, artifact evidence) | yes (`FrozenStore`, `tuiscotti check`) | no |
| Explicit approval (digest-arbitrated, refuses under `CI`) | yes (`ApprovalStore`, `tuiscotti accept`) | no (`-u`-style flows are out of scope here) |
| Offline render and static reports | yes (`tuiscotti render`, `tuiscotti report`) | no |
| PTY spawn, input, locators, waits, live capture, recording | no (upstream-owned) | yes (`tui_test::Session` and related APIs) |
| Pixel comparison and pinned raster profile | pinned profile here (`tuiscotti-pinned-v1`); rasterizer and comparison upstream | yes |

Two gates: `state` compares canonical state only;
`state-and-pixels` also compares the render contract and decoded
RGBA pixels. A gate change is a policy mismatch — never weaken a
gate after a failure. Semantics: [SNAPSHOTS.md](SNAPSHOTS.md).

The pinned profile is `tuiscotti-pinned-v1` with vendored font
bytes: same frame + same profile + same font bytes =
byte-identical PNGs. Limits: [LIMITATIONS.md](LIMITATIONS.md).

Earlier per-operation comparisons against third-party terminal
tools described a removed product surface (sessions, recorders, op
protocol, insta macros) and quoted retired suite counts. That
material is superseded; the engine history that replaced it is
recorded in [MIGRATION.md](MIGRATION.md) and the review record
under [migration/](migration/).
