use super::*;
use std::error::Error;
use std::io;

fn checked<T, E: std::fmt::Debug>(result: Result<T, E>) -> Result<T, Box<dyn Error>> {
    result.map_err(|error| io::Error::other(format!("{error:?}")).into())
}

#[test]
fn unsupported_logical_schema_reports_the_rejected_value_verbatim() -> Result<(), Box<dyn Error>> {
    let mut wire = Vec::new();
    bytes(1, b"tuiscotti.sample.v2", &mut wire);
    let mut budget = checked(DecodeBudget::new())?;

    let Err(error) = scan_sample(&wire, &mut budget) else {
        return Err(io::Error::other("unsupported logical schema unexpectedly scanned").into());
    };

    let DecodeError::UnsupportedSchema(value) = error else {
        return Err(
            io::Error::other(format!("unexpected logical schema result: {error:?}")).into(),
        );
    };
    assert_eq!(value, "tuiscotti.sample.v2");
    Ok(())
}

#[test]
fn unsupported_snapshot_schema_reports_its_version_payload_verbatim() -> Result<(), Box<dyn Error>>
{
    let mut snapshot = Vec::new();
    scalar(1, 2, &mut snapshot);
    let mut wire = Vec::new();
    bytes(4, &snapshot, &mut wire);
    let mut budget = checked(DecodeBudget::new())?;

    let Err(error) = scan_sample(&wire, &mut budget) else {
        return Err(io::Error::other("unsupported snapshot schema unexpectedly scanned").into());
    };

    let DecodeError::UnsupportedSchema(value) = error else {
        return Err(
            io::Error::other(format!("unexpected snapshot schema result: {error:?}")).into(),
        );
    };
    assert_eq!(value, "2");
    Ok(())
}

#[test]
fn maps_every_snapshot_component_through_upstream_validators() -> Result<(), Box<dyn Error>> {
    let bytes = valid_wire();
    let mut budget = checked(DecodeBudget::new())?;
    checked(budget.reserve_message_error_envelope(u64::try_from(bytes.len())?))?;
    checked(budget.set_phase(Phase::Codec))?;
    checked(budget.set_phase(Phase::Scan))?;
    let root = checked(scan_sample(&bytes, &mut budget))?;
    let sample = checked(map_sample(root, &mut budget))?;
    checked(budget.finish_output())?;

    assert_eq!(sample.test_id(), "suite/wide-cell");
    assert_eq!(sample.gate(), DomainGate::State);
    assert_eq!(sample.snapshot().cols(), 2);
    assert_eq!(sample.snapshot().rows(), 1);
    assert_eq!(sample.snapshot().cells()[0].text(), "界");
    assert_eq!(sample.snapshot().cells()[0].width(), 2);
    assert_eq!(sample.snapshot().cells()[0].fg(), SnapshotColor::Default);
    assert!(sample.snapshot().cells()[0].has(Attrs::BOLD));
    assert!(sample.snapshot().cells()[0].has(Attrs::ITALIC));
    assert_eq!(sample.snapshot().cells()[0].link_id(), Some("link-1"));
    assert_eq!(sample.snapshot().palette()[255], Rgb::new(12, 34, 56));
    assert_eq!(sample.snapshot().title(), Some("window"));
    assert_eq!(sample.snapshot().metadata().captured_at_ms, 1_234);
    assert_eq!(sample.snapshot().metadata().backend, "backend");
    assert_eq!(sample.snapshot().metadata().session, "session");
    assert_eq!(sample.snapshot().cursor().blink(), Some(false));
    assert!(sample.render_contract().is_none());
    assert!(sample.image().is_none());
    assert!(sample.observation().note().is_none());
    Ok(())
}

#[test]
fn maps_contract_fonts_and_exact_png_evidence() -> Result<(), Box<dyn Error>> {
    let (bytes, expected_png) = pixel_wire()?;
    let mut budget = checked(DecodeBudget::new())?;
    checked(budget.reserve_message_error_envelope(u64::try_from(bytes.len())?))?;
    checked(budget.set_phase(Phase::Codec))?;
    checked(budget.set_phase(Phase::Scan))?;
    let root = checked(scan_sample(&bytes, &mut budget))?;
    let sample = checked(map_sample(root, &mut budget))?;
    checked(budget.finish_output())?;

    let contract = sample
        .render_contract()
        .ok_or_else(|| io::Error::other("pixel fixture has no render contract"))?;
    assert_eq!(contract.profile(), "profile");
    assert_eq!(contract.renderer_version(), "renderer");
    assert_eq!(contract.raster_policy(), "pinned-v1");
    assert!(contract.cell_width().is_sign_negative());
    assert_eq!(contract.cell_height(), 20.0);
    assert_eq!(contract.font_size(), 12.5);
    assert_eq!(contract.baseline(), 15.0);
    assert_eq!(contract.scale(), 1.5);
    assert!(contract.title_bar());
    assert_eq!(contract.cursor_phase(), crate::sample::CursorPhase::Hidden);
    assert_eq!(contract.blink(), Some(false));
    assert_eq!(contract.fonts().len(), 1);
    assert_eq!(contract.fonts()[0].digest(), "a".repeat(64));
    assert_eq!(contract.fonts()[0].face_index(), 2);
    assert!(contract.fonts()[0].bold());
    assert!(contract.fonts()[0].italic());
    assert_eq!(contract.fonts()[0].fallback_position(), 3);
    assert_eq!(sample.observation().note(), Some("captured"));
    let image = sample
        .image()
        .ok_or_else(|| io::Error::other("pixel fixture has no image evidence"))?;
    assert_eq!((image.width(), image.height()), (20, 20));
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(image.png_base64())
            .map_err(io::Error::other)?,
        expected_png
    );
    Ok(())
}
