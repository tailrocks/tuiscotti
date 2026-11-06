#![doc = "End-to-end functional suite for the shipped tuiscotti binary."]

#[path = "functional/ci.rs"]
mod ci;
#[path = "functional/dependencies.rs"]
mod dependencies;
#[path = "functional/helpers.rs"]
mod helpers;
#[path = "functional/invalid_input.rs"]
mod invalid_input;
#[path = "functional/workflow.rs"]
mod workflow;

#[test]
fn ci_refuses_approval() {
    ci::ci_refuses_approval();
}

#[test]
fn closed_stdout() {
    ci::closed_stdout();
}

#[test]
fn invalid_input() {
    invalid_input::invalid_input();
}

#[test]
fn live_workflow() {
    workflow::live_workflow();
}
