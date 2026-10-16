use super::*;
use image::codecs::png::{CompressionType, FilterType};
use tuiscotti::VENDORED_FACES;
use tuiscotti::snapshot::Status;

// ------ C08: transactional candidate generation / consistent approval
#[test]
fn c08_interrupted_candidate_must_report_incomplete_not_match() {
    if rerun_without_ci() {
        return;
    }
    // Case A (classic store): report RE-RENDERS from the actual frame
    // (`Store::report_with` → `check_with`), silently healing an interrupted
    // candidate (frame present, PNG write lost) back to Matched.
    let (_dir, st) = tmp_classic("c08").expect("tmp_classic succeeds");
    let frame = frame_with("atomic candidate");
    drop(
        st.check("home", &frame, &profile(), &VENDORED_FACES, 1.0)
            .expect("st .check(\"home\", &frame, &profile(), &VENDORED_FACES, 1.0) succeeds"),
    );
    st.accept("home").expect("st.accept(\"home\") succeeds");
    let matched = st
        .check("home", &frame, &profile(), &VENDORED_FACES, 1.0)
        .expect("st .check(\"home\", &frame, &profile(), &VENDORED_FACES, 1.0) succeeds");
    assert_eq!(matched.status, Status::Matched, "setup: must match first");

    // Simulate interruption: actual frame survived, actual PNG did not.
    let actual_png = matched.actual_png.clone();
    std::fs::remove_file(&actual_png).expect("std::fs::remove_file(&actual_png) succeeds");
    let sidecar = actual_png.with_extension("png.fidelity.json");
    drop(std::fs::remove_file(&sidecar));
    assert!(!actual_png.exists(), "setup: actual PNG must be gone");

    let mut renderer = tuiscotti::render::Renderer::new(&profile(), &VENDORED_FACES)
        .expect("tuiscotti::render::Renderer::new(&profile(), &VENDORED_FACES) succeeds");
    let report = st
        .report_with(&mut renderer, 1.0, "c08")
        .expect("st.report_with(&mut renderer, 1.0, \"c08\") succeeds");
    let reported = &report.outcomes[0];
    assert!(
        !reported.status.matched(),
        "C08 gap (classic): interrupted candidate (actual frame without PNG) \
         reports {:?}; an absent completion must be incomplete, never pass",
        reported.status
    );

    // Case B (grouped store): a missing actual PNG is misattributed as a
    // pixel difference (`disk_status` byte-compares), not flagged as
    // incomplete/missing evidence.
    let (_gdir, gst) = tmp_grouped("c08g").expect("tmp_grouped succeeds");
    gst.check("s", &frame, &profile(), &VENDORED_FACES, 1.0)
        .expect("gst.check(\"s\", &frame, &profile(), &VENDORED_FACES, 1.0) succeeds");
    gst.accept("s").expect("gst.accept(\"s\") succeeds");
    let gmatched = gst
        .check("s", &frame, &profile(), &VENDORED_FACES, 1.0)
        .expect("gst .check(\"s\", &frame, &profile(), &VENDORED_FACES, 1.0) succeeds");
    assert_eq!(
        gmatched.status(),
        Status::Matched,
        "setup: must match first"
    );
    std::fs::remove_file(&gmatched.actual.png)
        .expect("std::fs::remove_file(&gmatched.actual.png) succeeds");
    let mut grenderer = tuiscotti::render::Renderer::new(&profile(), &VENDORED_FACES)
        .expect("tuiscotti::render::Renderer::new(&profile(), &VENDORED_FACES) succeeds");
    let greport = gst
        .report_with(&mut grenderer, 1.0, "c08g")
        .expect("gst.report_with(&mut grenderer, 1.0, \"c08g\") succeeds");
    let greported = &greport.outcomes[0];
    assert_eq!(
        greported.status,
        Status::MissingApproval,
        "C08 gap (grouped): interrupted candidate (actual frame without PNG) \
         reports {:?} instead of missing/incomplete evidence",
        greported.status
    );
}

// ------ C02: actual evidence from candidate, never approved
#[test]
fn c02_actual_evidence_comes_from_candidate_never_approved() {
    if rerun_without_ci() {
        return;
    }
    let (_dir, gst) = tmp_grouped("c02").expect("tmp_grouped");
    let name = "c02/evidence";
    let frame = frame_with("candidate evidence");
    gst.check(name, &frame, &profile(), &VENDORED_FACES, 1.0)
        .expect("check");
    gst.accept(name).expect("accept");

    let approved_png_path = gst.approved_root().join(format!("{name}.png"));
    let approved_html_path = gst.approved_root().join(format!("{name}.html"));
    let approved_png_before = std::fs::read(&approved_png_path).expect("read png");
    let decoded_rgba = image::load_from_memory(&approved_png_before)
        .expect("decode png")
        .to_rgba8();
    let reencoded = encode_rgba(&decoded_rgba, CompressionType::Best, FilterType::NoFilter)
        .expect("encode_rgba");
    assert_ne!(approved_png_before, reencoded);
    assert_eq!(
        image::load_from_memory(&reencoded)
            .expect("decode")
            .to_rgba8()
            .as_raw(),
        decoded_rgba.as_raw(),
    );
    std::fs::write(&approved_png_path, &reencoded).expect("write png");
    let mut tampered_html = std::fs::read(&approved_html_path).expect("read html");
    tampered_html.extend_from_slice(b"\n<!-- c02 tamper -->\n");
    std::fs::write(&approved_html_path, &tampered_html).expect("write html");

    let mut renderer = profile().renderer(&VENDORED_FACES).expect("renderer");
    let fresh = renderer.render_artifacts(&frame, name).expect("render");
    let mut r = profile().renderer(&VENDORED_FACES).expect("renderer");
    let outcome = gst
        .check_with(&mut r, name, &frame, 1.0)
        .expect("check_with");
    assert_eq!(outcome.ansi_match, Some(true));
    assert_eq!(outcome.txt_match, Some(true));
    let actual_png = std::fs::read(&outcome.actual.png).expect("read actual png");
    let actual_html = std::fs::read(&outcome.actual.html).expect("read actual html");
    assert_eq!(actual_png, fresh.png);
    assert_eq!(actual_html, fresh.html.as_bytes());
    assert_ne!(actual_png, reencoded);
    assert_ne!(actual_html, tampered_html);
    assert_eq!(outcome.status(), Status::PixelsDiffer);
    assert_eq!(outcome.html_match, Some(false));
}
