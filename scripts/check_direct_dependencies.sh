#!/bin/sh
if [ $# -ne 4 ]; then
    echo "check_direct_dependencies: input error: want --workspace WS --crate CRATE" >&2
    exit 2
fi
if [ "$1" != "--workspace" ]; then
    echo "check_direct_dependencies: input error: unknown argument: $1" >&2
    exit 2
fi
if [ "$3" != "--crate" ]; then
    echo "check_direct_dependencies: input error: unknown argument: $3" >&2
    exit 2
fi
if [ ! -r "$2" ]; then
    echo "check_direct_dependencies: input error: unreadable file: $2" >&2
    exit 2
fi
if [ ! -r "$4" ]; then
    echo "check_direct_dependencies: input error: unreadable file: $4" >&2
    exit 2
fi
awk -v forbidden="portable-pty alacritty_terminal termpane" '
function trim(s) {
    sub(/^[ \t]+/, "", s)
    sub(/[ \t]+$/, "", s)
    return s
}
function table_get(table_text, field,    rest, val) {
    rest = table_text
    if (!match(rest, field "[ \t]*=")) return ""
    rest = trim(substr(rest, RSTART + RLENGTH))
    if (match(rest, /^"[^"]*"/)) {
        val = substr(rest, 2, RLENGTH - 2)
        return val
    }
    if (match(rest, /^(true|false)/)) return substr(rest, 1, RLENGTH)
    return ""
}
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
function dep_section(name) {
    if (name == "dependencies") return 1
    if (name == "dev-dependencies") return 1
    if (name == "build-dependencies") return 1
    if (name ~ /^target\..*\.(dependencies|dev-dependencies|build-dependencies)$/) return 1
    return 0
}
function record_ws(key, pkg) {
    ws_name[key] = (pkg != "" ? pkg : key)
}
function record_dep(key, kind, name) {
    n_dep++
    dep_alias[n_dep] = key
    dep_kind[n_dep] = kind
    dep_name[n_dep] = name
}
function close_pending(    real) {
    if (pend_key == "") return
    if (pend_ws) {
        record_ws(pend_key, pend_pkg)
    } else if (pend_wstrue) {
        record_dep(pend_key, "workspace", "")
    } else {
        real = (pend_pkg != "" ? pend_pkg : pend_key)
        record_dep(pend_key, "direct", real)
    }
    pend_key = ""
    pend_ws = 0
}
function parse_error(msg) {
    print "check_direct_dependencies: input error: " msg > "/dev/stderr"
    bad = 1
    exit 2
}
BEGIN {
    n_forbidden = split(forbidden, wanted, " ")
    file_index = 0
    section = ""
    pend_key = ""
}
FNR == 1 { file_index++ }
{
    line = trim(strip_comment($0))
    if (line == "") next
    if (line ~ /^\[.*\]$/) {
        close_pending()
        section = substr(line, 2, length(line) - 2)
        if (section ~ /(^|\.)(dependencies|dev-dependencies|build-dependencies)\.[^.]+$/) {
            pend_key = section
            sub(/^(.*\.)?(dependencies|dev-dependencies|build-dependencies)\./, "", pend_key)
            pend_ws = (section ~ /^workspace\.dependencies\./) ? 1 : 0
            pend_pkg = ""
            pend_wstrue = 0
            pend_file = file_index
        }
        next
    }
    if (line ~ /^\[/) {
        if (file_index == 1) parse_error("invalid TOML in workspace manifest")
        else parse_error("invalid TOML in crate manifest")
    }
    if (pend_key != "") {
        if (!match(line, /^[^=]+=/)) {
            if (file_index == 1) parse_error("invalid TOML in workspace manifest")
            else parse_error("invalid TOML in crate manifest")
        }
        rest = trim(substr(line, RSTART + RLENGTH))
        if (rest ~ /^\{/) {
            pkg = table_get(rest, "package")
            if (pkg != "") pend_pkg = pkg
            if (table_get(rest, "workspace") == "true") pend_wstrue = 1
        } else {
            pkg = table_get(line, "package")
            if (pkg != "") pend_pkg = pkg
            if (table_get(line, "workspace") == "true") pend_wstrue = 1
        }
        next
    }
    if (file_index == 1) {
        if (section != "workspace.dependencies") next
        if (!match(line, /^[^=]+=/)) parse_error("invalid TOML in workspace manifest")
        key = trim(substr(line, RSTART, RLENGTH - 1))
        rest = trim(substr(line, RSTART + RLENGTH))
        if (rest ~ /^\{/) {
            record_ws(key, table_get(rest, "package"))
        } else if (rest ~ /^"/) {
            record_ws(key, "")
        } else {
            parse_error("invalid TOML in workspace manifest")
        }
        next
    }
    if (!dep_section(section)) next
    if (match(line, /^[^=.]+[ \t]*\.[ \t]*workspace[ \t]*=[ \t]*true[ \t]*$/)) {
        record_dep(trim(substr(line, 1, index(line, ".") - 1)), "workspace", "")
        next
    }
    if (!match(line, /^[^=]+=/)) parse_error("invalid TOML in crate manifest")
    key = trim(substr(line, RSTART, RLENGTH - 1))
    rest = trim(substr(line, RSTART + RLENGTH))
    if (rest ~ /^\{/) {
        if (table_get(rest, "workspace") == "true") {
            record_dep(key, "workspace", "")
        } else {
            pkg = table_get(rest, "package")
            record_dep(key, "direct", (pkg != "" ? pkg : key))
        }
        next
    }
    if (rest ~ /^"/) {
        record_dep(key, "direct", key)
        next
    }
    parse_error("invalid TOML in crate manifest")
}
END {
    if (bad) exit 2
    close_pending()
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
' "$2" "$4"
