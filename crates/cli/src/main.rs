#![forbid(unsafe_code)]
//! Headless CLI runner for ui-qa-tester.

fn main() {
    println!("ui-qa-cli v{}", ui_qa_core::workspace_version());
}
