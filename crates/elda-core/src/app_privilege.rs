use std::error::Error;
use std::io;
use std::path::Path;

use crate::error::CoreError;
use crate::privilege::PrivilegeRequest;

use crate::app::AppContext;

impl AppContext {
    /// On a live host (`/`), Elda state lives under root-owned paths. Map access
    /// failures into a privilege re-exec request instead of a vague DB blocked report.
    pub(crate) fn elevate_permission_denied(&self, error: CoreError) -> CoreError {
        if !self.live_host_unprivileged || !error_denies_permission(&error) {
            return error;
        }
        CoreError::PrivilegeRequired(PrivilegeRequest::from_config(&self.config.privilege))
    }
}

fn error_denies_permission(error: &CoreError) -> bool {
    if core_error_io_is_permission_denied(error) {
        return true;
    }

    let mut source = error.source();
    while let Some(next) = source {
        if let Some(io_error) = next.downcast_ref::<io::Error>()
            && io_error.kind() == io::ErrorKind::PermissionDenied
        {
            return true;
        }
        source = next.source();
    }

    false
}

fn core_error_io_is_permission_denied(error: &CoreError) -> bool {
    match error {
        CoreError::Io(io_error) => io_error.kind() == io::ErrorKind::PermissionDenied,
        CoreError::Db(elda_db::DbError::Io(io_error)) => {
            io_error.kind() == io::ErrorKind::PermissionDenied
        }
        _ => false,
    }
}

#[must_use]
pub(crate) fn live_host_unprivileged(root_dir: &Path, is_superuser: bool) -> bool {
    root_dir == Path::new("/") && !is_superuser
}

#[cfg(test)]
mod tests {
    use std::io;

    use crate::app::AppContext;
    use crate::config::Config;
    use crate::error::CoreError;
    use elda_db::{Database, StateLayout};

    fn unprivileged_live_context() -> AppContext {
        let layout = StateLayout::new("/", "/usr");
        AppContext {
            config: Config::default(),
            database: Database::new(layout),
            progress: std::sync::Arc::new(crate::progress::NullSink),
            frame_counter: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            activation_needs_privilege: true,
            live_host_unprivileged: true,
        }
    }

    #[test]
    fn permission_denied_db_error_becomes_privilege_required_on_live_host() {
        let context = unprivileged_live_context();
        let error = CoreError::Db(elda_db::DbError::Io(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "mutation.lock",
        )));

        match context.elevate_permission_denied(error) {
            CoreError::PrivilegeRequired(_) => {}
            other => panic!("expected privilege re-exec, got {other:?}"),
        }
    }

    #[test]
    fn permission_denied_is_not_elevated_for_disposable_roots() {
        let layout = StateLayout::new("/tmp/elda-test", "/opt/elda");
        let context = AppContext {
            config: Config::default(),
            database: Database::new(layout),
            progress: std::sync::Arc::new(crate::progress::NullSink),
            frame_counter: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            activation_needs_privilege: false,
            live_host_unprivileged: false,
        };
        let error = CoreError::Db(elda_db::DbError::Io(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "db",
        )));

        assert!(matches!(
            context.elevate_permission_denied(error),
            CoreError::Db(_)
        ));
    }
}
