use std::collections::BTreeSet;

use super::super::super::model::Policy;

pub(super) fn validate_constraint_catalog(p: &Policy) -> Result<(), String> {
    let catalog: BTreeSet<String> = [
        "UTF8",
        "UINT32",
        "UINT64",
        "BOOL_0_1",
        "ENUM_NONZERO_KNOWN",
        "FLOAT32_BITS",
        "FLOAT64_BITS",
        "SAMPLE_SCHEMA_EXPECTED",
        "TEST_ID_GRAMMAR_240_64",
        "GATE_EVIDENCE_PAIR",
        "SNAPSHOT_SCHEMA_EQ_1",
        "GRID_DIM_1_32768",
        "TITLE_UTF8_65536",
        "GRID_PRODUCT_CELLS_EXACT_MAX_1000000",
        "PALETTE_COUNT_EXACT_256",
        "UPSTREAM_SNAPSHOT_INVARIANTS",
        "RGB_U8",
        "INDEX_U8",
        "CELL_TEXT_UTF8_4096",
        "CELL_WIDTH_U16",
        "LINK_UTF8_4096",
        "CONTRACT_VERSION_EXPECTED",
        "USIZE_CHECKED",
        "IMAGE_DIM_1_32768_RGBA_MAX_268435456",
        "IMAGE_PIXEL_DIMS_MATCH_GRID_10X20",
        "EMPTY_MESSAGE",
        "AGGREGATE_BUDGET",
        "ONEOF_EXACTLY_ONE",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let used: BTreeSet<_> = p
        .messages
        .values()
        .flat_map(|m| {
            m.fields
                .values()
                .flat_map(|f| f.constraints.iter().cloned())
                .chain(m.constraints.iter().cloned())
        })
        .collect();
    if used != p.constraints.keys().cloned().collect() {
        return Err("constraint catalog contains unused or missing entries".into());
    }
    if !used.is_subset(&catalog) {
        return Err("unknown constraint identifier".into());
    }
    if p.constraints
        .values()
        .any(|c| !["field", "message", "aggregate", "mapping"].contains(&c.phase.as_str()))
    {
        return Err("invalid constraint phase".into());
    }
    for m in p.messages.values() {
        if m.constraints
            .iter()
            .any(|id| p.constraints[id].phase == "field")
        {
            return Err("field constraint attached at message phase".into());
        }
        for f in m.fields.values() {
            if f.constraints
                .iter()
                .any(|id| p.constraints[id].phase != "field")
            {
                return Err("field constraint has non-field phase".into());
            }
        }
        if m.constraints.iter().any(|id| {
            !["message", "aggregate", "mapping"].contains(&p.constraints[id].phase.as_str())
        }) {
            return Err("message constraint has field phase".into());
        }
    }
    Ok(())
}

pub(super) fn validate_constraints(ids: &[String], p: &Policy) -> Result<(), String> {
    for id in ids {
        let constraint = p
            .constraints
            .get(id)
            .ok_or_else(|| format!("unknown constraint {id}"))?;
        if !matches!(
            constraint.phase.as_str(),
            "field" | "message" | "aggregate" | "mapping"
        ) {
            return Err(format!(
                "constraint {id} has unsupported phase {:?}",
                constraint.phase
            ));
        }
    }
    Ok(())
}
