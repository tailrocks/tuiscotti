use super::*;
use std::fmt::Debug;
use std::io;
use tui_test::{SnapshotCursorShape, SnapshotMetadata, SnapshotParts};

fn checked<T, E: Debug>(result: Result<T, E>) -> io::Result<T> {
    result.map_err(|error| io::Error::other(format!("{error:?}")))
}

fn required<T>(value: Option<T>, field: &str) -> io::Result<T> {
    value.ok_or_else(|| io::Error::other(format!("fixture field {field:?} is missing or invalid")))
}

fn rich_snapshot() -> io::Result<TerminalSnapshot> {
    use tui_test::terminal::cell::Attrs as A;
    use tui_test::{SnapshotColor as C, SnapshotScope as S};
    let cells = vec![
        checked(SnapshotCell::new(
            "A".into(),
            1,
            C::Indexed(17),
            C::Rgb(21, 22, 23),
            A::BOLD | A::DIM | A::ITALIC | A::INVERSE | A::INVISIBLE | A::STRIKE | A::BLINK,
            SnapshotUnderline::Double,
            C::Indexed(18),
            Some("https://example.test/oracle".into()),
            Some("link-a".into()),
        ))?,
        checked(SnapshotCell::new(
            " ".into(),
            1,
            C::Default,
            C::Default,
            A::empty(),
            SnapshotUnderline::None,
            C::Default,
            None,
            None,
        ))?,
    ];
    let palette = std::array::from_fn(|i| {
        let i = i.to_le_bytes()[0];
        tui_test::profile::Rgb::new(i, i.wrapping_mul(3), 255u8.wrapping_sub(i))
    });
    checked(TerminalSnapshot::try_new(SnapshotParts {
        cols: 2,
        rows: 1,
        scope: S::Viewport,
        viewport_origin: 0,
        cells,
        palette,
        default_foreground: tui_test::profile::Rgb::new(61, 62, 63),
        default_background: tui_test::profile::Rgb::new(81, 82, 83),
        default_cursor: tui_test::profile::Rgb::new(101, 102, 103),
        cursor: tui_test::SnapshotCursor::new(
            0,
            0,
            true,
            false,
            SnapshotCursorShape::Bar,
            tui_test::profile::Rgb::new(41, 42, 43),
            Some(false),
        ),
        title: Some("oracle title".into()),
        metadata: SnapshotMetadata::default(),
    }))
}

#[test]
fn snapshot_projection_matches_legacy_value_bytes() -> io::Result<()> {
    let snapshot = rich_snapshot()?;
    let mut old = checked(serde_json::to_value(&snapshot))?;
    required(old.as_object_mut(), "snapshot object")?.remove("metadata");
    let expected = checked(serde_json::to_vec(&old))?;
    let mut actual = Vec::new();
    checked(write_snapshot_projection(&snapshot, &mut actual))?;
    assert_eq!(actual, expected);
    if !snapshot_order_preserved() {
        assert_eq!(
            actual,
            include_bytes!("../../../tests/fixtures/rich_state_projection.canonical.json")
        );
    }
    Ok(())
}

#[derive(Serialize)]
struct OldFont<'a> {
    digest: &'a str,
    face_index: u32,
    bold: bool,
    italic: bool,
    fallback_position: usize,
}
#[derive(Serialize)]
struct OldContract<'a> {
    profile: &'a str,
    contract_version: u32,
    renderer_version: &'a str,
    raster_policy: &'a str,
    fonts: [OldFont<'a>; 1],
    cell_width: f32,
    cell_height: f32,
    font_size: f32,
    baseline: f32,
    scale: f64,
    title_bar: bool,
    palette_digest: &'a str,
    cursor_phase: &'a str,
    blink: Option<bool>,
    platform_key: &'a str,
}

#[test]
fn contract_projection_matches_legacy_declared_bytes_for_float_and_blink_cases() -> io::Result<()> {
    for (cell_width, blink) in [(0.0_f32, None), (-0.0_f32, Some(false)), (1.25, Some(true))] {
        let font = ContractFont {
            digest: "d".into(),
            face_index: 7,
            bold: true,
            italic: false,
            fallback_position: 3,
        };
        let c = RenderContract {
            profile: "p".into(),
            contract_version: 1,
            renderer_version: "r".into(),
            raster_policy: "pinned-v1".into(),
            fonts: vec![font],
            cell_width,
            cell_height: 2.5,
            font_size: 3.5,
            baseline: 4.5,
            scale: 1.125,
            title_bar: false,
            palette_digest: "palette".into(),
            cursor_phase: CursorPhase::Hidden,
            blink,
            platform_key: "x-y".into(),
        };
        let old = OldContract {
            profile: "p",
            contract_version: 1,
            renderer_version: "r",
            raster_policy: "pinned-v1",
            fonts: [OldFont {
                digest: "d",
                face_index: 7,
                bold: true,
                italic: false,
                fallback_position: 3,
            }],
            cell_width,
            cell_height: 2.5,
            font_size: 3.5,
            baseline: 4.5,
            scale: 1.125,
            title_bar: false,
            palette_digest: "palette",
            cursor_phase: "hidden",
            blink,
            platform_key: "x-y",
        };
        let expected = checked(serde_json::to_vec(&old))?;
        let mut actual = Vec::new();
        checked(write_contract_projection(&c, &mut actual))?;
        assert_eq!(actual, expected);
    }
    for fixture in [
        include_bytes!("../../../tests/fixtures/render_contract_none.canonical.json").as_slice(),
        include_bytes!("../../../tests/fixtures/render_contract_some_false.canonical.json")
            .as_slice(),
        include_bytes!("../../../tests/fixtures/render_contract_some_true.canonical.json")
            .as_slice(),
        include_bytes!("../../../tests/fixtures/render_contract_positive_zero.canonical.json")
            .as_slice(),
        include_bytes!("../../../tests/fixtures/render_contract_negative_zero.canonical.json")
            .as_slice(),
    ] {
        let contract = contract_from_fixture(fixture)?;
        let mut actual = Vec::new();
        checked(write_contract_projection(&contract, &mut actual))?;
        assert_eq!(actual, fixture);
    }
    Ok(())
}

fn contract_from_fixture(bytes: &[u8]) -> io::Result<RenderContract> {
    let value: serde_json::Value = checked(serde_json::from_slice(bytes))?;
    let text =
        |key: &str| -> io::Result<String> { Ok(required(value[key].as_str(), key)?.to_owned()) };
    let fonts = value["fonts"]
        .as_array()
        .ok_or_else(|| io::Error::other("fixture fonts field is not an array"))?
        .iter()
        .map(|font| -> io::Result<ContractFont> {
            Ok(ContractFont {
                digest: required(font["digest"].as_str(), "font.digest")?.to_owned(),
                face_index: u32::try_from(required(
                    font["face_index"].as_u64(),
                    "font.face_index",
                )?)
                .map_err(io::Error::other)?,
                bold: required(font["bold"].as_bool(), "font.bold")?,
                italic: required(font["italic"].as_bool(), "font.italic")?,
                fallback_position: usize::try_from(required(
                    font["fallback_position"].as_u64(),
                    "font.fallback_position",
                )?)
                .map_err(io::Error::other)?,
            })
        })
        .collect::<io::Result<Vec<_>>>()?;
    let blink = match &value["blink"] {
        serde_json::Value::Null => None,
        serde_json::Value::Bool(value) => Some(*value),
        _ => return Err(io::Error::other("fixture blink is not boolean or null")),
    };
    Ok(RenderContract {
        profile: text("profile")?,
        contract_version: u32::try_from(required(
            value["contract_version"].as_u64(),
            "contract_version",
        )?)
        .map_err(io::Error::other)?,
        renderer_version: text("renderer_version")?,
        raster_policy: text("raster_policy")?,
        fonts,
        cell_width: checked(
            required(value["cell_width"].as_f64(), "cell_width")?
                .to_string()
                .parse(),
        )?,
        cell_height: checked(
            required(value["cell_height"].as_f64(), "cell_height")?
                .to_string()
                .parse(),
        )?,
        font_size: checked(
            required(value["font_size"].as_f64(), "font_size")?
                .to_string()
                .parse(),
        )?,
        baseline: checked(
            required(value["baseline"].as_f64(), "baseline")?
                .to_string()
                .parse(),
        )?,
        scale: required(value["scale"].as_f64(), "scale")?,
        title_bar: required(value["title_bar"].as_bool(), "title_bar")?,
        palette_digest: text("palette_digest")?,
        cursor_phase: match required(value["cursor_phase"].as_str(), "cursor_phase")? {
            "shown" => CursorPhase::Shown,
            "hidden" => CursorPhase::Hidden,
            _ => return Err(io::Error::other("fixture cursor phase is unknown")),
        },
        blink,
        platform_key: text("platform_key")?,
    })
}

#[test]
fn order_probe_matches_enabled_map_behavior() {
    let mut map = serde_json::Map::new();
    map.insert("b".into(), serde_json::Value::Null);
    map.insert("a".into(), serde_json::Value::Null);
    assert_eq!(
        snapshot_order_preserved(),
        map.keys().next().is_some_and(|key| key == "b")
    );
}

#[test]
fn state_digest_hashes_decoded_pixels_and_checks_claimed_dimensions_first() -> io::Result<()> {
    use crate::sample::{
        Gate, ImageFormat, Observation, Sample, SampleError, SampleImage, SampleSchema,
    };
    use base64::Engine as _;
    use sha2::{Digest as _, Sha256};
    let rgba = [3_u8, 5, 7, 11];
    let mut png = Vec::new();
    let mut encoder = crate::sample::embedded_png::png::Encoder::new(&mut png, 1, 1);
    encoder.set_color(crate::sample::embedded_png::png::ColorType::Rgba);
    encoder.set_depth(crate::sample::embedded_png::png::BitDepth::Eight);
    let mut writer = checked(encoder.write_header())?;
    checked(writer.write_image_data(&rgba))?;
    drop(writer);
    let mut sample = Sample {
        schema: SampleSchema,
        test_id: "suite/test".into(),
        gate: Gate::StateAndPixels,
        snapshot: rich_snapshot()?,
        render_contract: None,
        image: Some(SampleImage {
            format: ImageFormat::Png,
            width: 1,
            height: 1,
            png_sha256: String::new(),
            rgba_sha256: String::new(),
            png_base64: base64::engine::general_purpose::STANDARD.encode(&png),
        }),
        observation: Observation::empty(),
    };
    let mut expected = Sha256::new();
    expected.update(b"tuiscotti-state-v1");
    expected.update([1]);
    let mut projection = Vec::new();
    checked(write_snapshot_projection(&sample.snapshot, &mut projection))?;
    expected.update(projection);
    expected.update(rgba);
    assert_eq!(
        checked(sample.state_digest())?,
        crate::sample::hash::hex_bytes(&expected.finalize())
    );
    let image = required(sample.image.as_mut(), "sample image")?;
    image.width = 0;
    image.png_base64 = "%%%".into();
    assert!(matches!(
        sample.state_digest(),
        Err(SampleError::ZeroImageDimensions)
    ));
    let image = required(sample.image.as_mut(), "sample image")?;
    image.width = 1;
    assert!(matches!(
        sample.state_digest(),
        Err(SampleError::Base64Invalid { .. })
    ));
    Ok(())
}
