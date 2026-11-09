#![allow(
    missing_docs,
    reason = "the isolated schema generator is a non-shipping build tool"
)]

mod accessors;
mod codegen;
mod driver;
mod model;
mod policy;
mod render;
mod scanner;
#[cfg(test)]
mod tests;
mod writer;

const SCHEMA_DIR: &str = "../tuiscotti/src/sample/wire/schema";
const PROTO: &str = "tuiscotti-v1.proto";
const DESC: &str = "tuiscotti-v1.desc";
const POLICY: &str = "tuiscotti-v1.policy.toml";
const GENERATED: &str = "../tuiscotti/src/sample/wire/generated.rs";
const GENERATED_DIR: &str = "../tuiscotti/src/sample/wire/generated";

fn main() {
    if let Err(error) = driver::run() {
        eprintln!("schema-gen: {error}");
        std::process::exit(1);
    }
}

