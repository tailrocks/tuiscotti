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

### Registry blocker (why `[patch.crates-io]` is still pinned)

Both crates build against snapshot/render/pixel APIs that exist only
on the maintained `tui-test` fork pin (`rev 3887a75…`; upstream
`microsoft/tui-test` at the same `0.1.1` version number lacks them,
and crates.io carries only `tui-test-rs 0.1.1`). The workspace
`[patch.crates-io]` entry is workspace-local and is ignored for
downstream consumers, so a registry publish of `tuiscotti 0.4.0`
today would resolve `tui-test-rs ^0.1.1` to the upstream release and
fail to build downstream. Do **not** publish until upstream merges
the fork APIs and ships a new `tui-test-rs` registry release; then
drop the `[patch]` section and re-verify with the checks below.

### Source-install setup

Until the blocker clears, downstream crates must resolve
`tui-test-rs` to the same fork commit. Add to the consuming
workspace root (`Cargo.toml`):

```toml
[patch.crates-io]
tui-test-rs = { git = "https://github.com/donbeave/tui-test", rev = "3887a75dee4463ce0122c37eb278ab7ebb71aeaf" }
```

then depend on the git source (path dependencies work only inside
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
