use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Policy {
    pub(super) version: u32,
    pub(super) package: String,
    pub(super) constraints: BTreeMap<String, Constraint>,
    pub(super) messages: BTreeMap<String, MessagePolicy>,
    pub(super) enums: BTreeMap<String, Vec<EnumValue>>,
    pub(super) retired: Vec<Retired>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Constraint {
    pub(super) phase: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MessagePolicy {
    pub(super) fields: BTreeMap<String, FieldPolicy>,
    pub(super) constraints: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FieldPolicy {
    pub(super) tag: i32,
    pub(super) kind: String,
    pub(super) cardinality: String,
    pub(super) presence: String,
    pub(super) oneof: Option<String>,
    pub(super) type_name: Option<String>,
    pub(super) handler: String,
    pub(super) constraints: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EnumValue {
    pub(super) number: i32,
    pub(super) name: String,
    pub(super) accepted: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Retired {
    pub(super) message: String,
    pub(super) name: String,
    pub(super) number: i32,
}
