use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use sha2::Digest as _;
use tuiscotti::render_profile::{PROFILE_NAME, resolve_profile};
use tuiscotti::sample::{Observation, Sample};
use tuiscotti::view::{ViewSettings, capture_view};

pub(crate) static SPAWN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Debug)]
pub(crate) struct ChildEnv<'a> {
    pub(crate) vars: &'a [(&'a str, &'a str)],
    pub(crate) remove_ci: bool,
    pub(crate) close_stdout: bool,
}

#[expect(clippy::expect_used, reason = "test children must spawn")]
pub(crate) fn run_cli(args: &[&str], env: &ChildEnv<'_>) -> Output {
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_tuiscotti"));
    let mut cmd = Command::new(exe);
    cmd.args(args);
    if env.remove_ci {
        cmd.env_remove("CI");
    }
    for (key, value) in env.vars {
        cmd.env(key, value);
    }
    cmd.stderr(Stdio::piped());
    if env.close_stdout {
        cmd.stdout(Stdio::piped());
        let _guard = SPAWN_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut child = cmd.spawn().expect("CLI child spawns");
        drop(child.stdout.take());
        let mut stderr = Vec::new();
        if let Some(mut pipe) = child.stderr.take() {
            let _ignored = pipe.read_to_end(&mut stderr);
        }
        let status = child.wait().expect("CLI child waits");
        return Output {
            status,
            stdout: Vec::new(),
            stderr,
        };
    }
    cmd.stdout(Stdio::piped());
    let _guard = SPAWN_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let child = cmd.spawn().expect("CLI child spawns");
    child.wait_with_output().expect("CLI child waits")
}

pub(crate) fn ci_lane() -> ChildEnv<'static> {
    ChildEnv {
        vars: &[("CI", "true")],
        remove_ci: false,
        close_stdout: false,
    }
}

pub(crate) fn open_lane() -> ChildEnv<'static> {
    ChildEnv {
        vars: &[],
        remove_ci: true,
        close_stdout: false,
    }
}

pub(crate) fn check_direct_dependencies(workspace: &Path, crate_manifest: &Path) -> i32 {
    crate::dependencies::check_direct_dependencies(workspace, crate_manifest)
}

#[expect(clippy::expect_used, reason = "temp dirs anchor every case")]
pub(crate) fn case_dir(name: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("tuiscotti-functional-{name}-"))
        .tempdir()
        .expect("case temp dir builds")
}

#[expect(clippy::expect_used, reason = "sample bytes must persist")]
pub(crate) fn write_bytes(path: &Path, bytes: &[u8]) {
    std::fs::write(path, bytes).expect("case file writes");
}

pub(crate) fn sha256_file(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap_or_default();
    sha256_bytes(&bytes)
}

pub(crate) fn sha256_bytes(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    let mut out = String::with_capacity(64);
    for byte in hasher.finalize() {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}

#[expect(clippy::expect_used, reason = "temp paths are utf8")]
pub(crate) fn utf8(path: &Path) -> &str {
    path.to_str().expect("temp path is utf8")
}

#[expect(clippy::expect_used, reason = "temp dirs build")]
pub(crate) fn make_dir(path: &Path) {
    std::fs::create_dir_all(path).expect("temp dir builds");
}

#[expect(clippy::expect_used, reason = "test PNG decodes")]
pub(crate) fn decode_png(bytes: &[u8]) -> image::DynamicImage {
    image::load_from_memory(bytes).expect("png decodes")
}

pub(crate) fn pixel_test_id(name: &str) -> String {
    format!(
        "functional/{name}/pixels-{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

#[expect(clippy::expect_used, reason = "samples are the reviewed input")]
pub(crate) fn glyph_probe_sample(test_id: &str) -> Vec<u8> {
    let settings = ViewSettings::builtin();
    let snapshot = capture_view(30, 8, &settings, |frame| {
        let buffer = frame.buffer_mut();
        buffer[(0, 0)].set_char('🦀');
    })
    .expect("view captures");
    Sample::state(test_id, snapshot, Observation::empty())
        .expect("state sample builds without rendering")
        .to_bytes()
        .expect("sample encodes")
}

#[expect(clippy::expect_used, reason = "samples are the reviewed input")]
pub(crate) fn log_sample(test_id: &str, width: u16, first_line: u32, rows: u16) -> Vec<u8> {
    let settings = ViewSettings::builtin();
    let snapshot = capture_view(width, rows, &settings, |frame| {
        let buffer = frame.buffer_mut();
        let mut y: u16 = 0;
        let mut line = first_line;
        while y < rows {
            let text = format!("log line {line:04} ok");
            for (x, symbol) in (0_u16..).zip(text.chars()) {
                if x >= width {
                    break;
                }
                buffer[(x, y)].set_char(symbol);
            }
            y += 1;
            line += 1;
        }
    })
    .expect("view captures");
    let profile = resolve_profile(PROFILE_NAME).expect("pinned profile resolves");
    Sample::with_pixels(test_id, &snapshot, Observation::empty(), profile)
        .expect("sample builds")
        .to_bytes()
        .expect("sample encodes")
}

#[expect(clippy::expect_used, reason = "JSON shape is the assertion")]
pub(crate) fn parse_json_line(stdout: &[u8]) -> serde_json::Value {
    let text = String::from_utf8_lossy(stdout);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 1, "stdout carries one JSON line");
    serde_json::from_str(lines[0]).expect("stdout parses as JSON")
}

#[expect(clippy::expect_used, reason = "JSON shape is the assertion")]
pub(crate) fn report_file_name(body: &serde_json::Value) -> String {
    let value = body
        .get("output_path")
        .and_then(|path| path.get("value"))
        .and_then(serde_json::Value::as_str)
        .expect("report path present");
    Path::new(value)
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .expect("report file name")
        .to_string()
}

pub(crate) fn exit_code(out: &Output) -> i32 {
    out.status.code().unwrap_or(-1)
}

pub(crate) fn baseline_samples(root: &Path) -> Vec<PathBuf> {
    walk_files(root)
        .into_iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect()
}

pub(crate) fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                found.push(path);
            }
        }
    }
    found
}
