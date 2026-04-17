use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExitStatus {
    Success,
    OperatorFailure,
    ResolutionFailure,
    TrustFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputMode {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CrateBoundary {
    pub name: &'static str,
    pub responsibility: &'static str,
}

impl CrateBoundary {
    pub const fn new(name: &'static str, responsibility: &'static str) -> Self {
        Self {
            name,
            responsibility,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct NamespaceSpec {
    pub name: &'static str,
    pub commands: &'static [&'static str],
}

impl NamespaceSpec {
    pub const fn new(name: &'static str, commands: &'static [&'static str]) -> Self {
        Self { name, commands }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandReport {
    pub area: &'static str,
    pub status: &'static str,
    pub exit_status: ExitStatus,
    pub command_path: Vec<String>,
    pub operands: Vec<String>,
    pub output_mode: OutputMode,
    pub dry_run: bool,
    pub summary: String,
    pub details: Option<Value>,
}
