use std::path::{Path, PathBuf};

use super::{IndexError, IndexOutcome, escape_html};

#[doc = "Walk the artifact root for check records, sort them, and write one index page linking each report.\n\n# Errors\n\nReturns [`IndexError`] when the artifact root cannot be walked or the index page cannot be written."]
pub fn write_report_index(artifacts: &Path, out: &Path) -> Result<IndexOutcome, IndexError> {
    let records = collect_records(artifacts).map_err(|source| IndexError::Read {
        artifacts: artifacts.to_path_buf(),
        source,
    })?;
    let page = render_index(artifacts, out, &records);
    std::fs::write(out, page.as_bytes()).map_err(|source| IndexError::Write {
        path: out.to_path_buf(),
        source,
    })?;
    Ok(IndexOutcome {
        records: records.len(),
    })
}

struct IndexedRecord {
    test_id: String,
    reason: String,
    candidate_sha256: String,
    baseline_sha256: String,
    check_dir: PathBuf,
}

fn collect_records(artifacts: &Path) -> Result<Vec<IndexedRecord>, std::io::Error> {
    let mut records = Vec::new();
    let mut stack = vec![artifacts.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut entries: Vec<std::fs::DirEntry> = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            entries.push(entry?);
        }
        entries.sort_by_key(std::fs::DirEntry::path);
        for entry in entries {
            let file_type = entry.file_type()?;
            let path = entry.path();
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file()
                && path.file_name().is_some_and(|name| name == "result.json")
                && let Some(record) = read_record(&path, artifacts)
            {
                records.push(record);
            }
        }
    }
    records.sort_by(|left, right| {
        (&left.test_id, &left.candidate_sha256, &left.baseline_sha256).cmp(&(
            &right.test_id,
            &right.candidate_sha256,
            &right.baseline_sha256,
        ))
    });
    Ok(records)
}

fn read_record(path: &Path, artifacts: &Path) -> Option<IndexedRecord> {
    let bytes = crate::sample::read_bounded(path).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    if value.get("schema")?.as_str()? != crate::check::CHECK_SCHEMA {
        return None;
    }
    let check_dir = path.parent()?.strip_prefix(artifacts).ok()?.to_path_buf();
    Some(IndexedRecord {
        test_id: value.get("test_id")?.as_str()?.to_string(),
        reason: value.get("reason")?.as_str()?.to_string(),
        candidate_sha256: value.get("candidate_sha256")?.as_str()?.to_string(),
        baseline_sha256: value.get("baseline_sha256")?.as_str()?.to_string(),
        check_dir,
    })
}

fn render_index(artifacts: &Path, out: &Path, records: &[IndexedRecord]) -> String {
    let parent = out.parent().filter(|parent| !parent.as_os_str().is_empty());
    let parent = parent.unwrap_or_else(|| Path::new("."));
    let mut page = String::from(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head><meta charset=\"utf-8\">\n<title>tuiscotti report</title>\n</head>\n<body>\n<h1>tuiscotti report</h1>\n",
    );
    push_index_line(&mut page, &format!("<p>records: {}</p>", records.len()));
    push_index_line(&mut page, "<table>");
    push_index_line(
        &mut page,
        "<tr><th>test</th><th>reason</th><th>candidate</th><th>baseline</th><th>evidence</th></tr>",
    );
    for record in records {
        let test_id = escape_html(&record.test_id);
        let reason = escape_html(&record.reason);
        let candidate = escape_html(&record.candidate_sha256);
        let baseline = escape_html(&record.baseline_sha256);
        let target = artifacts.join(&record.check_dir).join("report.html");
        let href = escape_html(&relative_href(parent, &target));
        push_index_line(
            &mut page,
            &format!(
                "<tr><td>{test_id}</td><td>{reason}</td><td>{candidate}</td><td>{baseline}</td><td><a href=\"{href}\">report</a></td></tr>"
            ),
        );
    }
    push_index_line(&mut page, "</table>");
    push_index_line(&mut page, "</body>");
    push_index_line(&mut page, "</html>");
    page
}

fn push_index_line(buffer: &mut String, line: &str) {
    buffer.push_str(line);
    buffer.push('\n');
}

fn relative_href(index_parent: &Path, target: &Path) -> String {
    let base = normalize_lexical(&absolutize(index_parent));
    let target = normalize_lexical(&absolutize(target));
    if let Some(relative) = relative_lexical(&base, &target) {
        return relative;
    }
    file_url(&target)
}

fn absolutize(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn normalize_lexical(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut stack: Vec<Component<'_>> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(stack.last(), Some(Component::Normal(_))) {
                    stack.pop();
                } else {
                    stack.push(component);
                }
            }
            _ => stack.push(component),
        }
    }
    let mut normalized = PathBuf::new();
    for component in stack {
        normalized.push(component);
    }
    normalized
}

fn relative_lexical(base: &Path, target: &Path) -> Option<String> {
    use std::path::Component;
    let base: Vec<Component<'_>> = base.components().collect();
    let target: Vec<Component<'_>> = target.components().collect();
    match (base.first(), target.first()) {
        (Some(Component::Prefix(_)), Some(Component::Prefix(_))) if base[0] != target[0] => {
            return None;
        }
        _ => {}
    }
    let common = base
        .iter()
        .zip(target.iter())
        .take_while(|(left, right)| left == right)
        .count();
    let mut parts: Vec<String> = Vec::new();
    for component in &base[common..] {
        if matches!(component, Component::Normal(_)) {
            parts.push("..".to_string());
        }
    }
    for component in &target[common..] {
        match component {
            Component::Normal(name) => parts.push(encode_href_segment(name.as_encoded_bytes())),
            Component::ParentDir => parts.push("..".to_string()),
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
        }
    }
    if parts.is_empty() {
        return Some("report.html".to_string());
    }
    Some(parts.join("/"))
}

fn encode_href_segment(raw: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(raw.len());
    for &byte in raw {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~'
            | b'!'
            | b'$'
            | b'&'
            | b'\''
            | b'('
            | b')'
            | b'*'
            | b'+'
            | b','
            | b';'
            | b'='
            | b'@' => out.push(byte as char),
            _ => {
                out.push('%');
                out.push(HEX[(byte >> 4) as usize] as char);
                out.push(HEX[(byte & 15) as usize] as char);
            }
        }
    }
    out
}

fn file_url(target: &Path) -> String {
    use std::path::{Component, Prefix};
    let mut authority = String::new();
    let mut segments: Vec<String> = Vec::new();
    for component in target.components() {
        match component {
            Component::Prefix(prefix) => match prefix.kind() {
                Prefix::Disk(drive) => segments.push(format!("{}:", drive as char)),
                Prefix::UNC(server, share) => {
                    authority.push_str(&server.to_string_lossy());
                    segments.push(encode_href_segment(share.as_encoded_bytes()));
                }
                _ => segments.push(encode_href_segment(prefix.as_os_str().as_encoded_bytes())),
            },
            Component::RootDir | Component::CurDir => {}
            Component::ParentDir => segments.push("..".to_string()),
            Component::Normal(name) => segments.push(encode_href_segment(name.as_encoded_bytes())),
        }
    }
    format!("file://{authority}/{}", segments.join("/"))
}
