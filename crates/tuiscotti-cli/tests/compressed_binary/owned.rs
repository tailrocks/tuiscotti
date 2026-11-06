use super::*;

#[test]
fn owned_and_borrowed_checks_preserve_exact_evidence() {
    use tuiscotti::check::{CheckOutcome, FrozenStore};
    let temp = tempfile::tempdir().expect("temporary directory creates");
    let baselines = temp.path().join("baselines");
    let artifacts = temp.path().join("artifacts");
    std::fs::create_dir_all(&baselines).expect("baseline directory creates");
    let store = FrozenStore::new(&baselines, &artifacts).expect("store creates");
    let bytes = container(&valid_noncanonical_message());
    let mut allocated = Vec::with_capacity(bytes.len() + 4096);
    allocated.extend_from_slice(&bytes);
    for outcome in [store.check(&bytes), store.check_owned(allocated)] {
        let CheckOutcome::Failure(failure) = outcome.expect("check succeeds") else {
            panic!("missing baseline must fail comparison")
        };
        assert_eq!(failure.candidate_sha256(), sha256(&bytes));
        assert_eq!(
            std::fs::read(failure.candidate_path()).expect("evidence reads"),
            bytes
        );
    }
}

#[test]
fn sample_file_api_writes_and_reads_binary() {
    let temp = tempfile::tempdir().expect("temporary directory creates");
    let sample = Sample::from_bytes(&container(&valid_message())).expect("sample decodes");
    let path = temp.path().join("sample.tui.pb");
    sample.save(&path).expect("sample saves");
    let encoded = std::fs::read(&path).expect("sample file reads");
    assert_eq!(encoded, sample.to_bytes().expect("sample encodes"));
    assert!(encoded.starts_with(b"TUISCPB\0"));
    let loaded = Sample::from_file(&path).expect("sample loads");
    assert_eq!(
        loaded.state_digest().expect("digest"),
        sample.state_digest().expect("digest")
    );
}

#[test]
fn cli_classifies_image_corruption_color_and_dimensions() {
    use image::ImageEncoder as _;
    let temp = tempfile::tempdir().expect("temporary directory creates");
    let (baselines, artifacts) = dirs(temp.path());
    let candidate = temp.path().join("candidate.tui.pb");
    let mut rgb = Vec::new();
    image::codecs::png::PngEncoder::new(&mut rgb)
        .write_image(&[1, 2, 3], 1, 1, image::ExtendedColorType::Rgb8)
        .expect("RGB PNG encodes");
    let mut large = rgb.clone();
    large[16..20].copy_from_slice(&40_000_u32.to_be_bytes());
    let mut crc = !0_u32;
    for byte in &large[12..29] {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (0_u32.wrapping_sub(crc & 1)));
        }
    }
    large[29..33].copy_from_slice(&(!crc).to_be_bytes());
    for (name, png, width, code) in [
        ("corrupt", b"not a png".as_slice(), 1, "INVALID_IMAGE"),
        ("rgb", rgb.as_slice(), 1, "INVALID_IMAGE"),
        ("zero-claim", rgb.as_slice(), 0, "INVALID_IMAGE"),
        ("oversized", large.as_slice(), 20, "RESOURCE_LIMIT"),
    ] {
        write(
            &candidate,
            &container(&fixtures::message_with_png(png, width, 20)),
        );
        let output = run_cli(
            &[
                "check",
                "--sample",
                path(&candidate),
                "--baselines",
                path(&baselines),
                "--artifacts",
                path(&artifacts),
                "--json",
            ],
            Some("true"),
        );
        assert_eq!(output.status.code(), Some(3), "{name}");
        assert_eq!(
            result(&output)["error_code"],
            code,
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
