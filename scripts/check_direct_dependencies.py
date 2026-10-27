#!/usr/bin/env python3
"""Deny direct PTY/engine dependencies in the tuiscotti library manifest.

Text search cannot see through Cargo aliases: a crate manifest holding
only ``pty.workspace = true`` names no forbidden package, while the
workspace manifest maps that alias to ``portable-pty``. This checker
parses both manifests and resolves every alias before deciding.

Usage:
    check_direct_dependencies.py --workspace Cargo.toml \
        --crate crates/tuiscotti/Cargo.toml

Exit status: 0 allowed, 1 forbidden direct dependency, 2 input error.
"""

import argparse
import sys

try:
    import tomllib
except ImportError:  # Python without tomllib cannot parse manifests.
    tomllib = None

FORBIDDEN = ("portable-pty", "alacritty_terminal", "termpane")
DEP_SECTIONS = ("dependencies", "dev-dependencies", "build-dependencies")


def fail_input(message):
    print("check_direct_dependencies: input error: {0}".format(message), file=sys.stderr)
    return 2


def load_manifest(path):
    if tomllib is None:
        return None, "tomllib is unavailable"
    try:
        with open(path, "rb") as handle:
            return tomllib.load(handle), ""
    except FileNotFoundError:
        return None, "unreadable file: {0}".format(path)
    except OSError as exc:
        return None, "unreadable file: {0} ({1})".format(path, exc)
    except tomllib.TOMLDecodeError as exc:
        return None, "invalid TOML in {0} ({1})".format(path, exc)


def dep_entries(manifest):
    """Yield (section, key, spec) for every dependency declaration."""
    for section in DEP_SECTIONS:
        table = manifest.get(section)
        if isinstance(table, dict):
            for key, spec in table.items():
                yield section, key, spec
    target = manifest.get("target")
    if isinstance(target, dict):
        for _cfg, cfg_table in target.items():
            if not isinstance(cfg_table, dict):
                continue
            for section in DEP_SECTIONS:
                table = cfg_table.get(section)
                if isinstance(table, dict):
                    for key, spec in table.items():
                        yield "target." + section, key, spec


def main(argv):
    parser = argparse.ArgumentParser(description="Deny direct PTY/engine deps.")
    parser.add_argument("--workspace", required=True)
    parser.add_argument("--crate", required=True)
    args = parser.parse_args(argv)
    crate, error = load_manifest(args.crate)
    if crate is None:
        return fail_input(error)
    workspace, error = load_manifest(args.workspace)
    if workspace is None:
        return fail_input(error)
    if not isinstance(crate, dict) or not isinstance(workspace, dict):
        return fail_input("top-level manifest value is not a table")
    workspace_deps = workspace.get("workspace", {}).get("dependencies", {})
    if not isinstance(workspace_deps, dict):
        workspace_deps = {}
    violations = []
    for section, key, spec in dep_entries(crate):
        if isinstance(spec, dict) and spec.get("workspace") is True:
            if key not in workspace_deps:
                return fail_input(
                    "unresolved workspace entry for '{0}' in {1}".format(key, section)
                )
            inherited = workspace_deps[key]
            if isinstance(inherited, dict):
                name = inherited.get("package", key)
            else:
                name = key
        elif isinstance(spec, dict):
            name = spec.get("package", key)
        else:
            name = key
        if name in FORBIDDEN:
            violations.append("{0} (as '{1}' in {2})".format(name, key, section))
    if violations:
        for violation in violations:
            print(
                "check_direct_dependencies: forbidden direct dependency: {0}".format(
                    violation
                ),
                file=sys.stderr,
            )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
