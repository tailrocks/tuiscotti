# Changelog

All notable changes to the `tuiscotti` and `tuiscotti-cli` crates are
documented here. Versions follow Semantic Versioning.

## 0.4.0 — Unreleased (not yet published to crates.io or docs.rs)

> Status: prepared on the release branch, **not published yet**.
> Nothing below is installable from a registry until the 0.4.0
> publish lands. Use the [source-install setup](#source-install-setup)
> to try it from git in the meantime.

First public release. Two crates, one offline binary:

- `tuiscotti`: pure-view capture (`view`), versioned samples
  (`sample`), frozen checks (`check`), explicit digest-arbitrated
  approval (`approve`), offline rendering (`render`), static reports
  (`report`), one pinned render profile (`render_profile`), one
  typed error (`error`). Never spawns a process, never opens a
  terminal device, never touches the network.
- `tuiscotti-cli`: offline `tuiscotti` binary with exactly four
  commands — `check`, `accept`, `render`, `report`. Approval refuses
  before any write whenever the `CI` variable is set, in any form.

### Registry blocker (resolved: `tuiscotti-tui-test 0.2.1` adopted)

Both crates build against snapshot/render/pixel APIs that upstream
`microsoft/tui-test` (`tui-test-rs 0.1.1`) does not carry. The
maintained fork is now published as the renamed registry crate
`tuiscotti-tui-test 0.2.1` (lib name still `tui_test`), and the
workspace `[patch.crates-io]` git pin is removed: both crates depend
on `tuiscotti-tui-test 0.2.1` from crates.io like any other registry
dependency, so downstream consumers resolve the same release with no
extra pin. Remaining follow-up (R02) is switching the dependency name
back to upstream `tui-test-rs` once its registry release carries the
required APIs; `use tui_test::...` imports are unaffected either way.

### Source-install setup

Depend on the git source (path dependencies work only inside
this workspace):

```toml
[dependencies]
tuiscotti = { git = "https://github.com/tailrocks/tuiscotti", rev = "<published-0.4.0-sha>" }
```

Build the CLI binary from a workspace checkout instead of
`cargo install` from a registry:

```sh
cargo fetch --locked # warm the registry for offline builds
cargo build --locked --offline -p tuiscotti-cli
cargo install --path crates/tuiscotti-cli --locked --offline --root <dir>
```
