use super::super::super::super::*;
use tui_test::terminal::cell::Attrs;
use tui_test::{
    SnapshotCell, SnapshotColor, SnapshotCursor, SnapshotCursorShape, SnapshotMetadata,
    SnapshotParts, SnapshotScope, SnapshotSource, SnapshotUnderline, TerminalSnapshot,
};

pub(super) fn varint(mut value: u64) -> Vec<u8> {
    let mut result = Vec::new();
    while value >= 0x80 {
        result.push((value.to_le_bytes()[0] & 0x7f) | 0x80);
        value >>= 7;
    }
    result.push(value.to_le_bytes()[0]);
    result
}

pub(super) fn key(tag: u32, wire: u64) -> Vec<u8> {
    varint(u64::from(tag) * 8 + wire)
}

pub(super) fn vf(tag: u32, value: u64) -> Vec<u8> {
    let mut result = key(tag, 0);
    result.extend(varint(value));
    result
}

pub(super) fn bf(tag: u32, value: &[u8]) -> Vec<u8> {
    let mut result = key(tag, 2);
    result.extend(varint(value.len() as u64));
    result.extend_from_slice(value);
    result
}

pub(super) fn f32f(tag: u32, value: f32) -> Vec<u8> {
    let mut result = key(tag, 5);
    result.extend_from_slice(&value.to_bits().to_le_bytes());
    result
}

pub(super) fn f64f(tag: u32, value: f64) -> Vec<u8> {
    let mut result = key(tag, 1);
    result.extend_from_slice(&value.to_bits().to_le_bytes());
    result
}

pub(super) fn join(fields: impl IntoIterator<Item = Vec<u8>>) -> Vec<u8> {
    fields.into_iter().flatten().collect()
}

pub(super) fn nested(tag: u32, fields: impl IntoIterator<Item = Vec<u8>>) -> Vec<u8> {
    bf(tag, &join(fields))
}

pub(super) fn rgb(red: u8, green: u8, blue: u8) -> Vec<u8> {
    join([
        vf(1, u64::from(red)),
        vf(2, u64::from(green)),
        vf(3, u64::from(blue)),
    ])
}

pub(super) fn expected_color(color: SnapshotColor) -> Vec<u8> {
    match color {
        SnapshotColor::Default => nested(1, []),
        SnapshotColor::Indexed(index) => vf(2, u64::from(index)),
        SnapshotColor::Rgb(red, green, blue) => nested(3, [rgb(red, green, blue)]),
    }
}

pub(super) fn snapshot() -> TerminalSnapshot {
    let cell = SnapshotCell::new(
        "π".to_owned(),
        2,
        SnapshotColor::Indexed(17),
        SnapshotColor::Rgb(9, 8, 7),
        Attrs::BOLD
            | Attrs::DIM
            | Attrs::ITALIC
            | Attrs::INVERSE
            | Attrs::INVISIBLE
            | Attrs::STRIKE
            | Attrs::BLINK,
        SnapshotUnderline::Dashed,
        SnapshotColor::Default,
        Some("uri".to_owned()),
        Some("id".to_owned()),
    )
    .expect("fixture cell is valid");
    let palette =
        std::array::from_fn(|index| tui_test::profile::Rgb::new(index.to_le_bytes()[0], 2, 3));
    let cursor = SnapshotCursor::new(
        0,
        0,
        true,
        false,
        SnapshotCursorShape::Bar,
        tui_test::profile::Rgb::new(4, 5, 6),
        Some(false),
    );
    TerminalSnapshot::try_new(SnapshotParts {
        cols: 1,
        rows: 1,
        scope: SnapshotScope::FullHistory,
        viewport_origin: 0,
        cells: vec![cell],
        palette,
        default_foreground: tui_test::profile::Rgb::new(10, 11, 12),
        default_background: tui_test::profile::Rgb::new(13, 14, 15),
        default_cursor: tui_test::profile::Rgb::new(16, 17, 18),
        cursor,
        title: Some(String::new()),
        metadata: SnapshotMetadata {
            captured_at_ms: 20,
            revision: 21,
            backend: "backend".to_owned(),
            session: "session".to_owned(),
            source: SnapshotSource::Live,
        },
    })
    .expect("fixture snapshot is valid")
}

pub(super) fn state_only_sample() -> Sample {
    let cell = SnapshotCell::new(
        "x".to_owned(),
        1,
        SnapshotColor::Default,
        SnapshotColor::Default,
        Attrs::empty(),
        SnapshotUnderline::None,
        SnapshotColor::Default,
        None,
        None,
    )
    .expect("fixture cell is valid");
    let snapshot = TerminalSnapshot::try_new(SnapshotParts {
        cols: 1,
        rows: 1,
        scope: SnapshotScope::Viewport,
        viewport_origin: 0,
        cells: vec![cell],
        palette: [tui_test::profile::Rgb::new(0, 0, 0); 256],
        default_foreground: tui_test::profile::Rgb::new(0, 0, 0),
        default_background: tui_test::profile::Rgb::new(0, 0, 0),
        default_cursor: tui_test::profile::Rgb::new(0, 0, 0),
        cursor: SnapshotCursor::new(
            0,
            0,
            true,
            false,
            SnapshotCursorShape::Block,
            tui_test::profile::Rgb::new(0, 0, 0),
            None,
        ),
        title: None,
        metadata: SnapshotMetadata::default(),
    })
    .expect("fixture snapshot is valid");
    Sample {
        schema: SampleSchema,
        test_id: "suite/case".to_owned(),
        gate: Gate::State,
        snapshot,
        render_contract: None,
        image: None,
        observation: Observation::empty(),
    }
}

pub(super) fn top_level_tags(mut input: &[u8]) -> Vec<u64> {
    let mut tags = Vec::new();
    while !input.is_empty() {
        let mut key = 0u64;
        let mut shift = 0;
        loop {
            let byte = input[0];
            input = &input[1..];
            key |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        tags.push(key >> 3);
        match key & 7 {
            0 => {
                while input[0] & 0x80 != 0 {
                    input = &input[1..];
                }
                input = &input[1..];
            }
            1 => input = &input[8..],
            2 => {
                let mut length = 0u64;
                let mut shift = 0;
                loop {
                    let byte = input[0];
                    input = &input[1..];
                    length |= u64::from(byte & 0x7f) << shift;
                    if byte & 0x80 == 0 {
                        break;
                    }
                    shift += 7;
                }
                if let Ok(length) = usize::try_from(length) {
                    input = &input[length..];
                } else {
                    panic!("fixture length fits its message");
                }
            }
            5 => input = &input[4..],
            _ => panic!("test fixture uses only supported wire kinds"),
        }
    }
    tags
}
