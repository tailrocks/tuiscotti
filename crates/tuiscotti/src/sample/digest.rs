#![doc = include_str!("../../docs/digest.md")]

use std::io::Write;

use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};
use tui_test::{
    SnapshotCell, SnapshotColor, SnapshotCursor, SnapshotScope, SnapshotUnderline, TerminalSnapshot,
};

use super::{ContractFont, CursorPhase, RenderContract};

const CELL_INSERTION_ORDER: &[&str] = &[
    "text",
    "width",
    "fg",
    "bg",
    "bold",
    "dim",
    "italic",
    "inverse",
    "invisible",
    "strike",
    "blink",
    "underline",
    "underline_color",
    "link_uri",
    "link_id",
];
const CELL_SORTED_ORDER: &[&str] = &[
    "bg",
    "blink",
    "bold",
    "dim",
    "fg",
    "inverse",
    "invisible",
    "italic",
    "link_id",
    "link_uri",
    "strike",
    "text",
    "underline",
    "underline_color",
    "width",
];
const CURSOR_INSERTION_ORDER: &[&str] = &[
    "col",
    "row",
    "in_capture",
    "visible",
    "shape",
    "color",
    "blink",
];
const CURSOR_SORTED_ORDER: &[&str] = &[
    "blink",
    "col",
    "color",
    "in_capture",
    "row",
    "shape",
    "visible",
];

struct Sorted;

trait SortedFallback {
    fn swap_remove(&mut self, key: &str) -> Sorted;
}

impl SortedFallback for serde_json::Map<String, serde_json::Value> {
    fn swap_remove(&mut self, _: &str) -> Sorted {
        Sorted
    }
}

trait OrderingMarker {
    const PRESERVED: bool;
}

impl OrderingMarker for Sorted {
    const PRESERVED: bool = false;
}

impl OrderingMarker for Option<serde_json::Value> {
    const PRESERVED: bool = true;
}

fn preserves_order<F, R>(_: F) -> bool
where
    F: FnOnce(&mut serde_json::Map<String, serde_json::Value>) -> R,
    R: OrderingMarker,
{
    std::hint::black_box(
        SortedFallback::swap_remove
            as fn(&mut serde_json::Map<String, serde_json::Value>, &str) -> Sorted,
    );
    R::PRESERVED
}

fn snapshot_order_preserved() -> bool {
    preserves_order(|map| map.swap_remove("probe"))
}

/// Stream the old metadata-free snapshot Value bytes through `writer`.
pub(super) fn write_snapshot_projection<W: Write>(
    snapshot: &TerminalSnapshot,
    writer: W,
) -> Result<(), serde_json::Error> {
    serde_json::to_writer(
        writer,
        &SnapshotProjection(snapshot, snapshot_order_preserved()),
    )
}

/// Stream the legacy declaration-ordered render-contract bytes through `writer`.
pub(super) fn write_contract_projection<W: Write>(
    contract: &RenderContract,
    writer: W,
) -> Result<(), serde_json::Error> {
    serde_json::to_writer(writer, &ContractProjection(contract))
}

struct SnapshotProjection<'a>(&'a TerminalSnapshot, bool);

impl Serialize for SnapshotProjection<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let snapshot = self.0;
        let sorted = !self.1;
        let mut map = serializer.serialize_map(Some(12))?;
        if sorted {
            map.serialize_entry("cells", &Cells(snapshot.cells(), sorted))?;
            map.serialize_entry("cols", &snapshot.cols())?;
            map.serialize_entry("cursor", &CursorProjection(snapshot.cursor(), sorted))?;
            map.serialize_entry("default_background", &Rgb(snapshot.default_background()))?;
            map.serialize_entry("default_cursor", &Rgb(snapshot.default_cursor()))?;
            map.serialize_entry("default_foreground", &Rgb(snapshot.default_foreground()))?;
            map.serialize_entry("palette", &Palette(snapshot.palette()))?;
            map.serialize_entry("rows", &snapshot.rows())?;
            map.serialize_entry("schema", &tui_test::SNAPSHOT_SCHEMA_VERSION)?;
            map.serialize_entry("scope", &Scope(snapshot.scope()))?;
            map.serialize_entry("title", &snapshot.title())?;
            map.serialize_entry("viewport_origin", &snapshot.viewport_origin())?;
        } else {
            map.serialize_entry("schema", &tui_test::SNAPSHOT_SCHEMA_VERSION)?;
            map.serialize_entry("cols", &snapshot.cols())?;
            map.serialize_entry("rows", &snapshot.rows())?;
            map.serialize_entry("scope", &Scope(snapshot.scope()))?;
            map.serialize_entry("viewport_origin", &snapshot.viewport_origin())?;
            map.serialize_entry("cells", &Cells(snapshot.cells(), sorted))?;
            map.serialize_entry("palette", &Palette(snapshot.palette()))?;
            map.serialize_entry("default_foreground", &Rgb(snapshot.default_foreground()))?;
            map.serialize_entry("default_background", &Rgb(snapshot.default_background()))?;
            map.serialize_entry("default_cursor", &Rgb(snapshot.default_cursor()))?;
            map.serialize_entry("cursor", &CursorProjection(snapshot.cursor(), sorted))?;
            map.serialize_entry("title", &snapshot.title())?;
        }
        map.end()
    }
}

struct Cells<'a>(&'a [SnapshotCell], bool);

impl Serialize for Cells<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for cell in self.0 {
            seq.serialize_element(&CellProjection(cell, self.1))?;
        }
        seq.end()
    }
}

struct CellProjection<'a>(&'a SnapshotCell, bool);

impl Serialize for CellProjection<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use tui_test::terminal::cell::Attrs;
        let cell = self.0;
        let mut map = serializer.serialize_map(Some(15))?;
        for key in if self.1 {
            CELL_SORTED_ORDER
        } else {
            CELL_INSERTION_ORDER
        } {
            match *key {
                "text" => map.serialize_entry(key, cell.text())?,
                "width" => map.serialize_entry(key, &cell.width())?,
                "fg" => map.serialize_entry(key, &Color(cell.fg()))?,
                "bg" => map.serialize_entry(key, &Color(cell.bg()))?,
                "bold" => map.serialize_entry(key, &cell.has(Attrs::BOLD))?,
                "dim" => map.serialize_entry(key, &cell.has(Attrs::DIM))?,
                "italic" => map.serialize_entry(key, &cell.has(Attrs::ITALIC))?,
                "inverse" => map.serialize_entry(key, &cell.has(Attrs::INVERSE))?,
                "invisible" => map.serialize_entry(key, &cell.has(Attrs::INVISIBLE))?,
                "strike" => map.serialize_entry(key, &cell.has(Attrs::STRIKE))?,
                "blink" => map.serialize_entry(key, &cell.has(Attrs::BLINK))?,
                "underline" => map.serialize_entry(key, &Underline(cell.underline()))?,
                "underline_color" => map.serialize_entry(key, &Color(cell.underline_color()))?,
                "link_uri" => map.serialize_entry(key, &cell.link_uri())?,
                "link_id" => map.serialize_entry(key, &cell.link_id())?,
                _ => unreachable!("static projection key"),
            }
        }
        map.end()
    }
}

struct CursorProjection<'a>(&'a SnapshotCursor, bool);

impl Serialize for CursorProjection<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let cursor = self.0;
        let mut map = serializer.serialize_map(Some(7))?;
        for key in if self.1 {
            CURSOR_SORTED_ORDER
        } else {
            CURSOR_INSERTION_ORDER
        } {
            match *key {
                "col" => map.serialize_entry(key, &cursor.col())?,
                "row" => map.serialize_entry(key, &cursor.row())?,
                "in_capture" => map.serialize_entry(key, &cursor.in_capture())?,
                "visible" => map.serialize_entry(key, &cursor.visible())?,
                "shape" => map.serialize_entry(key, &cursor.shape().name())?,
                "color" => map.serialize_entry(key, &Rgb(cursor.color()))?,
                "blink" => map.serialize_entry(key, &cursor.blink())?,
                _ => unreachable!("static projection key"),
            }
        }
        map.end()
    }
}

struct Palette<'a>(&'a [tui_test::profile::Rgb; 256]);

impl Serialize for Palette<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for color in self.0 {
            seq.serialize_element(&Rgb(*color))?;
        }
        seq.end()
    }
}

struct Rgb(tui_test::profile::Rgb);

impl Serialize for Rgb {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut bytes = [0u8; 7];
        bytes[0] = b'#';
        for (index, value) in [self.0.r, self.0.g, self.0.b].into_iter().enumerate() {
            bytes[1 + index * 2] = hex(value >> 4);
            bytes[2 + index * 2] = hex(value & 15);
        }
        match std::str::from_utf8(&bytes) {
            Ok(text) => serializer.serialize_str(text),
            Err(_) => Err(serde::ser::Error::custom("invalid internal RGB encoding")),
        }
    }
}

fn hex(value: u8) -> u8 {
    match value {
        0..=9 => b'0' + value,
        _ => b'a' + value - 10,
    }
}

struct Color(SnapshotColor);

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            SnapshotColor::Default => serializer.serialize_str("default"),
            SnapshotColor::Indexed(index) => serializer.serialize_u8(index),
            SnapshotColor::Rgb(r, g, b) => {
                Rgb(tui_test::profile::Rgb::new(r, g, b)).serialize(serializer)
            }
        }
    }
}

struct Scope(SnapshotScope);

impl Serialize for Scope {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self.0 {
            SnapshotScope::Viewport => "viewport",
            SnapshotScope::ActiveScreen => "active_screen",
            SnapshotScope::FullHistory => "full_history",
        })
    }
}

struct Underline(SnapshotUnderline);

impl Serialize for Underline {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self.0 {
            SnapshotUnderline::None => "none",
            SnapshotUnderline::Single => "single",
            SnapshotUnderline::Double => "double",
            SnapshotUnderline::Curly => "curly",
            SnapshotUnderline::Dotted => "dotted",
            SnapshotUnderline::Dashed => "dashed",
        })
    }
}

struct ContractProjection<'a>(&'a RenderContract);

impl Serialize for ContractProjection<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let c = self.0;
        let mut map = serializer.serialize_map(Some(15))?;
        map.serialize_entry("profile", &c.profile)?;
        map.serialize_entry("contract_version", &c.contract_version)?;
        map.serialize_entry("renderer_version", &c.renderer_version)?;
        map.serialize_entry("raster_policy", &c.raster_policy)?;
        map.serialize_entry("fonts", &Fonts(&c.fonts))?;
        map.serialize_entry("cell_width", &c.cell_width)?;
        map.serialize_entry("cell_height", &c.cell_height)?;
        map.serialize_entry("font_size", &c.font_size)?;
        map.serialize_entry("baseline", &c.baseline)?;
        map.serialize_entry("scale", &c.scale)?;
        map.serialize_entry("title_bar", &c.title_bar)?;
        map.serialize_entry("palette_digest", &c.palette_digest)?;
        map.serialize_entry("cursor_phase", &Phase(c.cursor_phase))?;
        map.serialize_entry("blink", &c.blink)?;
        map.serialize_entry("platform_key", &c.platform_key)?;
        map.end()
    }
}

struct Fonts<'a>(&'a [ContractFont]);

impl Serialize for Fonts<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for font in self.0 {
            seq.serialize_element(&Font(font))?;
        }
        seq.end()
    }
}

struct Font<'a>(&'a ContractFont);

impl Serialize for Font<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let font = self.0;
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("digest", &font.digest)?;
        map.serialize_entry("face_index", &font.face_index)?;
        map.serialize_entry("bold", &font.bold)?;
        map.serialize_entry("italic", &font.italic)?;
        map.serialize_entry("fallback_position", &font.fallback_position)?;
        map.end()
    }
}

struct Phase(CursorPhase);

impl Serialize for Phase {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self.0 {
            CursorPhase::Shown => "shown",
            CursorPhase::Hidden => "hidden",
        })
    }
}

#[cfg(test)]
mod tests;
