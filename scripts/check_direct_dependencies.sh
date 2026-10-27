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
    if (!match(rest, field "[ \t]*=")) {
        if (!match(rest, "\"" field "\"[ \t]*=")) {
            if (!match(rest, "\x27" field "\x27[ \t]*=")) return ""
        }
    }
    rest = trim(substr(rest, RSTART + RLENGTH))
    if (match(rest, /^"[^"]*"/)) {
        val = substr(rest, 2, RLENGTH - 2)
        return decode_basic(val)
    }
    if (match(rest, "^\x27[^\x27]*\x27")) {
        val = substr(rest, 2, RLENGTH - 2)
        return val
    }
    if (match(rest, /^(true|false)/)) return substr(rest, 1, RLENGTH)
    return ""
}
function strip_comment(line,    out, i, c, in_str, in_lit, esc) {
    out = ""
    in_str = 0
    in_lit = 0
    esc = 0
    for (i = 1; i <= length(line); i++) {
        c = substr(line, i, 1)
        if (in_lit) {
            if (c == "\x27") in_lit = 0
            out = out c
            continue
        }
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
        if (c == "\"" && !in_lit) {
            in_str = !in_str
            out = out c
            continue
        }
        if (c == "\x27" && !in_str) {
            in_lit = 1
            out = out c
            continue
        }
        if (c == "#" && !in_str) break
        out = out c
    }
    return out
}
function normkey(s,    n, parts, i, p, out, f, l) {
    n = split(s, parts, /\./)
    out = ""
    for (i = 1; i <= n; i++) {
        p = parts[i]
        if (length(p) >= 2) {
            f = substr(p, 1, 1)
            l = substr(p, length(p), 1)
            if (f == "\"" && l == "\"") p = decode_basic(substr(p, 2, length(p) - 2))
            else if (f == "\x27" && l == "\x27") p = substr(p, 2, length(p) - 2)
        }
        out = (out == "" ? p : out "." p)
    }
    return out
}
function is_table_prefix(p) {
    if (p == "target" || p == "workspace") return 1
    if (p ~ /^target\./ && !is_table_path(p)) return 1
    return 0
}
function decode_basic(s,    out, i, c, e, j, hex, code, ok, h) {
    out = ""
    i = 1
    while (i <= length(s)) {
        c = substr(s, i, 1)
        if (c != "\\" || i == length(s)) { out = out c; i++; continue }
        e = substr(s, i + 1, 1)
        if (e == "n") { out = out "\n"; i += 2 }
        else if (e == "t") { out = out "\t"; i += 2 }
        else if (e == "r") { out = out "\r"; i += 2 }
        else if (e == "b") { out = out "\b"; i += 2 }
        else if (e == "f") { out = out "\f"; i += 2 }
        else if (e == "\"") { out = out "\""; i += 2 }
        else if (e == "\\") { out = out "\\"; i += 2 }
        else if (e == "/") { out = out "/"; i += 2 }
        else if ((e == "u" && i + 5 <= length(s)) || (e == "U" && i + 9 <= length(s))) {
            hex = (e == "u" ? substr(s, i + 2, 4) : substr(s, i + 2, 8))
            code = 0; ok = 1
            for (j = 1; j <= length(hex); j++) {
                h = substr(hex, j, 1)
                if (h >= "0" && h <= "9") code = code * 16 + h
                else if (h == "a" || h == "A") code = code * 16 + 10
                else if (h == "b" || h == "B") code = code * 16 + 11
                else if (h == "c" || h == "C") code = code * 16 + 12
                else if (h == "d" || h == "D") code = code * 16 + 13
                else if (h == "e" || h == "E") code = code * 16 + 14
                else if (h == "f" || h == "F") code = code * 16 + 15
                else { ok = 0; break }
            }
            if (ok) { out = out sprintf("%c", code); i += (e == "u" ? 6 : 10) }
            else { out = out c; i++ }
        }
        else { out = out c; i++ }
    }
    return out
}
function str_value(text,    q, v, i, c, out) {
    q = substr(text, 1, 1)
    v = substr(text, 2)
    if (q == "\"") {
        out = ""
        i = 1
        while (i <= length(v)) {
            c = substr(v, i, 1)
            if (c == "\\" && i < length(v)) { out = out c substr(v, i + 1, 1); i += 2; continue }
            if (c == "\"") break
            out = out c; i++
        }
        return decode_basic(out)
    }
    i = index(v, q)
    if (i > 0) v = substr(v, 1, i - 1)
    return v
}
function is_table_path(p) {
    if (p ~ /^(workspace\.)?(dependencies|dev-dependencies|build-dependencies)$/) return 1
    if (p == "workspace") return 1
    if (p ~ /^target\.[^.]+\.(dependencies|dev-dependencies|build-dependencies)$/) return 1
    return 0
}
function handle_ws_pair(key, rest,    alias, subkey, val) {
    if (rest ~ /^\{/) {
        record_ws(key, table_get(rest, "package"))
        return
    }
    if (rest ~ /^"/ || substr(rest, 1, 1) == "\x27") {
        if (index(key, ".") > 0) {
            alias = substr(key, 1, index(key, ".") - 1)
            subkey = substr(key, index(key, ".") + 1)
            val = str_value(rest)
            if (subkey == "package") {
                if (val != "") ws_name[alias] = val
                else if (!(alias in ws_name)) ws_name[alias] = alias
            }
            else if (!(alias in ws_name)) ws_name[alias] = alias
        } else {
            record_ws(key, "")
        }
        return
    }
    return
}
function handle_crate_pair(key, rest,    alias, pkg) {
    alias = key
    sub(/\..*$/, "", alias)
    if (rest ~ /^\{/) {
        if (table_get(rest, "workspace") == "true") {
            record_dep(alias, "workspace", "")
        } else {
            pkg = table_get(rest, "package")
            record_dep(alias, "direct", (pkg != "" ? pkg : alias))
        }
        return
    }
    if (rest ~ /^"/ || substr(rest, 1, 1) == "\x27") {
        if (alias != key) {
            record_dep(alias, "direct", str_value(rest))
        } else {
            record_dep(key, "direct", key)
        }
        return
    }
    return
}
function route_top(path, value, src,    t, inner, n, pairs, i, d, q, esc, c, cur, epos, j, pk, pv, fp) {
    t = trim(value)
    if (t !~ /^\{.*\}$/) parse_error("unsupported table form in " src)
    inner = substr(t, 2, length(t) - 2)
    n = 0; cur = ""; d = 0; q = ""; esc = 0
    for (i = 1; i <= length(inner); i++) {
        c = substr(inner, i, 1)
        if (q != "") {
            cur = cur c
            if (esc) esc = 0
            else if (q == "\"" && c == "\\") esc = 1
            else if (c == q) q = ""
            continue
        }
        if (c == "\"" || c == "\x27") { q = c; cur = cur c; continue }
        if (c == "{" || c == "[") d++
        if (c == "}" || c == "]") d--
        if (c == "," && d == 0) { pairs[++n] = cur; cur = ""; continue }
        cur = cur c
    }
    if (trim(cur) != "") pairs[++n] = cur
    for (i = 1; i <= n; i++) {
        epos = 0; d = 0; q = ""; esc = 0
        for (j = 1; j <= length(pairs[i]); j++) {
            c = substr(pairs[i], j, 1)
            if (q != "") {
                if (esc) esc = 0
                else if (q == "\"" && c == "\\") esc = 1
                else if (c == q) q = ""
                continue
            }
            if (c == "\"" || c == "\x27") { q = c; continue }
            if (c == "{" || c == "[") d++
            else if (c == "}" || c == "]") d--
            else if (c == "=" && d == 0) { epos = j; break }
        }
        if (epos == 0) parse_error("unsupported table form in " src)
        pk = normkey(trim(substr(pairs[i], 1, epos - 1)))
        pv = trim(substr(pairs[i], epos + 1))
        if (pk == "") parse_error("unsupported table form in " src)
        fp = path "." pk
        if (dep_section(path) || (src == "workspace manifest" && path == "workspace.dependencies")) {
            if (src == "workspace manifest") {
                handle_ws_pair(pk, pv)
            } else {
                handle_crate_pair(pk, pv)
            }
        } else if (pv ~ /^\{.*\}$/ && (is_table_path(fp) || is_table_prefix(fp))) {
            route_top(fp, pv, src)
        }
    }
}
function bdepth(s,    i, c, q, esc, d) {
    d = 0; q = ""; esc = 0; bdepth_open = 0
    for (i = 1; i <= length(s); i++) {
        c = substr(s, i, 1)
        if (q != "") {
            if (esc) esc = 0
            else if (q == "\"" && c == "\\") esc = 1
            else if (c == q) q = ""
            continue
        }
        if (c == "\"" || c == "\x27") q = c
        else if (c == "{" || c == "[") d++
        else if (c == "}" || c == "]") d--
    }
    if (q != "") bdepth_open = 1
    return d
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
FNR == 1 {
    if (buf != "") {
        if (file_index == 1) parse_error("unterminated table in workspace manifest")
        else parse_error("unterminated table in crate manifest")
    }
    if (pend_key != "") close_pending()
    buf = ""
    file_index++
    section = ""
}
{
    if (index($0, "\"\"\"") > 0) {
        if (file_index == 1) parse_error("unsupported multi-line string in workspace manifest")
        else parse_error("unsupported multi-line string in crate manifest")
    }
    part = trim(strip_comment($0))
    if (part != "") buf = (buf == "" ? part : buf "\n" part)
    if (buf == "") next
    if (bdepth(buf) != 0 || bdepth_open) next
    line = buf
    buf = ""
    if (line ~ /^\[.*\]$/) {
        close_pending()
        section = normkey(substr(line, 2, length(line) - 2))
        if (section ~ /(^|\.)(dependencies|dev-dependencies|build-dependencies)\.[^.]+$/) {
            pend_key = section
            sub(/^(.*\.)?(dependencies|dev-dependencies|build-dependencies)\./, "", pend_key)
            pend_key = normkey(pend_key)
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
    effsec = section
    eff = line
    if (section == "" && match(line, /^[^=]+=/)) {
        fullkey = trim(substr(line, 1, RLENGTH - 1))
        value = substr(line, RSTART + RLENGTH)
        k = fullkey
        gsub(/[ \t]*\.[ \t]*/, ".", k)
        k = normkey(k)
        prefix = ""
        rest = ""
        if (file_index == 1 && k ~ /^workspace\.dependencies\./) {
            prefix = "workspace.dependencies"
            rest = substr(k, 24)
        } else if (file_index == 2 && k ~ /^target\.[^.]+\.(dependencies|dev-dependencies|build-dependencies)\./) {
            t = k
            sub(/^target\.[^.]+\./, "", t)
            n = split(t, tp, /\./)
            prefix = tp[1]
            rest = substr(t, length(tp[1]) + 2)
        } else if (file_index == 2 && k ~ /^(dependencies|dev-dependencies|build-dependencies)\./) {
            n = split(k, tp, /\./)
            prefix = tp[1]
            rest = substr(k, length(tp[1]) + 2)
        }
        if (prefix != "" && rest != "") {
            effsec = prefix
            eff = rest " = " value
        } else if (prefix == "" && k != "" && (is_table_path(k) || is_table_prefix(k))) {
            if (file_index == 1 && k !~ /^workspace(\.|$)/) {
                t = trim(value)
                if (t !~ /^\{.*\}$/) parse_error("unsupported table form in workspace manifest")
                next
            }
            route_top(k, value, (file_index == 1 ? "workspace manifest" : "crate manifest"))
            next
        }
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
        if (effsec != "workspace.dependencies") next
        if (!match(eff, /^[^=]+=/)) parse_error("invalid TOML in workspace manifest")
        handle_ws_pair(normkey(trim(substr(eff, RSTART, RLENGTH - 1))), trim(substr(eff, RSTART + RLENGTH)))
        next
    }
    if (!dep_section(effsec)) next
    if (match(eff, /^[^=.]+[ \t]*\.[ \t]*workspace[ \t]*=[ \t]*true[ \t]*$/)) {
        record_dep(normkey(trim(substr(eff, 1, index(eff, ".") - 1))), "workspace", "")
        next
    }
    if (!match(eff, /^[^=]+=/)) parse_error("invalid TOML in crate manifest")
    handle_crate_pair(normkey(trim(substr(eff, RSTART, RLENGTH - 1))), trim(substr(eff, RSTART + RLENGTH)))
    next
}
END {
    if (bad) exit 2
    if (buf != "") {
        if (file_index == 2) parse_error("unterminated table in crate manifest")
        else parse_error("unterminated table in workspace manifest")
    }
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
