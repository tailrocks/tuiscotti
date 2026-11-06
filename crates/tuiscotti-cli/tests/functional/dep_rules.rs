use std::path::{Path, PathBuf};

use crate::helpers::{check_direct_dependencies, make_dir, write_bytes};

const DEP_CRATE: &str =
    "[package]\nname = \"tuiscotti\"\nversion = \"0.2.0\"\nedition = \"2024\"\n";
const DEP_BARE_WS: &str = "[workspace]\n";
const DEP_WS_ALIAS: &str = "[workspace]\n[workspace.dependencies]\npty = { package = \"portable-pty\", version = \"0.9\" }\n";

fn dep_case(dir: &Path, seq: u32, workspace: Option<&str>, crate_toml: Option<&str>, want: i32) {
    let ws_path = dir.join(format!("workspace-{seq}.toml"));
    let crate_path = dir.join(format!("crate-{seq}.toml"));
    if let Some(text) = workspace {
        write_bytes(&ws_path, text.as_bytes());
    }
    if let Some(text) = crate_toml {
        write_bytes(&crate_path, text.as_bytes());
    }
    assert_eq!(check_direct_dependencies(&ws_path, &crate_path), want);
}

pub(super) fn phase_direct_dep_rules(candidates: &Path) {
    let dir = candidates.join("dep-cases");
    make_dir(&dir);
    let manifest_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert_eq!(
        check_direct_dependencies(
            &manifest_root.join("Cargo.toml"),
            &manifest_root.join("crates/tuiscotti/Cargo.toml")
        ),
        0
    );
    dep_rule_flat_tables(&dir);
    dep_rule_string_forms(&dir);
    dep_rule_dotted_tables(&dir);
    dep_rule_errors(&dir);
    phase_direct_dep_parser_forms(&dir);
    phase_direct_dep_type_errors(&dir);
}

fn phase_direct_dep_type_errors(dir: &Path) {
    let mut seq = 500;
    let mut case = |workspace: Option<&str>, crate_toml: Option<&str>, want: i32| {
        seq += 1;
        dep_case(dir, seq, workspace, crate_toml, want);
    };
    case(
        Some(DEP_WS_ALIAS),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\npty = {{ workspace = \"true\" }}\n"
        )),
        2,
    );
    case(Some("workspace = 7\n"), Some(DEP_CRATE), 2);
}

fn phase_direct_dep_parser_forms(dir: &Path) {
    let mut seq = 400;
    let mut case = |workspace: Option<&str>, crate_toml: Option<&str>, want: i32| {
        seq += 1;
        dep_case(dir, seq, workspace, crate_toml, want);
    };
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[target.'cfg(unix)']\ndependencies = {{ \"portable-pty\" = \"0.9\" }}\n"
        )),
        1,
    );
    case(
        Some(DEP_WS_ALIAS),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\npty.\"workspace\" = true\n"
        )),
        1,
    );
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\npty = {{ package = '''portable-pty''', version = \"0.9\" }}\n"
        )),
        1,
    );
    case(
        Some(DEP_BARE_WS),
        Some(
            "[package]\nname = \"tuiscotti\"\nname = \"tuiscotti-duplicate\"\nversion = \"0.2.0\"\nedition = \"2024\"\n[dependencies]\nok = \"1.0\"\n",
        ),
        2,
    );
}

fn dep_rule_flat_tables(dir: &Path) {
    let mut seq = 0;
    let mut case = |workspace: Option<&str>, crate_toml: Option<&str>, want: i32| {
        seq += 1;
        dep_case(dir, seq, workspace, crate_toml, want);
    };
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\nportable-pty = \"0.9\"\n"
        )),
        1,
    );
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\npty = {{ package = \"portable-pty\", version = \"0.9\" }}\n"
        )),
        1,
    );
    case(
        Some(DEP_WS_ALIAS),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\npty.workspace = true\n"
        )),
        1,
    );
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[build-dependencies]\npty = {{ package = \"portable-pty\", version = \"0.9\" }}\n"
        )),
        1,
    );
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[dev-dependencies]\nterm = {{ package = \"termpane\", version = \"0.1\" }}\n"
        )),
        1,
    );
    case(
        Some(DEP_WS_ALIAS),
        Some(&format!(
            "{DEP_CRATE}[target.'cfg(windows)'.dependencies]\npty.workspace = true\n"
        )),
        1,
    );
    case(
        Some(DEP_WS_ALIAS),
        Some(&format!(
            "{DEP_CRATE}[dependencies.ok]\nversion = \"1.0\"\n"
        )),
        0,
    );
    case(Some(DEP_WS_ALIAS), Some(DEP_CRATE), 0);
}

fn dep_rule_string_forms(dir: &Path) {
    let mut seq = 300;
    let mut case = |workspace: Option<&str>, crate_toml: Option<&str>, want: i32| {
        seq += 1;
        dep_case(dir, seq, workspace, crate_toml, want);
    };
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\npty = {{ package = 'portable-pty', version = '0.9' }}\n"
        )),
        1,
    );
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\n\"portable-pty\" = \"0.9\"\n"
        )),
        1,
    );
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\npty = {{ package = \"portable\\u002dpty\", path = \"../portable\" }}\n"
        )),
        1,
    );
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\n\"portable\\u002dpty\" = {{ path = \"../portable\" }}\n"
        )),
        1,
    );
}

fn dep_rule_dotted_tables(dir: &Path) {
    let mut seq = 100;
    let mut case = |workspace: Option<&str>, crate_toml: Option<&str>, want: i32| {
        seq += 1;
        dep_case(dir, seq, workspace, crate_toml, want);
    };
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[dependencies.pty]\npackage = \"portable-pty\"\nversion = \"0.9\"\n"
        )),
        1,
    );
    case(
        Some(DEP_BARE_WS),
        Some("dependencies.pty.package = \"portable-pty\"\ndependencies.pty.version = \"0.9\"\n"),
        1,
    );
    case(
        Some(DEP_BARE_WS),
        Some("dependencies.ok.version = \"1.0\"\n"),
        0,
    );
    case(
        Some("workspace.dependencies.pty = { package = \"portable-pty\", version = \"0.9\" }\n"),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\npty.workspace = true\n"
        )),
        1,
    );
    case(
        Some(
            "workspace.dependencies.pty.package = \"portable-pty\"\nworkspace.dependencies.pty.path = \"portable\"\n",
        ),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\npty.workspace = true\n"
        )),
        1,
    );
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "target = {{ 'cfg(unix)' = {{ dependencies = {{ \"portable-pty\" = {{ path = \"../portable\" }} }} }} }}\n{DEP_CRATE}"
        )),
        1,
    );
}

fn dep_rule_errors(dir: &Path) {
    let mut seq = 200;
    let mut case = |workspace: Option<&str>, crate_toml: Option<&str>, want: i32| {
        seq += 1;
        dep_case(dir, seq, workspace, crate_toml, want);
    };
    case(
        Some(DEP_BARE_WS),
        Some(&format!(
            "{DEP_CRATE}[dependencies]\npty.workspace = true\n"
        )),
        2,
    );
    case(Some(DEP_BARE_WS), Some("[[["), 2);
    let ws_path = dir.join("workspace-unreadable.toml");
    write_bytes(&ws_path, DEP_BARE_WS.as_bytes());
    assert_eq!(
        check_direct_dependencies(&ws_path, &dir.join("crate-absent.toml")),
        2
    );
}
