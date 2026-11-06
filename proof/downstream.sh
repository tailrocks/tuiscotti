#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/src"
printf 'fn main() {}\n' > "$tmp/src/main.rs"
manifest() {
  cat > "$tmp/Cargo.toml" <<MANIFEST
[package]
name = "zstd-assert-consumer"
version = "0.0.0"
edition = "2024"
[dependencies]
tuiscotti = { path = "$root/crates/tuiscotti" }
zstd-sys = { version = "=2.1.2", default-features = false, features = [$1] }
MANIFEST
}
run() { mbx build --offline --manifest-path "$tmp/Cargo.toml"; }
manifest '"pkg-config", "legacy", "thin", "zstdmt", "zdict_builder"'
run
if command -v cmake >/dev/null 2>&1; then
  manifest '"cmake"'
  run
else
  echo "SKIP: cmake is unavailable" >&2
fi
manifest '"non-cargo"'
if run >"$tmp/failure" 2>&1; then echo 'non-cargo unexpectedly succeeded' >&2; exit 1; fi
rg 'DEP_ZSTD_INCLUDE|zstd-sys' "$tmp/failure"
manifest ''
if ZSTD_SYS_USE_PKG_CONFIG= run >"$tmp/failure" 2>&1; then echo 'empty pkg-config environment unexpectedly succeeded' >&2; exit 1; fi
rg 'ZSTD_SYS_USE_PKG_CONFIG|pkg-config' "$tmp/failure"
