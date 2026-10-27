#!/bin/sh
# Deny direct PTY/engine dependencies in the tuiscotti library manifest.
#
# Text search cannot see through Cargo aliases: a crate manifest holding
# only `pty.workspace = true` names no forbidden package, while the
# workspace manifest maps that alias to `portable-pty`. This checker
# reads both manifests and resolves every alias before deciding.
#
# Usage:
#   check_direct_dependencies.sh --workspace Cargo.toml \
#       --crate crates/tuiscotti/Cargo.toml
#
# Exit status: 0 allowed, 1 forbidden direct dependency, 2 input error.
# Only single-line TOML entries are read; the repository keeps its
# manifests and fixtures in that form.

FORBIDDEN="portable-pty alacritty_terminal termpane"

usage_error() {
    echo "check_direct_dependencies: input error: $1" >&2
    exit 2
}

WORKSPACE=""
CRATE=""
while [ $# -gt 0 ]; do
    case "$1" in
        --workspace) WORKSPACE="$2"; shift 2 ;;
        --crate) CRATE="$2"; shift 2 ;;
        *) usage_error "unknown argument: $1" ;;
    esac
done
[ -n "$WORKSPACE" ] || usage_error "missing --workspace"
[ -n "$CRATE" ] || usage_error "missing --crate"
[ -r "$WORKSPACE" ] || usage_error "unreadable file: $WORKSPACE"
[ -r "$CRATE" ] || usage_error "unreadable file: $CRATE"

awk -v workspace_file="$WORKSPACE" -v crate_file="$CRATE" \
    -v forbidden="$FORBIDDEN" '
function strip_comment(line,    out, i, c, in_str, esc) {
    out = ""
    in_str = 0
    esc = 0
    for (i = 1; i <= length(line); i++) {
        c = substr(line, i, 1)
        if (esc) {
            out = out c
            esc = 0
            continue
        }
        if (c == "\\" && in_str) {
            out = out c
            esc = 1
            continue
        }
        if (c == "\"") {
            in_str = !in_str
            out = out c
            continue
        }
        if (c == "#" && !in_str) break
        out = out c
    }
    return out
}
function trim(s) {
    sub(/^[ \t]+/, "", s)
    sub(/[ \t]+$/, "", s)
    return s
}
function dep_section(name) {
    if (name == "dependencies") return 1
    if (name == "dev-dependencies") return 1
    if (name == "build-dependencies") return 1
    if (name ~ /^target\..*\.(dependencies|dev-dependencies|build-dependencies)$/) return 1
    return 0
}
function table_get(table_text, field,    rest, val) {
    rest = table_text
    if (!match(rest, field "[ \t]*=")) return ""
    rest = substr(rest, RSTART + RLENGTH)
    rest = trim(rest)
    if (match(rest, /^"[^"]*"/)) {
        val = substr(rest, 2, RLENGTH - 2)
        return val
    }
    if (match(rest, /^(true|false)/)) return substr(rest, 1, RLENGTH)
    return ""
}
function parse_error(msg) {
    print "check_direct_dependencies: input error: " msg > "/dev/stderr"
    bad = 1
    exit 2
}
BEGIN {
    n_forbidden = split(forbidden, wanted, " ")
    file_index = 0
}
FNR == 1 { file_index++ }
{
    line = trim(strip_comment($0))
    if (line == "") next
    if (line ~ /^\[.*\]$/) {
        section = substr(line, 2, length(line) - 2)
        next
    }
    if (line ~ /^\[/) {
        if (file_index == 1) parse_error("invalid TOML in " workspace_file)
        else parse_error("invalid TOML in " crate_file)
    }
    if (file_index == 1) {
        # Workspace manifest: only [workspace.dependencies] matters.
        if (section != "workspace.dependencies") next
        if (!match(line, /^[^=]+=/)) parse_error("invalid TOML in " workspace_file)
        key = trim(substr(line, RSTART, RLENGTH - 1))
        rest = trim(substr(line, RSTART + RLENGTH))
        if (rest ~ /^\{/) {
            pkg = table_get(rest, "package")
            ws_name[key] = (pkg != "" ? pkg : key)
        } else if (rest ~ /^"/) {
            ws_name[key] = key
        } else {
            parse_error("invalid TOML in " workspace_file)
        }
        next
    }
    # Crate manifest: dependency sections, including target-specific ones.
    if (!dep_section(section)) next
    if (match(line, /^[^=.]+[ \t]*\.[ \t]*workspace[ \t]*=[ \t]*true[ \t]*$/)) {
        key = trim(substr(line, 1, index(line, ".") - 1))
        dep_alias[++n_dep] = key
        dep_kind[n_dep] = "workspace"
        next
    }
    if (!match(line, /^[^=]+=/)) parse_error("invalid TOML in " crate_file)
    key = trim(substr(line, RSTART, RLENGTH - 1))
    rest = trim(substr(line, RSTART + RLENGTH))
    if (rest ~ /^\{/) {
        if (table_get(rest, "workspace") == "true") {
            dep_alias[++n_dep] = key
            dep_kind[n_dep] = "workspace"
        } else {
            pkg = table_get(rest, "package")
            dep_alias[++n_dep] = key
            dep_kind[n_dep] = "direct"
            dep_name[n_dep] = (pkg != "" ? pkg : key)
        }
        next
    }
    if (rest ~ /^"/) {
        dep_alias[++n_dep] = key
        dep_kind[n_dep] = "direct"
        dep_name[n_dep] = key
        next
    }
    parse_error("invalid TOML in " crate_file)
}
END {
    if (bad) exit 2
    for (i = 1; i <= n_dep; i++) {
        if (dep_kind[i] == "workspace") {
            key = dep_alias[i]
            if (!(key in ws_name)) {
                print "check_direct_dependencies: input error: unresolved workspace entry for \x27" key "\x27" > "/dev/stderr"
                exit 2
            }
            real = ws_name[key]
        } else {
            real = dep_name[i]
        }
        for (j = 1; j <= n_forbidden; j++) {
            if (real == wanted[j]) {
                print "check_direct_dependencies: forbidden direct dependency: " real " (as \x27" dep_alias[i] "\x27)" > "/dev/stderr"
                found = 1
            }
        }
    }
    exit (found ? 1 : 0)
}
' "$WORKSPACE" "$CRATE"
