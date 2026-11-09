use std::collections::{BTreeMap, BTreeSet};

use super::super::model::Policy;

pub(crate) fn validate_production_scan_depth(p: &Policy) -> Result<usize, String> {
    fn visit(
        name: &str,
        policy: &Policy,
        active: &mut BTreeSet<String>,
        depths: &mut BTreeMap<String, usize>,
    ) -> Result<usize, String> {
        if let Some(depth) = depths.get(name) {
            return Ok(*depth);
        }
        if !active.insert(name.to_owned()) {
            return Err(format!(
                "production message graph contains a cycle at {name}"
            ));
        }
        let message = policy
            .messages
            .get(name)
            .ok_or_else(|| format!("message graph references unknown type {name}"))?;
        let mut depth = 1usize;
        for field in message
            .fields
            .values()
            .filter(|field| field.kind == "message")
        {
            let child = field
                .type_name
                .as_deref()
                .ok_or_else(|| format!("message field in {name} lacks type name"))?;
            depth = depth.max(1 + visit(child, policy, active, depths)?);
        }
        active.remove(name);
        depths.insert(name.to_owned(), depth);
        Ok(depth)
    }

    let mut active = BTreeSet::new();
    let mut depths = BTreeMap::new();
    let mut maximum = 0usize;
    for name in p.messages.keys() {
        maximum = maximum.max(visit(name, p, &mut active, &mut depths)?);
    }
    if maximum > 32 {
        return Err(format!(
            "production message depth {maximum} exceeds the 32-frame scanner"
        ));
    }
    Ok(maximum)
}
