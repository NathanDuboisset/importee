use serde::Serialize;

#[derive(Serialize, Debug, Default)]
pub struct CheckResult {
    pub issues: Vec<Issue>,
}

#[derive(Serialize, Debug)]
pub struct Issue {
    pub rule_name: String,
    pub path: String,
    pub line: u32,
    pub message: String,
}

impl CheckResult {
    pub fn new() -> Self {
        Self { issues: Vec::new() }
    }
}
