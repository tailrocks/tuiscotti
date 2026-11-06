use super::helpers::{
    ChildEnv, accept_candidate, baseline_samples, case_dir, exit_code, log_sample, make_dir,
    open_lane, parse_json_line, pixel_test_id, run_cli, sha256_bytes, sha256_file, utf8,
    walk_files, write_bytes,
};

pub(super) fn ci_refuses_approval() {
    let candidate_bytes = log_sample(&pixel_test_id("ci-refuses-approval"), 30, 0, 8);
    let digest = sha256_bytes(&candidate_bytes);
    for (name, vars, remove_ci, expect_code) in [
        ("unset", &[][..], true, 0),
        ("empty", &[("CI", "")][..], false, 3),
        ("false-value", &[("CI", "false")][..], false, 3),
        ("true-value", &[("CI", "true")][..], false, 3),
    ] {
        let root = case_dir(name);
        let baselines = root.path().join("baselines");
        make_dir(&baselines);
        let candidate = root.path().join("candidate.tui.pb");
        write_bytes(&candidate, &candidate_bytes);
        let env = ChildEnv {
            vars,
            remove_ci,
            close_stdout: false,
        };
        let (code, stdout) =
            accept_candidate(&candidate, digest.as_str(), "missing", &baselines, &env);
        assert_eq!(code, expect_code);
        if expect_code == 0 {
            assert_eq!(baseline_samples(&baselines).len(), 1);
        } else {
            assert_eq!(walk_files(&baselines), [] as [std::path::PathBuf; 0]);
            assert_eq!(
                parse_json_line(&stdout)["error_code"],
                "CI_APPROVAL_FORBIDDEN"
            );
        }
    }
}

pub(super) fn closed_stdout() {
    let root = case_dir("closed-stdout");
    let baselines = root.path().join("baselines");
    let artifacts = root.path().join("artifacts");
    let candidates = root.path().join("candidates");
    for dir in [&baselines, &artifacts, &candidates] {
        make_dir(dir);
    }
    let good = candidates.join("good.tui.pb");
    write_bytes(
        &good,
        &log_sample(&pixel_test_id("closed-stdout"), 30, 0, 8),
    );
    let digest = sha256_file(&good);
    let (code, _) = accept_candidate(&good, digest.as_str(), "missing", &baselines, &open_lane());
    assert_eq!(code, 0);
    let pass_env = ChildEnv {
        vars: &[("CI", "true")],
        remove_ci: false,
        close_stdout: true,
    };
    let out = run_cli(
        &[
            "check",
            "--sample",
            utf8(&good),
            "--baselines",
            utf8(&baselines),
            "--artifacts",
            utf8(&artifacts),
            "--json",
        ],
        &pass_env,
    );
    assert_eq!(exit_code(&out), 0);
    assert!(!String::from_utf8_lossy(&out.stderr).contains("panicked"));
    let absent = candidates.join("absent.tui.pb");
    let out = run_cli(
        &[
            "check",
            "--sample",
            utf8(&absent),
            "--baselines",
            utf8(&baselines),
            "--artifacts",
            utf8(&artifacts),
            "--json",
        ],
        &pass_env,
    );
    assert_eq!(exit_code(&out), 3);
    assert!(!String::from_utf8_lossy(&out.stderr).contains("panicked"));
}
