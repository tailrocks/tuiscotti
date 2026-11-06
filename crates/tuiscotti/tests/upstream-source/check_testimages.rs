use std::collections::BTreeMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader};
use std::path::{Component, Path, PathBuf};

use crc32fast::Hasher as Crc32;

const BASE_PATH: [&str; 2] = [".", "tests"];
const TEST_SUITES: [&str; 3] = ["pngsuite", "pngsuite-extra", "bugfixes"];
const APNG_SUITES: [&str; 1] = ["animated"];

fn process_images<F>(results_path: &str, test_suites: &[&str], func: F) -> io::Result<()>
where
    F: Fn(PathBuf) -> Result<u32, crate::sample::embedded_png::png::DecodingError>,
{
    let base: PathBuf = BASE_PATH.iter().collect();
    let mut results = BTreeMap::new();
    let mut expected_failures = vec![];
    for suite in test_suites {
        let mut path = base.clone();
        path.push(suite);
        path.push("*.png");

        let pattern = &*format!("{}", path.display());
        for path in glob::glob(pattern).map_err(io::Error::other)? {
            let path = path.map_err(io::Error::other)?;
            print!("{}: ", path.display());
            match func(path.clone()) {
                Ok(crc) => {
                    results.insert(format!("{}", path.display()), format!("{}", crc));
                    println!("{}", crc)
                }
                Err(_)
                    if path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with('x')) =>
                {
                    expected_failures.push(format!("{}", path.display()));
                    println!("Expected failure")
                }
                Err(error) => {
                    return Err(io::Error::other(format!("{}: {error:?}", path.display())));
                }
            }
        }
    }
    let mut path = base;
    path.push(results_path);
    let mut ref_results = BTreeMap::new();
    let mut failures = vec![];
    for line in BufReader::new(File::open(path)?).lines() {
        let line = line?;
        let mut parts = line.split(": ");
        let Some(result_path) = parts.next() else {
            return Err(io::Error::other(format!(
                "result file line is empty: {line}"
            )));
        };
        let Some(result) = parts.next() else {
            return Err(io::Error::other(format!(
                "result file line is malformed: {line}"
            )));
        };
        if parts.next().is_some() {
            return Err(io::Error::other(format!(
                "result file line has extra fields: {line}"
            )));
        }
        if result == "Expected failure" {
            failures.push(format!(
                "{}",
                normalize_path(Path::new(result_path)).display()
            ));
        } else {
            let current_path = format!("{}", normalize_path(Path::new(result_path)).display());
            ref_results.insert(current_path, result.to_string());
        }
    }
    assert_eq!(
        expected_failures.len(),
        failures.len(),
        "missing, extra: {:?}",
        {
            let mut same = vec![];
            for (idx, f) in expected_failures.iter().enumerate() {
                if let Some(inner) = failures.iter().position(|el| *el == *f) {
                    failures.remove(inner);
                    same.push(idx);
                }
            }
            for &idx in same.iter().rev() {
                expected_failures.remove(idx);
            }
            (failures, expected_failures)
        }
    );
    for (path, crc) in results.iter() {
        let Some(reference) = ref_results.get(path) else {
            return Err(io::Error::other(format!(
                "reference for {path} is missing, expected {crc}"
            )));
        };
        assert_eq!(reference, crc, "{}", path);
    }
    Ok(())
}

#[test]
fn render_images() -> io::Result<()> {
    process_images("results.txt", &TEST_SUITES, |path| {
        let mut decoder =
            crate::sample::embedded_png::png::Decoder::new(BufReader::new(File::open(path)?));
        decoder.set_transformations(
            crate::sample::embedded_png::png::Transformations::normalize_to_color8(),
        );
        let mut reader = decoder.read_info()?;
        let mut img_data = vec![
            0;
            reader.output_buffer_size()?.ok_or_else(
                || std::io::Error::other("decoded image size is unavailable")
            )?
        ];
        let info = reader.next_frame(&mut img_data)?;
        // First sanity check:
        assert_eq!(
            img_data.len(),
            info.width as usize
                * info.height as usize
                * info.color_type.samples()
                * info.bit_depth as usize
                / 8
        );
        let mut crc = Crc32::new();
        crc.update(&img_data);
        Ok(crc.finalize())
    })?;
    Ok(())
}

#[test]
fn render_images_identity() -> io::Result<()> {
    process_images("results_identity.txt", &TEST_SUITES, |path| {
        let decoder =
            crate::sample::embedded_png::png::Decoder::new(BufReader::new(File::open(&path)?));
        let mut reader = decoder.read_info()?;
        let mut img_data = vec![
            0;
            reader.output_buffer_size()?.ok_or_else(
                || std::io::Error::other("decoded image size is unavailable")
            )?
        ];
        let info = reader.next_frame(&mut img_data)?;
        let bits =
            ((info.width as usize * info.color_type.samples() * info.bit_depth as usize + 7) & !7)
                * info.height as usize;
        // First sanity check:
        assert_eq!(
            img_data.len() * 8,
            bits,
            "path: {} info: {:?} bits: {}",
            path.display(),
            info,
            bits
        );
        let mut crc = Crc32::new();
        crc.update(&img_data);
        Ok(crc.finalize())
    })?;
    Ok(())
}

#[test]
fn render_images_alpha() -> io::Result<()> {
    process_images("results_alpha.txt", &TEST_SUITES, |path| {
        let mut decoder =
            crate::sample::embedded_png::png::Decoder::new(BufReader::new(File::open(&path)?));
        decoder.set_transformations(crate::sample::embedded_png::png::Transformations::ALPHA);
        let mut reader = decoder.read_info()?;
        let mut img_data = vec![
            0;
            reader.output_buffer_size()?.ok_or_else(
                || std::io::Error::other("decoded image size is unavailable")
            )?
        ];
        let info = reader.next_frame(&mut img_data)?;
        let bits =
            ((info.width as usize * info.color_type.samples() * info.bit_depth as usize + 7) & !7)
                * info.height as usize;
        // First sanity check:
        assert_eq!(
            img_data.len() * 8,
            bits,
            "path: {} info: {:?} bits: {}",
            path.display(),
            info,
            bits
        );
        let mut crc = Crc32::new();
        crc.update(&img_data);
        Ok(crc.finalize())
    })?;
    Ok(())
}

#[test]
fn apng_images() -> io::Result<()> {
    process_images("results_apng.txt", &APNG_SUITES, |path: PathBuf| {
        let frame_count: usize = {
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| io::Error::other("test image should have a Unicode filename"))?;
            let count = stem
                .rsplit("_f")
                .next()
                .ok_or_else(|| io::Error::other("test image name should end with `_f0`"))?
                .parse()
                .map_err(io::Error::other)?;
            count
        };

        let mut decoder =
            crate::sample::embedded_png::png::Decoder::new(BufReader::new(File::open(&path)?));
        decoder.set_transformations(
            crate::sample::embedded_png::png::Transformations::normalize_to_color8(),
        );
        let mut reader = decoder.read_info()?;
        let mut img_data = vec![
            0;
            reader.output_buffer_size()?.ok_or_else(
                || std::io::Error::other("decoded image size is unavailable")
            )?
        ];
        let real_frames = reader
            .info()?
            .animation_control()
            .ok_or_else(|| io::Error::other("APNG image has no animation control"))?
            .num_frames;
        // Print out frame info, helps with blessing the result file for new images.
        println!(
            "file {}; images to decode: {}, per actl: {}",
            path.display(),
            frame_count,
            real_frames
        );

        let mut crc = Crc32::new();
        for _ in 0..frame_count {
            reader.next_frame(&mut img_data)?;
            crc.update(&img_data);
        }
        Ok(crc.finalize())
    })?;
    Ok(())
}

// until rust standardizes path normalization, see https://github.com/rust-lang/rfcs/issues/2208
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}
