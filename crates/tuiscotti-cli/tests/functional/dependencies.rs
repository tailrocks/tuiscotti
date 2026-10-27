use std::collections::HashMap;
use std::path::Path;

const FORBIDDEN: [&str; 3] = ["portable-pty", "alacritty_terminal", "termpane"];
const DEP_SECTIONS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

pub(crate) fn check_direct_dependencies(workspace: &Path, crate_manifest: &Path) -> i32 {
    match run(workspace, crate_manifest) {
        Ok(found) => i32::from(found),
        Err(()) => 2,
    }
}

fn run(workspace: &Path, crate_manifest: &Path) -> Result<bool, ()> {
    let ws_text = std::fs::read_to_string(workspace).map_err(|_| ())?;
    let crate_text = std::fs::read_to_string(crate_manifest).map_err(|_| ())?;
    let ws_table: toml::Table = ws_text.parse().map_err(|_| ())?;
    let crate_table: toml::Table = crate_text.parse().map_err(|_| ())?;
    let ws_names = workspace_names(&ws_table)?;
    let mut found = false;
    for name in direct_names(&crate_table, &ws_names)? {
        if FORBIDDEN.contains(&name.as_str()) {
            found = true;
        }
    }
    Ok(found)
}

fn workspace_names(ws: &toml::Table) -> Result<HashMap<String, String>, ()> {
    let mut names = HashMap::new();
    let Some(deps) = ws
        .get("workspace")
        .and_then(toml::Value::as_table)
        .and_then(|table| table.get("dependencies"))
    else {
        return Ok(names);
    };
    let table = deps.as_table().ok_or(())?;
    for (alias, spec) in table {
        names.insert(alias.clone(), workspace_identity(alias, spec)?);
    }
    Ok(names)
}

fn workspace_identity(alias: &str, spec: &toml::Value) -> Result<String, ()> {
    if spec.is_str() {
        return Ok(alias.to_string());
    }
    let table = spec.as_table().ok_or(())?;
    if let Some(package) = table.get("package") {
        return Ok(package.as_str().ok_or(())?.to_string());
    }
    Ok(alias.to_string())
}

fn direct_names(
    crate_table: &toml::Table,
    ws: &HashMap<String, String>,
) -> Result<Vec<String>, ()> {
    let mut names = Vec::new();
    collect_section_deps(crate_table, ws, &mut names)?;
    if let Some(target) = crate_table.get("target") {
        let targets = target.as_table().ok_or(())?;
        for cfg in targets.values() {
            collect_section_deps(cfg.as_table().ok_or(())?, ws, &mut names)?;
        }
    }
    Ok(names)
}

fn collect_section_deps(
    table: &toml::Table,
    ws: &HashMap<String, String>,
    out: &mut Vec<String>,
) -> Result<(), ()> {
    for section in DEP_SECTIONS {
        let Some(deps) = table.get(section) else {
            continue;
        };
        let dep_table = deps.as_table().ok_or(())?;
        for (alias, spec) in dep_table {
            out.push(resolve(alias, spec, ws)?);
        }
    }
    Ok(())
}

fn resolve(alias: &str, spec: &toml::Value, ws: &HashMap<String, String>) -> Result<String, ()> {
    if spec.is_str() {
        return Ok(alias.to_string());
    }
    let table = spec.as_table().ok_or(())?;
    if table.get("workspace").and_then(toml::Value::as_bool) == Some(true) {
        return ws.get(alias).cloned().ok_or(());
    }
    if let Some(package) = table.get("package") {
        return Ok(package.as_str().ok_or(())?.to_string());
    }
    Ok(alias.to_string())
}
