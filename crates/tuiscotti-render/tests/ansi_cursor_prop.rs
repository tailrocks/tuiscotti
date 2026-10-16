//! Property and corpus tests for `ansi_dump_with_cursor` <-> `frame_from_ansi`.

use std::path::{Path, PathBuf};
use tuiscotti_core::frame::{
    Cell, Color, Cursor, CursorStyle, Frame, Mods, Provenance, Rgb, UnderlineStyle,
};
use tuiscotti_render::formats::ansi::{assert_normalized_sgr, frame_from_ansi};
use tuiscotti_render::render::{ansi_dump, ansi_dump_with_cursor};

struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn gen_range(&mut self, upper: usize) -> usize {
        usize::try_from(self.next_u64() % u64::try_from(upper).unwrap_or(1)).unwrap_or(0)
    }

    fn gen_u8(&mut self) -> u8 {
        u8::try_from(self.next_u64() & 0xff).unwrap_or(0)
    }

    fn gen_bool(&mut self) -> bool {
        (self.next_u64() & 1) == 1
    }

    fn gen_color(&mut self) -> Color {
        match self.gen_range(3) {
            0 => Color::Default,
            1 => Color::Indexed(self.gen_u8()),
            _ => Color::Rgb(Rgb::new(self.gen_u8(), self.gen_u8(), self.gen_u8())),
        }
    }

    fn gen_mods(&mut self) -> Mods {
        let underline = self.gen_bool();
        let underline_style = if underline {
            match self.gen_range(5) {
                0 => UnderlineStyle::Single,
                1 => UnderlineStyle::Double,
                2 => UnderlineStyle::Curly,
                3 => UnderlineStyle::Dotted,
                _ => UnderlineStyle::Dashed,
            }
        } else {
            UnderlineStyle::None
        };
        Mods {
            bold: self.gen_bool(),
            dim: self.gen_bool(),
            italic: self.gen_bool(),
            underline,
            underline_style,
            blink: self.gen_bool(),
            reverse: self.gen_bool(),
            hidden: self.gen_bool(),
            strikethrough: self.gen_bool(),
        }
    }

    fn gen_cursor(&mut self, cols: u16, rows: u16) -> Cursor {
        if self.gen_range(4) == 0 {
            Cursor::default()
        } else {
            let style = match self.gen_range(3) {
                0 => CursorStyle::Block,
                1 => CursorStyle::Underline,
                _ => CursorStyle::Bar,
            };
            Cursor {
                x: u16::try_from(self.gen_range(usize::from(cols))).unwrap_or(0),
                y: u16::try_from(self.gen_range(usize::from(rows))).unwrap_or(0),
                visible: true,
                style,
                blinking: self.gen_bool(),
            }
        }
    }
}

const NARROW_GLYPHS: &[&str] = &[
    " ",
    "a",
    "Z",
    "0",
    "─",
    "│",
    "╭",
    "╮",
    "\u{00a0}",
    "e\u{0301}",
    "a\u{0300}\u{0323}",
    "o\u{0308}",
];

const WIDE_GLYPHS: &[&str] = &["界", "漢", "語", "🦀", "🧪"];

#[test]
fn proptest_ansi_dump_with_cursor_and_frame_from_ansi_round_trip() {
    let mut rng = Rng(0x5eed_cafe_d00d_beef);
    for case in 0..500 {
        let cols = u16::try_from(1 + rng.gen_range(32)).expect("cols u16");
        let rows = u16::try_from(1 + rng.gen_range(12)).expect("rows u16");
        let mut frame = Frame::blank(cols, rows, Provenance::default());
        frame.cursor = rng.gen_cursor(cols, rows);

        for y in 0..rows {
            let mut x = 0;
            while x < cols {
                let use_wide = x + 1 < cols && rng.gen_range(5) == 0;
                let fg = rng.gen_color();
                let bg = rng.gen_color();
                let mods = rng.gen_mods();
                let underline_color = if mods.underline {
                    rng.gen_color()
                } else {
                    Color::Default
                };
                if use_wide {
                    let sym = WIDE_GLYPHS[rng.gen_range(WIDE_GLYPHS.len())];
                    frame.set(Cell {
                        x,
                        y,
                        symbol: sym.to_string(),
                        width: 2,
                        continuation: false,
                        fg,
                        bg,
                        mods,
                        underline_color,
                    });
                    frame.set(Cell {
                        x: x + 1,
                        y,
                        symbol: String::new(),
                        width: 0,
                        continuation: true,
                        fg,
                        bg,
                        mods,
                        underline_color,
                    });
                    x += 2;
                } else {
                    let sym = NARROW_GLYPHS[rng.gen_range(NARROW_GLYPHS.len())];
                    frame.set(Cell {
                        x,
                        y,
                        symbol: sym.to_string(),
                        width: 1,
                        continuation: false,
                        fg,
                        bg,
                        mods,
                        underline_color,
                    });
                    x += 1;
                }
            }
        }

        frame.validate().expect("generated frame must be valid");
        let encoded = ansi_dump_with_cursor(&frame);
        assert_normalized_sgr(&encoded)
            .unwrap_or_else(|e| panic!("case {case}: assert_normalized_sgr failed: {e}"));
        let decoded = frame_from_ansi(&encoded, Provenance::default())
            .unwrap_or_else(|e| panic!("case {case}: frame_from_ansi failed: {e}"));
        assert_eq!(decoded.cols, frame.cols, "case {case}: cols");
        assert_eq!(decoded.rows, frame.rows, "case {case}: rows");
        assert_eq!(decoded.cursor, frame.cursor, "case {case}: cursor");
        assert_eq!(decoded.cells, frame.cells, "case {case}: cells");
        assert_eq!(
            ansi_dump_with_cursor(&decoded),
            encoded,
            "case {case}: re-encoded ANSI bytes"
        );
    }
}

fn collect_ansi_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_ansi_files(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("ansi") {
            out.push(path);
        }
    }
    Ok(())
}

#[test]
fn external_ansi_corpus_round_trips_byte_identically_when_configured() {
    let Some(dir) = std::env::var_os("TUISCOTTI_ANSI_CORPUS_DIR") else {
        return;
    };
    let root = PathBuf::from(dir);
    let mut files = Vec::new();
    collect_ansi_files(&root, &mut files).expect("walk TUISCOTTI_ANSI_CORPUS_DIR");
    files.sort();
    assert!(
        !files.is_empty(),
        "no .ansi files found under {}",
        root.display()
    );
    let mut with_trailer = 0_usize;
    let mut legacy_no_trailer = 0_usize;
    let mut visible_cursors = 0_usize;
    for path in &files {
        let original = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        assert_normalized_sgr(&original)
            .unwrap_or_else(|e| panic!("assert_normalized_sgr failed for {}: {e}", path.display()));
        let frame = frame_from_ansi(&original, Provenance::default())
            .unwrap_or_else(|e| panic!("frame_from_ansi failed for {}: {e}", path.display()));
        if frame.cursor.visible {
            visible_cursors += 1;
        }
        if original.contains("\x1b[?25") {
            with_trailer += 1;
            let reencoded = ansi_dump_with_cursor(&frame);
            assert_eq!(
                reencoded,
                original,
                "byte mismatch on cursor-trailer corpus file {}",
                path.display()
            );
        } else {
            legacy_no_trailer += 1;
            assert_eq!(
                ansi_dump(&frame),
                original,
                "byte mismatch on legacy corpus file {}",
                path.display()
            );
            assert_eq!(
                ansi_dump_with_cursor(&frame),
                format!("{original}\x1b[?25l"),
                "trailer append mismatch on legacy corpus file {}",
                path.display()
            );
        }
    }
    eprintln!(
        "verified {} .ansi corpus files ({} with cursor trailer, {} legacy trailerless, {} with visible cursor)",
        files.len(),
        with_trailer,
        legacy_no_trailer,
        visible_cursors
    );
}

fn set_wide(frame: &mut Frame, x: u16, y: u16, sym: &str) {
    frame.set(Cell {
        x,
        y,
        symbol: sym.to_string(),
        width: 2,
        continuation: false,
        fg: Color::Default,
        bg: Color::Default,
        mods: Mods::default(),
        underline_color: Color::Default,
    });
    frame.set(Cell {
        x: x + 1,
        y,
        symbol: String::new(),
        width: 0,
        continuation: true,
        fg: Color::Default,
        bg: Color::Default,
        mods: Mods::default(),
        underline_color: Color::Default,
    });
}

fn set_narrow(frame: &mut Frame, x: u16, y: u16, sym: &str) {
    frame.set(Cell {
        x,
        y,
        symbol: sym.to_string(),
        width: 1,
        continuation: false,
        fg: Color::Default,
        bg: Color::Default,
        mods: Mods::default(),
        underline_color: Color::Default,
    });
}

#[test]
fn cross_cell_grapheme_clusters_round_trip_without_joining() {
    // 1. Regional-indicator pair in two separate width-1 cells.
    let mut ri = Frame::blank(4, 2, Provenance::default());
    set_narrow(&mut ri, 0, 0, "\u{1F1FA}");
    set_narrow(&mut ri, 1, 0, "\u{1F1F8}");

    // 2. 'e' followed by a cell holding only U+0301 (combining acute accent).
    let mut combining = Frame::blank(4, 2, Provenance::default());
    set_narrow(&mut combining, 0, 0, "e");
    set_narrow(&mut combining, 1, 0, "\u{0301}");

    // 3. Emoji ZWJ sequence split across cells (3-cell split and 2-cell split).
    let mut zwj_a = Frame::blank(6, 2, Provenance::default());
    set_wide(&mut zwj_a, 0, 0, "\u{1F469}");
    set_narrow(&mut zwj_a, 2, 0, "\u{200D}");
    set_wide(&mut zwj_a, 3, 0, "\u{1F4BB}");

    let mut zwj_b = Frame::blank(6, 2, Provenance::default());
    set_wide(&mut zwj_b, 0, 0, "\u{1F469}\u{200D}");
    set_wide(&mut zwj_b, 2, 0, "\u{1F4BB}");

    // 4. Hangul L + V jamo in adjacent cells.
    let mut hangul = Frame::blank(4, 2, Provenance::default());
    set_wide(&mut hangul, 0, 0, "\u{1100}");
    set_narrow(&mut hangul, 2, 0, "\u{1161}");

    for (label, frame) in [
        ("regional-indicator", ri),
        ("e+combining", combining),
        ("zwj-after", zwj_a),
        ("zwj-before", zwj_b),
        ("hangul-l-v", hangul),
    ] {
        frame
            .validate()
            .unwrap_or_else(|e| panic!("{label}: frame validate failed: {e}"));
        let encoded = ansi_dump_with_cursor(&frame);
        assert_normalized_sgr(&encoded)
            .unwrap_or_else(|e| panic!("{label}: assert_normalized_sgr failed: {e}"));
        let decoded = frame_from_ansi(&encoded, Provenance::default())
            .unwrap_or_else(|e| panic!("{label}: frame_from_ansi failed: {e}"));
        assert_eq!(decoded.cells, frame.cells, "{label}: cells round-trip");
        assert_eq!(
            ansi_dump_with_cursor(&decoded),
            encoded,
            "{label}: re-encoded bytes"
        );
    }
}
