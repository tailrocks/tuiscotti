//! ANSI: normalized VT/SGR screen output regenerated from canonical state,
//! plus exact inverse decoding (`frame_from_ansi`) for `.ansi` + cursor-trailer
//! baselines.
//!
//! [`ansi_normalized`] is a *normalized projection*, not the original raw
//! transcript: SGR runs are recomputed from canonical cells in one canonical
//! parameter order (modifiers, then fg, then bg, then underline color), a
//! full reset (`ESC[0m`) precedes every style change, and every row ends with
//! a reset plus `\n`. Two captures of the same canonical screen produce
//! byte-identical ANSI regardless of which byte sequences the application
//! originally emitted.
//!
//! [`ansi_dump_with_cursor`] appends one canonical VT cursor trailer
//! (`ESC[?25l` when hidden, or `ESC[?25h ESC[<y>;<x>H ESC[<Ps> q` when visible),
//! and [`frame_from_ansi`] reconstructs the validated [`Frame`] from either
//! form so PNG/HTML review artifacts can be rebuilt on demand.

use tuiscotti_core::frame::{
    Cell, Color, Cursor, CursorStyle, FRAME_VERSION, Frame, Mods, Provenance, Rgb, UnderlineStyle,
};
use unicode_segmentation::UnicodeSegmentation;

pub use crate::formats::FormatError;
pub use crate::render::ansi_dump_with_cursor;

/// Render `frame` as normalized ANSI (see [`crate::render::ansi_dump`]).
#[must_use]
pub fn ansi_normalized(frame: &Frame) -> String {
    crate::render::ansi_dump(frame)
}

/// Fail unless `text` contains only normalized output: printable text, `\n`
/// row joins, `ESC[0m` resets, `ESC[<params>m` SGR runs with canonical
/// numeric/`;`/`:` parameters, and at most one canonical cursor trailer at the
/// very end (`ESC[?25l` or `ESC[?25h ESC[<y>;<x>H ESC[<1..=6> q`). Any other
/// escape sequence (OSC, mid-stream cursor moves, mode sets — raw-transcript
/// residue) is rejected.
///
/// # Errors
///
/// Returns `FormatError` naming the first non-normalized sequence found.
pub fn assert_normalized_sgr(text: &str) -> Result<(), FormatError> {
    let body = strip_optional_cursor_trailer(text)?.0;
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == 0x1b {
            i = check_escape(bytes, i)?;
        } else if b < 0x20 && b != b'\n' {
            return Err(FormatError(format!(
                "ANSI carries raw control byte 0x{b:02x} at offset {i}"
            )));
        } else {
            i += 1;
        }
    }
    Ok(())
}

/// Validate one escape sequence at `bytes[i] == ESC`; return the offset past it.
fn check_escape(bytes: &[u8], i: usize) -> Result<usize, FormatError> {
    let rest = &bytes[i..];
    if rest.len() < 3 || rest[1] != b'[' {
        return Err(FormatError(format!(
            "ANSI carries non-CSI escape at offset {i}"
        )));
    }
    let mut j = i + 2;
    while j < bytes.len() && (bytes[j].is_ascii_digit() || bytes[j] == b';' || bytes[j] == b':') {
        j += 1;
    }
    if j >= bytes.len() || bytes[j] != b'm' {
        return Err(FormatError(format!(
            "ANSI carries non-SGR escape at offset {i}"
        )));
    }
    Ok(j + 1)
}

/// Reconstruct a validated [`Frame`] from [`ansi_dump_with_cursor`] (or bare
/// [`crate::render::ansi_dump`] without a cursor trailer, which defaults to a
/// hidden cursor at `(0, 0)`).
///
/// Grid width is recovered from row 0's cell widths. Every SGR sequence, color,
/// underline style, wide-char continuation, and optional cursor trailer is
/// strictly validated, followed by [`Frame::validate`].
///
/// # Errors
///
/// Returns [`FormatError`] when `text` is not a well-formed normalized ANSI
/// frame or fails [`Frame::validate`].
pub fn frame_from_ansi(text: &str, provenance: Provenance) -> Result<Frame, FormatError> {
    let (body, cursor) = strip_optional_cursor_trailer(text)?;
    if body.is_empty() {
        return Err(FormatError("ANSI frame body is empty".to_string()));
    }
    if !body.ends_with('\n') {
        return Err(FormatError(
            "ANSI frame body must end with newline".to_string(),
        ));
    }
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    for line in body.split_terminator('\n') {
        let y = u16::try_from(rows.len())
            .map_err(|_| FormatError("ANSI row count overflows u16".to_string()))?;
        rows.push(decode_row(line, y)?);
    }
    let cols = rows
        .first()
        .ok_or_else(|| FormatError("ANSI frame has no rows".to_string()))
        .and_then(|r| {
            u16::try_from(r.len())
                .map_err(|_| FormatError("ANSI column count overflows u16".to_string()))
        })?;
    let nrows = u16::try_from(rows.len())
        .map_err(|_| FormatError("ANSI row count overflows u16".to_string()))?;
    for (idx, row) in rows.iter().enumerate() {
        if row.len() != usize::from(cols) {
            return Err(FormatError(format!(
                "ANSI row {idx} has {} cells, expected {cols}",
                row.len()
            )));
        }
    }
    let frame = Frame {
        version: FRAME_VERSION,
        cols,
        rows: nrows,
        cells: rows.into_iter().flatten().collect(),
        cursor,
        provenance,
    };
    frame
        .validate()
        .map_err(|e| FormatError(format!("decoded ANSI frame invalid: {e}")))?;
    Ok(frame)
}

/// Split `text` into `(body, cursor)`, validating the trailing cursor sequence
/// if `\x1b[?25` appears.
fn strip_optional_cursor_trailer(text: &str) -> Result<(&str, Cursor), FormatError> {
    let Some(i) = text.rfind("\x1b[?25") else {
        return Ok((text, Cursor::default()));
    };
    let body = &text[..i];
    let trailer = &text[i..];
    if body.contains("\x1b[?25") {
        return Err(FormatError(
            "ANSI contains multiple cursor visibility sequences".to_string(),
        ));
    }
    if trailer == "\x1b[?25l" {
        return Ok((body, Cursor::default()));
    }
    let Some(t) = trailer.strip_prefix("\x1b[?25h\x1b[") else {
        return Err(FormatError(format!(
            "malformed ANSI cursor trailer: {trailer:?}"
        )));
    };
    let Some(h) = t.find('H') else {
        return Err(FormatError(format!(
            "malformed ANSI cursor trailer (missing H): {trailer:?}"
        )));
    };
    let coords = &t[..h];
    let Some((y_str, x_str)) = coords.split_once(';') else {
        return Err(FormatError(format!(
            "malformed ANSI cursor position `{coords}`"
        )));
    };
    let y1 = parse_strict_u16_nonzero(y_str, "cursor row")?;
    let x1 = parse_strict_u16_nonzero(x_str, "cursor col")?;
    let after_h = &t[h + 1..];
    let Some(ps_str) = after_h
        .strip_prefix("\x1b[")
        .and_then(|s| s.strip_suffix(" q"))
    else {
        return Err(FormatError(format!(
            "malformed ANSI DECSCUSR sequence in trailer: {trailer:?}"
        )));
    };
    if ps_str.len() != 1 {
        return Err(FormatError(format!(
            "invalid DECSCUSR parameter `{ps_str}`"
        )));
    }
    let ps: u8 = ps_str
        .parse()
        .map_err(|_| FormatError(format!("invalid DECSCUSR parameter `{ps_str}`")))?;
    let style = match ps {
        1 | 2 => CursorStyle::Block,
        3 | 4 => CursorStyle::Underline,
        5 | 6 => CursorStyle::Bar,
        _ => {
            return Err(FormatError(format!(
                "DECSCUSR parameter {ps} out of range 1..=6"
            )));
        }
    };
    Ok((
        body,
        Cursor {
            x: x1 - 1,
            y: y1 - 1,
            visible: true,
            style,
            blinking: ps % 2 == 1,
        },
    ))
}

fn parse_strict_u16_nonzero(s: &str, label: &str) -> Result<u16, FormatError> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) || s.starts_with('0') {
        return Err(FormatError(format!("invalid 1-based {label} `{s}`")));
    }
    s.parse::<u16>()
        .map_err(|_| FormatError(format!("{label} `{s}` overflows u16")))
}

#[derive(Clone, Copy, Default)]
struct Style {
    fg: Color,
    bg: Color,
    ul: Color,
    mods: Mods,
}

fn decode_row(line: &str, y: u16) -> Result<Vec<Cell>, FormatError> {
    let mut cells: Vec<Cell> = Vec::new();
    let mut st = Style::default();
    let mut rest = line;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("\x1b[") {
            let Some(m) = after.find('m') else {
                return Err(FormatError(format!("unterminated CSI sequence on row {y}")));
            };
            let p = &after[..m];
            st = if p == "0" {
                Style::default()
            } else {
                parse_sgr(p)?
            };
            rest = &after[m + 1..];
            continue;
        }
        if rest.starts_with('\x1b') {
            return Err(FormatError(format!("non-CSI escape sequence on row {y}")));
        }
        let end = rest.find('\x1b').unwrap_or(rest.len());
        for g in rest[..end].graphemes(true) {
            if g.bytes().any(|b| b < 0x20) {
                return Err(FormatError(format!(
                    "raw control byte in text on row {y}: {g:?}"
                )));
            }
            let wide = unicode_width::UnicodeWidthStr::width(g) == 2;
            let x = u16::try_from(cells.len())
                .map_err(|_| FormatError("ANSI column index overflows u16".to_string()))?;
            let lead = Cell {
                x,
                y,
                symbol: g.to_string(),
                width: if wide { 2 } else { 1 },
                continuation: false,
                fg: st.fg,
                bg: st.bg,
                mods: st.mods,
                underline_color: st.ul,
            };
            if wide {
                let cont_x = x
                    .checked_add(1)
                    .ok_or_else(|| FormatError("wide cell column overflows u16".to_string()))?;
                let cont = Cell {
                    x: cont_x,
                    y,
                    symbol: String::new(),
                    width: 0,
                    continuation: true,
                    fg: lead.fg,
                    bg: lead.bg,
                    mods: lead.mods,
                    underline_color: lead.underline_color,
                };
                cells.push(lead);
                cells.push(cont);
            } else {
                cells.push(lead);
            }
        }
        rest = &rest[end..];
    }
    Ok(cells)
}

fn parse_color(toks: &[&str], i: &mut usize) -> Result<Color, FormatError> {
    let kind = *toks
        .get(*i + 1)
        .ok_or_else(|| FormatError("truncated SGR color".to_string()))?;
    if kind == "5" {
        let idx_str = *toks
            .get(*i + 2)
            .ok_or_else(|| FormatError("truncated indexed SGR color".to_string()))?;
        let idx: u8 = idx_str
            .parse()
            .map_err(|_| FormatError(format!("bad indexed color `{idx_str}`")))?;
        *i += 3;
        Ok(Color::Indexed(idx))
    } else if kind == "2" {
        let parse_ch = |off: usize| -> Result<u8, FormatError> {
            let s = *toks
                .get(*i + off)
                .ok_or_else(|| FormatError("truncated RGB SGR color".to_string()))?;
            s.parse::<u8>()
                .map_err(|_| FormatError(format!("bad RGB channel `{s}`")))
        };
        let c = Color::Rgb(Rgb::new(parse_ch(2)?, parse_ch(3)?, parse_ch(4)?));
        *i += 5;
        Ok(c)
    } else {
        Err(FormatError(format!("unknown SGR color mode `{kind}`")))
    }
}

fn parse_sgr(params: &str) -> Result<Style, FormatError> {
    if params.is_empty() {
        return Err(FormatError("empty SGR sequence".to_string()));
    }
    let mut st = Style::default();
    let toks: Vec<&str> = params.split(';').collect();
    let mut i = 0;
    while i < toks.len() {
        match toks[i] {
            "1" => st.mods.bold = true,
            "2" => st.mods.dim = true,
            "3" => st.mods.italic = true,
            "5" => st.mods.blink = true,
            "7" => st.mods.reverse = true,
            "8" => st.mods.hidden = true,
            "9" => st.mods.strikethrough = true,
            "4" | "4:2" | "4:3" | "4:4" | "4:5" => {
                st.mods.underline = true;
                st.mods.underline_style = match toks[i] {
                    "4" => UnderlineStyle::Single,
                    "4:2" => UnderlineStyle::Double,
                    "4:3" => UnderlineStyle::Curly,
                    "4:4" => UnderlineStyle::Dotted,
                    _ => UnderlineStyle::Dashed,
                };
            }
            "38" => {
                st.fg = parse_color(&toks, &mut i)?;
                continue;
            }
            "48" => {
                st.bg = parse_color(&toks, &mut i)?;
                continue;
            }
            "58" => {
                st.ul = parse_color(&toks, &mut i)?;
                continue;
            }
            other => {
                return Err(FormatError(format!("unsupported SGR parameter `{other}`")));
            }
        }
        i += 1;
    }
    Ok(st)
}
