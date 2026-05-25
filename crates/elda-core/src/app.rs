use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use crate::app_render_tree::{TreeStyle, set_configured_tree_style};
use crate::config::{Config, InstallPreference};
use crate::error::CoreError;
use crate::flags::ResolvedFlagState;
use crate::privilege::{PrivilegeConfig, PrivilegeRequest, PrivilegeStatus};
use crate::progress::{NullSink, ProgressSink};
use crate::progress_live::{LiveSink, LiveSinkMode};
use crate::run_log::CommandLogSession;
use crate::{CommandReport, CommandRequest};
use elda_build::{BinarySourceVerification, BuiltPackage};
use elda_db::{Database, InstalledPackageDetails, StateLayout};
use elda_install::MutationPolicy;
use elda_types::PackageVersion;

pub(crate) use crate::app_model::{
    DesiredStateDocument, DesiredStatePackage, DesiredStateProfile, ResolvedProfileState,
};

#[derive(Debug, Clone, Default)]
pub(crate) struct ParsedInstallRequest {
    pub(crate) targets: Vec<String>,
    pub(crate) hard_lane: Option<InstallPreference>,
    pub(crate) preferred_lane: Option<InstallPreference>,
    pub(crate) source_option: Option<usize>,
    pub(crate) source_strategy: Option<String>,
    pub(crate) git_ref: Option<elda_recipe::GitRefRequest>,
    pub(crate) git_source_refs: BTreeMap<String, String>,
    pub(crate) git_ref_overrides: BTreeMap<String, elda_recipe::GitRefRequest>,
    pub(crate) cli_flag_overrides: BTreeMap<String, bool>,
    pub(crate) replace: bool,
    pub(crate) exclude: Vec<String>,
    /// Explicit virtual-name → provider package overrides from `--provider`.
    pub(crate) provider_choices: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedSearchRequest {
    pub(crate) query: String,
    pub(crate) regex: bool,
    pub(crate) interactive: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedUpgradeRequest {
    pub(crate) targets: Vec<String>,
    pub(crate) refresh_weak_deps: bool,
    pub(crate) rebuild_variant_drift: bool,
    pub(crate) git_ref: Option<elda_recipe::GitRefRequest>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedRemoveRequest {
    pub(crate) packages: Vec<String>,
    pub(crate) cascade: bool,
    pub(crate) purge_conffiles: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedDiffRequest {
    pub(crate) package: String,
    pub(crate) candidate: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedDowngradeRequest {
    pub(crate) package: String,
    pub(crate) version: Option<PackageVersion>,
    pub(crate) git_ref: Option<elda_recipe::GitRefRequest>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedRdepsRequest {
    pub(crate) package: String,
    pub(crate) recursive: bool,
    pub(crate) include_weak: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedHoldRequest {
    pub(crate) package: String,
    pub(crate) source: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedVendorAddRequest {
    pub(crate) package_name: String,
    pub(crate) source: String,
    pub(crate) binary: Option<String>,
    pub(crate) asset: Option<String>,
    pub(crate) replace: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedInstallTarget {
    pub(crate) recipe: elda_recipe::RecipeDocument,
    pub(crate) selected_lane: String,
    pub(crate) selected_source_kind: String,
    pub(crate) persisted_source_kind: String,
    pub(crate) flag_state: ResolvedFlagState,
    pub(crate) source_ref: Option<String>,
    pub(crate) remote_name: Option<String>,
    pub(crate) remote_recipe_source: Option<RemoteRecipeSource>,
    pub(crate) binary_source_verification: Option<BinarySourceVerification>,
    pub(crate) ad_hoc_git: bool,
    pub(crate) ad_hoc_git_moving: bool,
    pub(crate) generated_recipe_name: Option<String>,
    pub(crate) generated_recipe_dir: Option<PathBuf>,
    pub(crate) source_options: Vec<elda_recipe::SourceOptionReport>,
    pub(crate) selected_source_option: Option<elda_recipe::SourceOptionReport>,
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteRecipeSource {
    pub(crate) remote_name: String,
    pub(crate) packages_url: String,
    pub(crate) package_name: String,
    pub(crate) repo_commit: String,
    pub(crate) indexed_pkg_lua: String,
}

#[derive(Debug, Clone)]
pub(crate) struct BuiltInstallTarget {
    pub(crate) resolved: ResolvedInstallTarget,
    pub(crate) package: BuiltPackage,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedDependencyPlan {
    pub(crate) target: String,
    pub(crate) dependency_name: String,
    pub(crate) dependency_kind: String,
    pub(crate) raw_expr: String,
    pub(crate) is_weak: bool,
    pub(crate) provider_group: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PlannedInstallAction {
    pub(crate) target: String,
    pub(crate) package_name: String,
    pub(crate) resolved: ResolvedInstallTarget,
    pub(crate) replaced_packages: Vec<String>,
    pub(crate) install_reason: String,
    pub(crate) requested_by: Option<String>,
    pub(crate) dependency_kind: Option<String>,
    pub(crate) raw_expr: Option<String>,
    pub(crate) is_weak: bool,
    pub(crate) provider_group: Option<String>,
    pub(crate) dependencies: Vec<ResolvedDependencyPlan>,
    pub(crate) already_installed: Option<InstalledPackageDetails>,
}

#[derive(Debug, Clone)]
pub(crate) struct PlannedUpgradeAction {
    pub(crate) package_name: String,
    pub(crate) resolved: ResolvedInstallTarget,
    pub(crate) replaced_packages: Vec<String>,
    pub(crate) install_reason: String,
    pub(crate) requested_by: Option<String>,
    pub(crate) dependency_kind: Option<String>,
    pub(crate) raw_expr: Option<String>,
    pub(crate) dependencies: Vec<ResolvedDependencyPlan>,
    pub(crate) installed: Option<InstalledPackageDetails>,
    pub(crate) explicit_target: bool,
    pub(crate) candidate_repo_commit: Option<String>,
    pub(crate) rebuild_variant_drift: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct DependencyCandidate {
    pub(crate) target: String,
    pub(crate) source_priority: Option<u32>,
    pub(crate) candidate_version: Option<PackageVersion>,
}

#[derive(Debug, Clone)]
pub(crate) struct UpgradeDecision {
    pub(crate) installed_version: Option<String>,
    pub(crate) candidate_version: String,
    pub(crate) selected_lane: String,
    pub(crate) needs_change: bool,
    pub(crate) change_kind: &'static str,
    pub(crate) blocked_reason: Option<&'static str>,
    pub(crate) pinned_version: Option<String>,
    pub(crate) hold_source: Option<String>,
}

#[derive(Clone)]
pub struct AppContext {
    pub(crate) config: Config,
    pub(crate) database: Database,
    pub(crate) progress: Arc<dyn ProgressSink>,
    pub(crate) frame_counter: Arc<AtomicU64>,
}

impl std::fmt::Debug for AppContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppContext")
            .field("config", &self.config)
            .field("database", &self.database)
            .finish_non_exhaustive()
    }
}

impl AppContext {
    pub fn from_root(
        root_dir: impl AsRef<Path>,
        force_system_mode: bool,
    ) -> Result<Self, CoreError> {
        let root_dir = root_dir.as_ref();
        let live_host_root = root_dir == Path::new("/");
        let default_privilege = PrivilegeConfig::default();
        let default_request = PrivilegeRequest::from_config(&default_privilege);
        let default_status = PrivilegeStatus::detect(&default_privilege);
        let config_path = root_dir.join("etc/elda/config.toml");
        if !config_path.exists()
            && let Err(error) = Config::write_default(root_dir)
        {
            if matches!(
                &error,
                CoreError::Io(io_error) if io_error.kind() == std::io::ErrorKind::PermissionDenied
            ) && live_host_root
                && !default_status.is_superuser
            {
                return Err(CoreError::PrivilegeRequired(default_request.clone()));
            }
            return Err(error);
        }
        let mut config = match Config::load(root_dir) {
            Ok(config) => config,
            Err(CoreError::Io(error))
                if error.kind() == std::io::ErrorKind::PermissionDenied
                    && live_host_root
                    && !default_status.is_superuser =>
            {
                return Err(CoreError::PrivilegeRequired(default_request.clone()));
            }
            Err(error) => return Err(error),
        };
        if config.defaults.mode_policy == "host" && !config.defaults.allow_system_mode {
            config.defaults.allow_system_mode = true;
        }
        let privilege = PrivilegeStatus::detect(&config.privilege);
        let effective_prefix = if force_system_mode {
            PathBuf::from("/usr")
        } else {
            config.defaults.prefix.clone()
        };

        if live_host_root
            && effective_prefix == Path::new("/usr")
            && !force_system_mode
            && !config.defaults.allow_system_mode
        {
            return Err(CoreError::Operator(
                "live host system mode is disabled; pass `-S` for this invocation or set `defaults.allow_system_mode = true` in `/etc/elda/config.toml`".to_owned(),
            ));
        }
        if live_host_root && !privilege.is_superuser {
            return Err(CoreError::PrivilegeRequired(PrivilegeRequest::from_config(
                &config.privilege,
            )));
        }

        config.defaults.prefix = effective_prefix.clone();
        let layout = StateLayout::new(root_dir, effective_prefix);
        let database = Database::new(layout);

        Ok(Self {
            config,
            database,
            progress: Arc::new(NullSink),
            frame_counter: Arc::new(AtomicU64::new(0)),
        })
    }

    #[must_use]
    pub fn with_progress_sink(mut self, sink: Arc<dyn ProgressSink>) -> Self {
        self.progress = sink;
        self
    }

    pub(crate) fn next_frame_id(&self) -> crate::progress::FrameId {
        crate::progress::next_frame_id(&self.frame_counter)
    }

    pub(crate) fn progress_sink(&self) -> &dyn ProgressSink {
        self.progress.as_ref()
    }

    pub(crate) fn progress_sink_arc(&self) -> std::sync::Arc<dyn ProgressSink> {
        std::sync::Arc::clone(&self.progress)
    }

    pub fn handle(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        self.enforce_capabilities(&request)?;
        crate::app_mutation_gate::confirm_dispatch_mutation(&request)?;
        match request.command_path.as_slice() {
            [command] if command == "a" || command == "add" => self.handle_metadata_add(request),
            [command] if command == "i" || command == "ig" || command == "ib" => {
                self.handle_install(request)
            }
            [command] if command == "ls" => self.handle_ls(request),
            [command] if command == "list" => self.handle_list(request),
            [command] if command == "rm" => self.handle_remove(request),
            [command] if command == "u" => self.handle_upgrade(request),
            [command] if command == "sync" => self.handle_sync(request),
            [command] if command == "check" => self.handle_check(request),
            [command] if command == "doctor" => self.handle_doctor(request),
            [command] if command == "version" => self.handle_version(request),
            [namespace, command] if namespace == "review" && command == "ls" => {
                self.handle_review_list(request)
            }
            [namespace, command] if namespace == "review" && command == "info" => {
                self.handle_review_info(request)
            }
            [namespace, command] if namespace == "review" && command == "forget" => {
                self.handle_review_forget(request)
            }
            [namespace, command] if namespace == "review" && command == "diff" => {
                self.handle_review_diff(request)
            }
            [command] if command == "search" => self.handle_search(request),
            [command] if command == "info" => self.handle_info(request),
            [command] if command == "verify" => self.handle_verify(request),
            [command] if command == "reverify" => self.handle_reverify(request),
            [command] if command == "diff" => self.handle_diff(request),
            [command] if command == "why" => self.handle_why(request),
            [command] if command == "rdeps" => self.handle_rdeps(request),
            [command] if command == "versions" => self.handle_git_versions(request),
            [command] if command == "pin" => self.handle_pin(request),
            [command] if command == "unpin" => self.handle_unpin(request),
            [command] if command == "hold" => self.handle_hold(request),
            [command] if command == "unhold" => self.handle_unhold(request),
            [command] if command == "downgrade" => self.handle_downgrade(request),
            [command] if command == "recover" => self.handle_recover(request),
            [command] if command == "fix-triggers" => self.handle_fix_triggers(request),
            [namespace, command] if namespace == "trigger" && command == "ls" => {
                self.handle_trigger_list(request)
            }
            [namespace, command] if namespace == "trigger" && command == "info" => {
                self.handle_trigger_info(request)
            }
            [namespace, command] if namespace == "trigger" && command == "run" => {
                self.handle_trigger_run(request)
            }
            [namespace, command] if namespace == "trigger" && command == "diff" => {
                self.handle_trigger_diff(request)
            }
            [namespace, command] if namespace == "maint" && command == "check" => {
                self.handle_maint_check(request)
            }
            [namespace, command] if namespace == "maint" && command == "fix" => {
                self.handle_maint_fix(request)
            }
            [command] if command == "init" => self.handle_init(request),
            [namespace, command] if namespace == "config" && command == "pending" => {
                self.handle_config_pending(request)
            }
            [namespace, command] if namespace == "config" && command == "diff" => {
                self.handle_config_diff(request)
            }
            [namespace, command] if namespace == "config" && command == "apply" => {
                self.handle_config_apply(request)
            }
            [namespace, command] if namespace == "config" && command == "keep" => {
                self.handle_config_keep(request)
            }
            [command] if command == "rollback" => self.handle_rollback(request),
            [command] if command == "autoremove" => self.handle_autoremove_plan(request),
            [command] if command == "files" => self.handle_files(request),
            [command, subcommand] if command == "files" && subcommand == "search" => {
                self.handle_file_search(request)
            }
            [command, subcommand] if command == "files" && subcommand == "owner" => {
                self.handle_file_owner(request)
            }
            [namespace, command] if namespace == "rmt" && command == "add" => {
                self.handle_remote_add(request)
            }
            [namespace, command] if namespace == "rmt" && command == "add-from-bundle" => {
                self.handle_remote_add_from_bundle(request)
            }
            [namespace, command] if namespace == "rmt" && command == "ls" => {
                self.handle_remote_list(request)
            }
            [namespace, command] if namespace == "rmt" && command == "info" => {
                self.handle_remote_info(request)
            }
            [namespace, command] if namespace == "rmt" && command == "preview" => {
                self.handle_remote_preview(request)
            }
            [namespace, command] if namespace == "rmt" && command == "trust" => {
                self.handle_remote_trust(request)
            }
            [namespace, command] if namespace == "rmt" && command == "enable" => {
                self.handle_remote_set_enabled(request, true)
            }
            [namespace, command] if namespace == "rmt" && command == "disable" => {
                self.handle_remote_set_enabled(request, false)
            }
            [namespace, command] if namespace == "rmt" && command == "set-priority" => {
                self.handle_remote_set_priority(request)
            }
            [namespace, command] if namespace == "rmt" && command == "rm" => {
                self.handle_remote_remove(request)
            }
            [namespace, command] if namespace == "rc" && command == "add" => {
                self.handle_recipe_add(request)
            }
            [namespace, command] if namespace == "rc" && command == "show" => {
                self.handle_recipe_show(request)
            }
            [namespace, command] if namespace == "rc" && command == "diff" => {
                self.handle_recipe_diff(request)
            }
            [namespace, command] if namespace == "rc" && command == "publish-ready" => {
                self.handle_recipe_publish_ready(request)
            }
            [namespace, command] if namespace == "rc" && command == "edit" => {
                self.handle_recipe_edit(request)
            }
            [namespace, command] if namespace == "rc" && command == "check" => {
                self.handle_recipe_check(request)
            }
            [namespace, command] if namespace == "rc" && command == "format" => {
                self.handle_recipe_format(request)
            }
            [namespace, command] if namespace == "rc" && command == "normalize" => {
                self.handle_recipe_normalize(request)
            }
            [namespace, command] if namespace == "rc" && command == "ls" => {
                self.handle_recipe_ls(request)
            }
            [namespace, command] if namespace == "rc" && command == "rm" => {
                self.handle_recipe_rm(request)
            }
            [namespace, command] if namespace == "vendor" && command == "add" => {
                self.handle_vendor_add(request)
            }
            [namespace, command] if namespace == "vendor" && command == "import" => {
                self.handle_vendor_import(request)
            }
            [namespace, command] if namespace == "vendor" && command == "export" => {
                self.handle_vendor_export(request)
            }
            [namespace, command] if namespace == "cache" && command == "add" => {
                self.handle_cache_add(request)
            }
            [namespace, command] if namespace == "cache" && command == "ls" => {
                self.handle_cache_list(request)
            }
            [namespace, command] if namespace == "pf" && command == "show" => {
                self.handle_profile_show(request)
            }
            [namespace, command] if namespace == "pf" && command == "apply" => {
                self.handle_profile_apply(request)
            }
            [namespace, command] if namespace == "pf" && command == "add" => {
                self.handle_profile_add(request)
            }
            [namespace, command] if namespace == "pf" && command == "rm" => {
                self.handle_profile_remove(request)
            }
            [namespace, command] if namespace == "pf" && command == "set-init" => {
                self.handle_profile_set_init(request)
            }
            [namespace, command] if namespace == "pf" && command == "clear-init" => {
                self.handle_profile_clear_init(request)
            }
            [namespace, command] if namespace == "pf" && command == "set-arch" => {
                self.handle_profile_set_arch(request)
            }
            [namespace, command] if namespace == "pf" && command == "add-foreign-arch" => {
                self.handle_profile_add_foreign_arch(request)
            }
            [namespace, command] if namespace == "pf" && command == "remove-foreign-arch" => {
                self.handle_profile_remove_foreign_arch(request)
            }
            [namespace, command] if namespace == "fl" && command == "check" => {
                self.handle_flag_check(request)
            }
            [namespace, command] if namespace == "fl" && command == "diff" => {
                self.handle_flag_diff(request)
            }
            [command] if command == "adopt" => self.handle_adopt(request),
            [namespace, ..] if namespace == "mg" => self.handle_migration_namespace(request),
            [namespace, command] if namespace == "daemon" && command == "status" => {
                self.handle_daemon_status(request)
            }
            [namespace, command] if namespace == "daemon" && command == "run" => {
                self.handle_daemon_run(request)
            }
            [namespace, command] if namespace == "daemon" && command == "refresh" => {
                self.handle_daemon_refresh(request)
            }
            [namespace, ..] if namespace == "host" => {
                crate::app_host::handle_host_namespace(self, request)
            }
            [namespace, ..] if namespace == "publish" => {
                crate::app_publish::handle_publish_namespace(self, request)
            }
            [namespace, ..] if namespace == "ci" => self.handle_ci_namespace(request),
            [namespace, ..] if namespace == "forge" => self.handle_forge_namespace(request),
            [namespace, command] if namespace == "git" && command == "tags" => {
                self.handle_git_tags(request)
            }
            [namespace, command] if namespace == "git" && command == "releases" => {
                self.handle_git_releases(request)
            }
            [namespace, command] if namespace == "appimage" && command == "inspect" => {
                self.handle_appimage_inspect(request)
            }
            [namespace, command] if namespace == "ext" && command == "ls" => {
                self.handle_extension_list(request)
            }
            [namespace, ..] if namespace == "qa" => self.handle_qa_namespace(request),
            [namespace, command] if namespace == "state" && command == "show" => {
                self.handle_state_show(request)
            }
            [namespace, command] if namespace == "state" && command == "export" => {
                self.handle_state_export(request)
            }
            [namespace, command] if namespace == "state" && command == "import" => {
                self.handle_state_import(request)
            }
            _ => Ok(self.handle_stub(request)),
        }
    }

    fn enforce_capabilities(&self, request: &CommandRequest) -> Result<(), CoreError> {
        let capabilities = &self.config.capabilities;
        let needs = match request.command_path.as_slice() {
            [command] if command == "sync" || command == "search" || command == "info" => {
                Some(("network.fetch", capabilities.network_fetch))
            }
            [namespace, ..] if namespace == "git" => {
                Some(("network.fetch", capabilities.network_fetch))
            }
            [command] if command == "versions" => {
                Some(("network.fetch", capabilities.network_fetch))
            }
            [command] if command == "a" || command == "add" => {
                Some(("local.exec_build", capabilities.local_exec_build))
            }
            [command] if command == "i" || command == "ig" || command == "ib" => {
                Some(("local.exec_build", capabilities.local_exec_build))
            }
            [namespace, ..] if namespace == "ci" => {
                Some(("network.publish", capabilities.network_publish))
            }
            [namespace, ..] if namespace == "mg" => Some(("migration", capabilities.migration)),
            [namespace, ..] if namespace == "ext" => {
                Some(("extension.runtime", capabilities.extension_runtime))
            }
            [namespace, command] if namespace == "pf" && command == "apply" => {
                Some(("profile.apply", capabilities.profile_apply))
            }
            _ => None,
        };

        if let Some((name, enabled)) = needs
            && !enabled
        {
            return Err(CoreError::Operator(format!(
                "command disabled by capability policy `{name}` (profile `{}`)",
                capabilities.profile
            )));
        }

        Ok(())
    }

    pub(crate) fn mutation_policy(&self) -> MutationPolicy {
        let snapshot_tool = self.config.defaults.snapshot_tool.trim().to_owned();

        MutationPolicy {
            snapshot_tool: (!snapshot_tool.is_empty() && snapshot_tool != "none")
                .then_some(snapshot_tool),
        }
    }
}

fn display_tree_style(value: &str) -> Option<TreeStyle> {
    match value.trim().to_ascii_lowercase().as_str() {
        "ascii" => Some(TreeStyle::Ascii),
        "unicode" => Some(TreeStyle::Unicode),
        _ => None,
    }
}

pub fn run_from_root(
    root_dir: impl AsRef<Path>,
    mut request: CommandRequest,
) -> Result<CommandReport, CoreError> {
    let root_dir = root_dir.as_ref();
    crate::interrupt::install_handler_if_needed(&request)?;
    let context = AppContext::from_root(root_dir, request.system_mode)?;
    set_configured_tree_style(display_tree_style(&context.config.display.tree_chars));
    if request.output_mode == crate::OutputMode::Human
        && context.config.display.default_mode == "json"
    {
        request.output_mode = crate::OutputMode::Json;
    }
    let context = if let Some(mode) = LiveSinkMode::detect(request.output_mode, request.no_stream) {
        context.with_progress_sink(Arc::new(LiveSink::new(mode)))
    } else {
        context
    };
    let log_session = CommandLogSession::start(root_dir, &context.config, &request)?;
    let request_for_logging = request.clone();
    let result = context.handle(request);

    match result {
        Ok(mut report) => {
            if let Some(log_session) = &log_session {
                log_session.write_success(
                    root_dir,
                    &context.config,
                    &request_for_logging,
                    &report,
                )?;
                log_session.attach_to_report(&mut report);
            }
            Ok(report)
        }
        Err(error) => {
            if let Some(log_session) = &log_session {
                log_session.write_error(root_dir, &context.config, &request_for_logging, &error)?;
            }
            Err(error)
        }
    }
}
