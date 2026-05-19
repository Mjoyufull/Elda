use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use rustix::process::geteuid;
use serde_json::{Value, json};

use crate::config::Config;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest};

#[derive(Debug, Clone)]
pub(crate) struct CommandLogSession {
    path: PathBuf,
    level: u8,
    started_at_secs: u64,
    started_at_nanos: u32,
}

impl CommandLogSession {
    pub(crate) fn start(
        root_dir: &Path,
        config: &Config,
        request: &CommandRequest,
    ) -> Result<Option<Self>, CoreError> {
        if !is_mutating_command(&request.command_path) {
            return Ok(None);
        }

        let level = request.log_level.unwrap_or(config.logging.level);
        if level == 0 {
            return Ok(None);
        }
        if !(1..=3).contains(&level) {
            return Err(CoreError::Operator(format!(
                "invalid log level `{level}`; expected 0, 1, 2, or 3"
            )));
        }

        let directory = resolve_log_dir(root_dir, &config.logging.dir);
        fs::create_dir_all(&directory)?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| CoreError::Operator(error.to_string()))?;
        let file_name = format!(
            "{}.{}-pid{}-{}.log",
            now.as_secs(),
            now.subsec_nanos(),
            process::id(),
            command_slug(&request.command_path),
        );
        let path = directory.join(file_name);

        Ok(Some(Self {
            path,
            level,
            started_at_secs: now.as_secs(),
            started_at_nanos: now.subsec_nanos(),
        }))
    }

    pub(crate) fn attach_to_report(&self, report: &mut CommandReport) {
        let session_log = json!({
            "path": self.path,
            "level": self.level,
        });

        match report.details.take() {
            Some(Value::Object(mut object)) => {
                object.insert("session_log".to_owned(), session_log);
                report.details = Some(Value::Object(object));
            }
            Some(other) => {
                report.details = Some(json!({
                    "session_log": session_log,
                    "report": other,
                }));
            }
            None => {
                report.details = Some(json!({
                    "session_log": session_log,
                }));
            }
        }
    }

    pub(crate) fn write_success(
        &self,
        root_dir: &Path,
        config: &Config,
        request: &CommandRequest,
        report: &CommandReport,
    ) -> Result<(), CoreError> {
        let mut lines = self.base_lines(root_dir, config, request);
        lines.push("result = success".to_owned());
        lines.push(format!("area = {}", report.area));
        lines.push(format!("status = {}", report.status));
        lines.push(format!("summary = {}", report.summary));

        if self.level >= 2 {
            lines.push(String::new());
            lines.push("[report]".to_owned());
            lines.push(serde_json::to_string_pretty(report)?);
        }
        if self.level >= 3 {
            lines.push(String::new());
            lines.push("[config]".to_owned());
            lines.push(serde_json::to_string_pretty(&json!({
                "prefix": config.defaults.prefix,
                "default_remote": config.defaults.remote,
                "build_mode": config.defaults.build_mode,
                "snapshot_tool": config.defaults.snapshot_tool,
            }))?);
        }

        fs::write(&self.path, lines.join("\n"))?;
        Ok(())
    }

    pub(crate) fn write_error(
        &self,
        root_dir: &Path,
        config: &Config,
        request: &CommandRequest,
        error: &CoreError,
    ) -> Result<(), CoreError> {
        let mut lines = self.base_lines(root_dir, config, request);
        lines.push("result = error".to_owned());
        lines.push(format!("error = {error}"));

        let mut chain = error.source();
        let mut index = 0;
        while let Some(cause) = chain {
            lines.push(format!("cause.{index} = {cause}"));
            chain = cause.source();
            index += 1;
        }

        if self.level >= 2 {
            lines.push(String::new());
            lines.push("[request]".to_owned());
            lines.push(serde_json::to_string_pretty(&json!({
                "command_path": request.command_path,
                "operands": request.operands,
                "dry_run": request.dry_run,
                "offline": request.offline,
                "system_mode": request.system_mode,
                "output_mode": request.output_mode,
            }))?);
        }
        if self.level >= 3 {
            lines.push(String::new());
            lines.push("[config]".to_owned());
            lines.push(serde_json::to_string_pretty(&json!({
                "prefix": config.defaults.prefix,
                "default_remote": config.defaults.remote,
                "build_mode": config.defaults.build_mode,
                "snapshot_tool": config.defaults.snapshot_tool,
                "logging_dir": config.logging.dir,
            }))?);
        }

        fs::write(&self.path, lines.join("\n"))?;
        Ok(())
    }

    fn base_lines(
        &self,
        root_dir: &Path,
        config: &Config,
        request: &CommandRequest,
    ) -> Vec<String> {
        vec![
            format!("started_at_secs = {}", self.started_at_secs),
            format!("started_at_nanos = {}", self.started_at_nanos),
            format!("pid = {}", process::id()),
            format!("log_level = {}", self.level),
            format!("root_dir = {}", root_dir.display()),
            format!("prefix = {}", config.defaults.prefix.display()),
            format!("command_path = {}", request.command_path.join(" ")),
            format!("operands = {:?}", request.operands),
            format!("dry_run = {}", request.dry_run),
            format!("offline = {}", request.offline),
            format!("system_mode = {}", request.system_mode),
            format!("output_mode = {:?}", request.output_mode),
        ]
    }
}

pub(crate) fn session_log_path(details: &Value) -> Option<&str> {
    details.get("session_log")?.get("path")?.as_str()
}

fn is_mutating_command(command_path: &[String]) -> bool {
    match command_path {
        [command] => matches!(
            command.as_str(),
            "i" | "ig"
                | "ib"
                | "rm"
                | "u"
                | "sync"
                | "pin"
                | "unpin"
                | "hold"
                | "unhold"
                | "downgrade"
                | "recover"
                | "rollback"
                | "fix-triggers"
                | "autoremove"
        ),
        [namespace, command] if namespace == "rmt" => command == "add",
        [namespace, command] if namespace == "rc" => {
            matches!(command.as_str(), "add" | "edit" | "rm")
        }
        [namespace, command] if namespace == "vendor" => {
            matches!(command.as_str(), "add" | "import" | "export")
        }
        [namespace, command] if namespace == "cache" => command == "add",
        [namespace, command] if namespace == "pf" => matches!(
            command.as_str(),
            "apply"
                | "add"
                | "rm"
                | "set-init"
                | "clear-init"
                | "set-arch"
                | "add-foreign-arch"
                | "remove-foreign-arch"
        ),
        [namespace, command] if namespace == "state" => command == "import",
        [namespace, command] if namespace == "daemon" => {
            matches!(command.as_str(), "run" | "refresh")
        }
        [namespace, command] if namespace == "mg" => {
            matches!(command.as_str(), "from" | "lock" | "unlock")
        }
        [namespace, command] if namespace == "ci" => {
            matches!(command.as_str(), "sub" | "run" | "retry")
        }
        [namespace, command, subcommand] if namespace == "ci" && command == "batch" => {
            matches!(subcommand.as_str(), "new" | "add" | "push")
        }
        _ => false,
    }
}

fn resolve_log_dir(root_dir: &Path, configured_dir: &str) -> PathBuf {
    if root_dir != Path::new("/") {
        return root_relative_path(root_dir, configured_dir);
    }

    if let Some(stripped) = configured_dir.strip_prefix("~/")
        && let Some(config_home) = user_config_home()
    {
        return config_home.join(stripped.strip_prefix(".config/").unwrap_or(stripped));
    }

    PathBuf::from(configured_dir)
}

fn root_relative_path(root_dir: &Path, configured_dir: &str) -> PathBuf {
    if let Some(stripped) = configured_dir.strip_prefix("~/") {
        return root_dir.join(stripped);
    }

    let configured_path = Path::new(configured_dir);
    if configured_path.is_absolute() {
        return root_dir.join(configured_path.strip_prefix("/").unwrap_or(configured_path));
    }

    root_dir.join(configured_path)
}

fn user_config_home() -> Option<PathBuf> {
    if geteuid().is_root() {
        if let Some(uid) = env::var_os("SUDO_UID")
            .and_then(|value| value.to_str().and_then(|text| text.parse::<u32>().ok()))
            && let Some(home) = home_dir_for_uid(uid)
        {
            return Some(home.join(".config"));
        }
        if let Ok(user) = env::var("SUDO_USER")
            && user != "root"
            && let Some(home) = home_dir_for_username(&user)
        {
            return Some(home.join(".config"));
        }
        if let Ok(user) = env::var("DOAS_USER")
            && user != "root"
            && let Some(home) = home_dir_for_username(&user)
        {
            return Some(home.join(".config"));
        }
    }

    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
}

fn home_dir_for_uid(uid: u32) -> Option<PathBuf> {
    let passwd = fs::read_to_string("/etc/passwd").ok()?;
    passwd.lines().find_map(|line| {
        let mut parts = line.split(':');
        let _name = parts.next()?;
        let _password = parts.next()?;
        let uid_field = parts.next()?;
        if uid_field.parse::<u32>().ok()? != uid {
            return None;
        }
        for _ in 0..2 {
            parts.next()?;
        }
        let home = parts.next()?;
        Some(PathBuf::from(home))
    })
}

fn home_dir_for_username(name: &str) -> Option<PathBuf> {
    let passwd = fs::read_to_string("/etc/passwd").ok()?;
    passwd.lines().find_map(|line| {
        let mut parts = line.split(':');
        let user = parts.next()?;
        if user != name {
            return None;
        }
        let _password = parts.next()?;
        let _uid = parts.next()?;
        let _gid = parts.next()?;
        let _gecos = parts.next()?;
        let home = parts.next()?;
        Some(PathBuf::from(home))
    })
}

fn command_slug(command_path: &[String]) -> String {
    let slug = command_path.join("-");
    if slug.is_empty() {
        "command".to_owned()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::json;

    use crate::config::Config;
    use crate::{CommandRequest, OutputMode};

    use super::{CommandLogSession, command_slug, resolve_log_dir, session_log_path};

    #[test]
    fn resolve_log_dir_uses_root_relative_path_for_isolated_roots() {
        let directory = resolve_log_dir(Path::new("/tmp/elda-root"), "~/.config/elda/logs");

        assert_eq!(directory, Path::new("/tmp/elda-root/.config/elda/logs"));
    }

    #[test]
    fn command_log_session_skips_file_when_level_zero() {
        let config = Config::default();
        assert_eq!(config.logging.level, 0);
        let request = CommandRequest::new(
            vec!["sync".to_owned()],
            Vec::new(),
            OutputMode::Human,
            false,
        );
        let session =
            CommandLogSession::start(Path::new("/"), &config, &request).expect("session start");
        assert!(session.is_none());
    }

    #[test]
    fn session_log_path_reads_attached_path() {
        let details = json!({
            "session_log": {
                "path": "/tmp/logs/example.log",
                "level": 2,
            },
        });

        assert_eq!(session_log_path(&details), Some("/tmp/logs/example.log"));
    }

    #[test]
    fn command_slug_joins_segments() {
        assert_eq!(command_slug(&["ci".to_owned(), "run".to_owned()]), "ci-run");
    }
}
