use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Advisory,
    Blocking,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub file: String,
    pub line: usize,
    pub rule: String,
    pub severity: Severity,
    pub message: String,
    pub source: &'static str,
}

#[derive(Debug, Default, Serialize)]
pub struct Outcome {
    pub findings: Vec<Finding>,
}

impl Outcome {
    pub fn exit_code(&self) -> i32 {
        if self
            .findings
            .iter()
            .any(|f| f.severity == Severity::Blocking)
        {
            2
        } else if self.findings.is_empty() {
            0
        } else {
            1
        }
    }
}
