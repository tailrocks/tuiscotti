use super::*;

#[test]
fn check_keeps_binary_candidate_evidence_and_never_writes_a_baseline() {
    let temp = tempfile::tempdir().expect("temporary directory creates");
    let (baselines, artifacts) = dirs(temp.path());
    let candidate = temp.path().join("candidate.tui.pb");
    let bytes = valid_container();
    write(&candidate, &bytes);
    let stale_json = baselines.join("binary/full-values.tui.json");
    std::fs::create_dir_all(stale_json.parent().expect("baseline has parent"))
        .expect("legacy JSON directory creates");
    write(&stale_json, br#"{"schema":"tuiscotti.sample.v1"}"#);

    let out = run_cli(
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
        Some(""),
    );
    let result = result(&out);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(result["status"], "missing");
    assert_eq!(result["candidate_sha256"], sha256(&bytes));
    assert!(
        walk_files(&baselines)
            .iter()
            .all(|file| file.extension().is_some_and(|extension| extension != "pb"))
    );
    assert_eq!(
        std::fs::read(stale_json).expect("legacy JSON remains untouched"),
        br#"{"schema":"tuiscotti.sample.v1"}"#
    );

    let candidate_evidence = walk_files(&artifacts)
        .into_iter()
        .find(|file| file.file_name().is_some_and(|name| name == "sample.tui.pb"))
        .expect("binary candidate evidence is linked from the report");
    assert_eq!(
        std::fs::read(candidate_evidence).expect("evidence reads"),
        bytes
    );
    let report = result["output_path"]["value"]
        .as_str()
        .expect("report path is present");
    assert!(Path::new(report).is_file());
    assert!(
        std::fs::read_to_string(report)
            .expect("report reads")
            .contains("sample.tui.pb")
    );
}

#[test]
fn accept_stores_exact_noncanonical_candidate_bytes_and_ci_refuses() {
    let temp = tempfile::tempdir().expect("temporary directory creates");
    let (baselines, _) = dirs(temp.path());
    let candidate = temp.path().join("candidate.tui.pb");
    let canonical = valid_message();
    let noncanonical = valid_noncanonical_message();
    assert_ne!(noncanonical, canonical);
    let bytes = container(&noncanonical);
    write(&candidate, &bytes);

    let owned_args = accept_args(&candidate, &baselines, &bytes);
    let args: Vec<&str> = owned_args.iter().map(String::as_str).collect();
    let refused = run_cli(&args, Some(""));
    let refused_result = result(&refused);
    assert_eq!(refused.status.code(), Some(3));
    assert_eq!(refused_result["error_code"], "CI_APPROVAL_FORBIDDEN");
    assert_eq!(walk_files(&baselines), [] as [PathBuf; 0]);

    let accepted = run_cli(&args, None);
    let accepted_result = result(&accepted);
    assert_eq!(accepted.status.code(), Some(0));
    assert_eq!(accepted_result["status"], "pass");
    let baseline = walk_files(&baselines)
        .into_iter()
        .find(|file| file.extension().is_some_and(|extension| extension == "pb"))
        .expect("binary baseline was installed");
    assert_eq!(std::fs::read(&baseline).expect("baseline reads"), bytes);

    let artifacts = temp.path().join("artifacts");
    std::fs::create_dir_all(&artifacts).expect("artifact directory creates");
    let checked = run_cli(
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
    let checked_result = result(&checked);
    assert_eq!(checked.status.code(), Some(0));
    assert_eq!(checked_result["status"], "pass");
    assert_eq!(checked_result["candidate_sha256"], sha256(&bytes));
    assert_eq!(checked_result["baseline_sha256"], sha256(&bytes));

    let json_candidate = temp.path().join("legacy.tui.json");
    let json_bytes = br#"{"schema":"tuiscotti.sample.v1"}"#;
    write(&json_candidate, json_bytes);
    let json_args = accept_args(&json_candidate, &baselines, json_bytes);
    let json_refs: Vec<&str> = json_args.iter().map(String::as_str).collect();
    let rejected_json = run_cli(&json_refs, None);
    let rejected_json_result = result(&rejected_json);
    assert_eq!(rejected_json.status.code(), Some(3));
    assert_eq!(rejected_json_result["error_code"], "INVALID_SAMPLE");
    assert_eq!(
        std::fs::read(baseline).expect("approved baseline remains exact"),
        bytes
    );
}

#[test]
fn concurrent_different_checks_keep_each_binary_candidate_and_report_link() {
    let temp = tempfile::tempdir().expect("temporary directory creates");
    let (baselines, artifacts) = dirs(temp.path());
    let baseline_path = temp.path().join("baseline.tui.pb");
    let baseline_bytes = container(&valid_message());
    write(&baseline_path, &baseline_bytes);
    let accept = accept_args(&baseline_path, &baselines, &baseline_bytes);
    let accept_refs: Vec<&str> = accept.iter().map(String::as_str).collect();
    let accepted = run_cli(&accept_refs, None);
    assert_eq!(accepted.status.code(), Some(0));

    let candidates = [
        container(&valid_message_with_text("candidate-a")),
        container(&valid_message_with_text("candidate-b")),
    ];
    let paths = [
        temp.path().join("candidate-a.tui.pb"),
        temp.path().join("candidate-b.tui.pb"),
    ];
    for (path, bytes) in paths.iter().zip(&candidates) {
        write(path, bytes);
    }

    let outputs = run_concurrent_checks(&paths, &baselines, &artifacts);

    let mut observed = Vec::new();
    for output in outputs {
        let result = result(&output);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(result["status"], "different");
        assert_eq!(result["reason"], "state");
        let report = result["output_path"]["value"]
            .as_str()
            .expect("report path exists");
        let html = std::fs::read_to_string(report).expect("concurrent report reads");
        assert!(html.contains("sample.tui.pb"));
        observed.push(
            result["candidate_sha256"]
                .as_str()
                .expect("candidate digest is reported")
                .to_string(),
        );
    }
    observed.sort();
    let mut expected = candidates
        .iter()
        .map(|bytes| sha256(bytes))
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(observed, expected);

    let saved = walk_files(&artifacts)
        .into_iter()
        .filter(|file| file.file_name().is_some_and(|name| name == "sample.tui.pb"))
        .map(|file| std::fs::read(file).expect("candidate evidence reads"))
        .collect::<Vec<_>>();
    assert_eq!(saved.len(), 2);
    assert!(candidates.iter().all(|candidate| saved.contains(candidate)));
    let baseline_files = walk_files(&baselines);
    assert_eq!(baseline_files.len(), 2);
    assert!(baseline_files.iter().any(|file| {
        file.file_name()
            .is_some_and(|name| name == ".tuiscotti-approval.lock")
    }));
    assert_eq!(
        baseline_files
            .iter()
            .filter(|file| file.extension().is_some_and(|ext| ext == "pb"))
            .count(),
        1
    );
}

fn run_concurrent_checks(
    paths: &[PathBuf; 2],
    baselines: &Path,
    artifacts: &Path,
) -> Vec<std::process::Output> {
    std::thread::scope(|scope| {
        let jobs: Vec<_> = paths
            .iter()
            .map(|candidate| {
                scope.spawn(|| {
                    let owned = [
                        "check".to_string(),
                        "--sample".to_string(),
                        candidate.to_str().expect("path is UTF-8").to_string(),
                        "--baselines".to_string(),
                        baselines.to_str().expect("path is UTF-8").to_string(),
                        "--artifacts".to_string(),
                        artifacts.to_str().expect("path is UTF-8").to_string(),
                        "--json".to_string(),
                    ];
                    let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
                    run_cli(&refs, Some("true"))
                })
            })
            .collect();
        jobs.into_iter()
            .map(|job| job.join().expect("check process returns"))
            .collect()
    })
}

#[test]
fn concurrent_first_approvals_serialize_on_the_stable_lock() {
    let temp = tempfile::tempdir().expect("temporary directory creates");
    let (baselines, _) = dirs(temp.path());
    let candidates = [
        container(&valid_message_with_text("first")),
        container(&valid_message_with_text("second")),
    ];
    let paths = [
        temp.path().join("first.tui.pb"),
        temp.path().join("second.tui.pb"),
    ];
    for (candidate, bytes) in paths.iter().zip(&candidates) {
        write(candidate, bytes);
    }

    let outputs = std::thread::scope(|scope| {
        let jobs = paths
            .iter()
            .zip(&candidates)
            .map(|(candidate, bytes)| {
                scope.spawn(|| {
                    let owned = accept_args(candidate, &baselines, bytes);
                    let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
                    run_cli(&refs, None)
                })
            })
            .collect::<Vec<_>>();
        jobs.into_iter()
            .map(|job| job.join().expect("approval process returns"))
            .collect::<Vec<_>>()
    });

    let mut accepted_digest = None;
    let mut stale_count = 0;
    for output in outputs {
        let result = result(&output);
        match output.status.code() {
            Some(0) => {
                assert_eq!(result["status"], "pass");
                accepted_digest = Some(
                    result["candidate_sha256"]
                        .as_str()
                        .expect("accepted candidate digest is reported")
                        .to_string(),
                );
            }
            Some(3) => {
                assert_eq!(result["error_code"], "STALE_BASELINE");
                stale_count += 1;
            }
            actual => assert_eq!(actual, Some(0), "unexpected accept exit code"),
        }
    }
    assert_eq!(stale_count, 1);

    let accepted_digest = accepted_digest.expect("one candidate wins the first approval");
    let baseline = walk_files(&baselines)
        .into_iter()
        .find(|file| file.extension().is_some_and(|extension| extension == "pb"))
        .expect("winning baseline is installed");
    let installed = std::fs::read(baseline).expect("installed baseline reads");
    assert_eq!(sha256(&installed), accepted_digest);
    assert!(candidates.contains(&installed));
}
