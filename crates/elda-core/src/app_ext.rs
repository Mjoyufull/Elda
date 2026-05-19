use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::json;

use crate::app::AppContext;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};

#[derive(Debug, Clone, Deserialize)]
struct ExtensionDocument {
    name: String,
    kind: String,
    version: String,
    #[serde(default = "default_enabled")]
    enabled: bool,
    binary: PathBuf,
    #[serde(default)]
    capabilities: Vec<String>,
}

#[derive(Debug, Clone)]
struct ExtensionEntry {
    document: ExtensionDocument,
    config_path: PathBuf,
}

impl AppContext {
    pub(crate) fn handle_extension_list(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let extensions = list_extensions(&self.database.layout().extensions_dir)?;
        let enabled = extensions
            .iter()
            .filter(|entry| entry.document.enabled)
            .count();

        Ok(CommandReport {
            area: "extension",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "found {} configured extension(s), {} enabled.",
                extensions.len(),
                enabled
            ),
            details: Some(json!({
                "extensions_dir": self.database.layout().extensions_dir,
                "total": extensions.len(),
                "enabled": enabled,
                "extensions": extensions.iter().map(extension_json).collect::<Vec<_>>(),
            })),
        })
    }
}

fn list_extensions(extensions_dir: &Path) -> Result<Vec<ExtensionEntry>, CoreError> {
    if !extensions_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(extensions_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }
        let raw = fs::read_to_string(&path)?;
        let document = toml::from_str::<ExtensionDocument>(&raw).map_err(|error| {
            CoreError::Operator(format!(
                "extension document `{}` is invalid TOML or schema: {error}",
                path.display()
            ))
        })?;
        entries.push(ExtensionEntry {
            document,
            config_path: path,
        });
    }

    entries.sort_by(|left, right| left.document.name.cmp(&right.document.name));
    Ok(entries)
}

fn extension_json(entry: &ExtensionEntry) -> serde_json::Value {
    json!({
        "name": entry.document.name,
        "kind": entry.document.kind,
        "version": entry.document.version,
        "enabled": entry.document.enabled,
        "binary": entry.document.binary,
        "capabilities": entry.document.capabilities,
        "config_path": entry.config_path,
    })
}

fn default_enabled() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use crate::{CommandRequest, OutputMode, run_from_root};

    #[test]
    fn ext_ls_reports_configured_extension_documents() {
        let tempdir = TempDir::new().expect("tempdir should be created");
        let extensions_dir = tempdir.path().join("etc/elda/extensions.d");
        fs::create_dir_all(&extensions_dir).expect("extensions dir should be created");
        fs::write(
            extensions_dir.join("demo.toml"),
            r#"name = "demo-adapter"
kind = "interepo-adapter"
version = "0.1.0"
enabled = true
binary = "/usr/lib/elda/extensions/demo-adapter"
capabilities = ["interepo-resolve", "interepo-fetch"]
"#,
        )
        .expect("extension document should be written");

        let report = run_from_root(
            tempdir.path(),
            CommandRequest::new(
                vec!["ext".to_owned(), "ls".to_owned()],
                Vec::new(),
                OutputMode::Json,
                false,
            ),
        )
        .expect("ext ls should succeed");

        let details = report.details.expect("report should carry details");
        assert_eq!(details["total"], 1);
        assert_eq!(details["enabled"], 1);
        assert_eq!(details["extensions"][0]["name"], "demo-adapter");
        assert_eq!(
            details["extensions"][0]["capabilities"][0],
            "interepo-resolve"
        );
    }
}
