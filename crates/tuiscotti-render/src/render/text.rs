//! One-shot PNG conveniences and the normalized ANSI dump.

use super::{RenderError, Rendered, Renderer};
use crate::profile::FontFaces;
use crate::profile::Profile;
use tuiscotti_core::frame::Frame;

/// Render a validated frame to PNG bytes under `profile`.
///
/// One-shot convenience: the default gate renders through the thread-local
/// shared [`Renderer`] (faces parsed once per thread, warm glyph cache);
/// custom profiles/faces construct a fresh instance per call (bulk gates
/// with custom profiles should keep a `Renderer` instead).
///
/// # Errors
///
/// Returns `RenderError` when the frame is invalid or PNG encoding fails.
pub fn render_png(
    frame: &Frame,
    profile: &Profile,
    faces: &FontFaces<'_>,
) -> Result<Vec<u8>, RenderError> {
    Renderer::with_profile(profile, faces, |r| r.render_png(frame))
}

/// Render plus exact coverage accounting (see [`Fidelity`](super::Fidelity)).
///
/// One-shot convenience: same sharing as [`render_png`] — the default gate
/// reuses the thread-local shared [`Renderer`], custom profiles/faces
/// construct per call.
///
/// # Errors
///
/// Returns `RenderError` when the frame is invalid or PNG encoding fails.
pub fn render_png_report(
    frame: &Frame,
    profile: &Profile,
    faces: &FontFaces<'_>,
) -> Result<Rendered, RenderError> {
    Renderer::with_profile(profile, faces, |r| r.render(frame))
}

/// Normalized ANSI dump (SGR runs from canonical state — for debugging, not
/// for replay; replay raw streams with `crate::ansi::replay_raw`).
#[must_use]
pub fn ansi_dump(frame: &Frame) -> String {
    let mut out = String::new();
    for y in 0..frame.rows {
        let mut cur = String::new();
        let mut prev_sym = "";
        for x in 0..frame.cols {
            let Some(c) = frame.get(x, y) else { continue };
            if c.continuation {
                continue;
            }
            let sgr = sgr_for(c);
            let boundary = !prev_sym.is_empty() && graphemes_join(prev_sym, &c.symbol);
            if sgr != cur || boundary {
                out.push_str("\x1b[0m");
                if !sgr.is_empty() {
                    out.push_str("\x1b[");
                    out.push_str(&sgr);
                    out.push('m');
                }
                cur = sgr;
            }
            out.push_str(&c.symbol);
            prev_sym = &c.symbol;
        }
        out.push_str("\x1b[0m\n");
    }
    out
}

fn graphemes_join(a: &str, b: &str) -> bool {
    use unicode_segmentation::UnicodeSegmentation;
    let mut joined = String::with_capacity(a.len() + b.len());
    joined.push_str(a);
    joined.push_str(b);
    joined.graphemes(true).count() != a.graphemes(true).count() + b.graphemes(true).count()
}

/// Normalized ANSI dump plus one canonical VT cursor trailer: `ESC[?25l` when
/// the cursor is hidden, or `ESC[?25h ESC[<y>;<x>H ESC[<Ps> q` (1-based
/// row/col and DECSCUSR shape/blink `1..=6`) when visible.
#[must_use]
pub fn ansi_dump_with_cursor(frame: &Frame) -> String {
    use tuiscotti_core::frame::CursorStyle;
    let mut s = ansi_dump(frame);
    let c = frame.cursor;
    if c.visible {
        use std::fmt::Write as _;
        let ps = match (c.style, c.blinking) {
            (CursorStyle::Block, true) => 1,
            (CursorStyle::Block, false) => 2,
            (CursorStyle::Underline, true) => 3,
            (CursorStyle::Underline, false) => 4,
            (CursorStyle::Bar, true) => 5,
            (CursorStyle::Bar, false) => 6,
        };
        write!(
            s,
            "\x1b[?25h\x1b[{};{}H\x1b[{ps} q",
            u32::from(c.y) + 1,
            u32::from(c.x) + 1
        )
        .ok();
    } else {
        s.push_str("\x1b[?25l");
    }
    s
}

fn sgr_for(c: &tuiscotti_core::frame::Cell) -> String {
    let mut p: Vec<String> = Vec::new();
    if c.mods.hidden {
        p.push("8".into());
    }
    if c.mods.blink {
        p.push("5".into());
    }
    if c.mods.bold {
        p.push("1".into());
    }
    if c.mods.dim {
        p.push("2".into());
    }
    if c.mods.italic {
        p.push("3".into());
    }
    match c.mods.effective_underline_style() {
        tuiscotti_core::frame::UnderlineStyle::None => {}
        tuiscotti_core::frame::UnderlineStyle::Single => p.push("4".into()),
        tuiscotti_core::frame::UnderlineStyle::Double => p.push("4:2".into()),
        tuiscotti_core::frame::UnderlineStyle::Curly => p.push("4:3".into()),
        tuiscotti_core::frame::UnderlineStyle::Dotted => p.push("4:4".into()),
        tuiscotti_core::frame::UnderlineStyle::Dashed => p.push("4:5".into()),
    }
    if c.mods.strikethrough {
        p.push("9".into());
    }
    if c.mods.reverse {
        p.push("7".into());
    }
    let push_color = |p: &mut Vec<String>, code: u8, c: tuiscotti_core::frame::Color| match c {
        tuiscotti_core::frame::Color::Default => {}
        tuiscotti_core::frame::Color::Indexed(i) => p.push(format!("{code};5;{i}")),
        tuiscotti_core::frame::Color::Rgb(r) => {
            p.push(format!("{code};2;{};{};{}", r.r, r.g, r.b));
        }
    };
    push_color(&mut p, 38, c.fg);
    push_color(&mut p, 48, c.bg);
    push_color(&mut p, 58, c.underline_color);
    p.join(";")
}
