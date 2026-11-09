use super::super::super::super::*;
use super::helpers::*;
use base64::Engine as _;
use tui_test::TerminalSnapshot;

pub(super) fn rich_sample() -> Sample {
    let png = [0, 1, 2, 255];
    let image = SampleImage {
        format: ImageFormat::Png,
        width: 1,
        height: 1,
        png_sha256: "png-digest".to_owned(),
        rgba_sha256: "rgba-digest".to_owned(),
        png_base64: base64::engine::general_purpose::STANDARD.encode(png),
    };
    let font = ContractFont {
        digest: "font-digest".to_owned(),
        face_index: 2,
        bold: true,
        italic: false,
        fallback_position: 3,
    };
    Sample {
        schema: SampleSchema,
        test_id: format!(
            "suite/case/pixels-{}",
            crate::render_profile::platform_key()
        ),
        gate: Gate::StateAndPixels,
        snapshot: snapshot(),
        render_contract: Some(RenderContract {
            profile: "profile".to_owned(),
            contract_version: 1,
            renderer_version: "renderer".to_owned(),
            raster_policy: "pinned".to_owned(),
            fonts: vec![font],
            cell_width: 0.0,
            cell_height: -0.0,
            font_size: 12.5,
            baseline: 8.25,
            scale: 1.5,
            title_bar: true,
            palette_digest: "palette".to_owned(),
            cursor_phase: CursorPhase::Hidden,
            blink: Some(false),
            platform_key: crate::render_profile::platform_key(),
        }),
        image: Some(image),
        observation: Observation::noted("note"),
    }
}

fn expected_snapshot(snap: &TerminalSnapshot) -> Vec<u8> {
    let cell = &snap.cells()[0];
    let cell_message = join([
        bf(1, cell.text().as_bytes()),
        vf(2, u64::from(cell.width())),
        nested(3, [expected_color(cell.fg())]),
        nested(4, [expected_color(cell.bg())]),
        vf(5, 1),
        vf(6, 1),
        vf(7, 1),
        vf(8, 1),
        vf(9, 1),
        vf(10, 1),
        vf(11, 1),
        vf(12, 6),
        nested(13, [expected_color(cell.underline_color())]),
        bf(14, b"uri"),
        bf(15, b"id"),
    ]);
    let cursor = snap.cursor();
    let cursor_message = join([
        vf(1, u64::from(cursor.col())),
        vf(2, u64::from(cursor.row())),
        vf(3, 1),
        vf(4, 0),
        vf(5, 3),
        nested(6, [rgb(4, 5, 6)]),
        vf(7, 0),
    ]);
    let metadata = snap.metadata();
    let metadata_message = join([
        vf(1, metadata.captured_at_ms),
        vf(2, metadata.revision),
        bf(3, b"backend"),
        bf(4, b"session"),
        vf(5, 1),
    ]);
    let snapshot_fields = std::iter::once(vf(1, 1))
        .chain([vf(2, 1), vf(3, 1), vf(4, 3), vf(5, 0)])
        .chain(std::iter::once(bf(6, &cell_message)))
        .chain(
            snap.palette()
                .iter()
                .map(|color| nested(7, [rgb(color.r, color.g, color.b)])),
        )
        .chain([
            nested(8, [rgb(10, 11, 12)]),
            nested(9, [rgb(13, 14, 15)]),
            nested(10, [rgb(16, 17, 18)]),
            bf(11, &cursor_message),
            bf(12, b""),
            bf(13, &metadata_message),
        ]);
    join(snapshot_fields)
}

fn expected_contract() -> Vec<u8> {
    let font_message = join([
        bf(1, b"font-digest"),
        vf(2, 2),
        vf(3, 1),
        vf(4, 0),
        vf(5, 3),
    ]);
    join([
        bf(1, b"profile"),
        vf(2, 1),
        bf(3, b"renderer"),
        bf(4, b"pinned"),
        bf(5, &font_message),
        f32f(6, 0.0),
        f32f(7, -0.0),
        f32f(8, 12.5),
        f32f(9, 8.25),
        f64f(10, 1.5),
        vf(11, 1),
        bf(12, b"palette"),
        vf(13, 2),
        vf(14, 0),
        bf(15, crate::render_profile::platform_key().as_bytes()),
    ])
}

fn expected_rich(sample: &Sample) -> Vec<u8> {
    join([
        bf(1, SAMPLE_SCHEMA.as_bytes()),
        bf(2, sample.test_id().as_bytes()),
        vf(3, 2),
        bf(4, &expected_snapshot(sample.snapshot())),
        bf(5, &expected_contract()),
        nested(
            6,
            [
                vf(1, 1),
                vf(2, 1),
                vf(3, 1),
                bf(4, b"png-digest"),
                bf(5, b"rgba-digest"),
                bf(6, &[0, 1, 2, 255]),
            ],
        ),
        nested(7, [bf(1, b"note")]),
    ])
}

#[test]
fn generated_writer_maps_every_domain_field_canonically() {
    let sample = rich_sample();
    let actual = super::super::encode_sample(&sample).expect("fixture maps completely");
    assert_eq!(actual, expected_rich(&sample));
    assert!(actual.windows(4).any(|window| window == [0, 1, 2, 255]));
}
