use super::command_name::command_name;
use super::common::{
    AdoptArgs, DiffArgs, DowngradeArgs, HoldArgs, InstallArgs, InstallTargetsArgs, ListArgs,
    ListDetailArgs, PackageArg, RdepsArgs, RemoveArgs, RollbackArgs, SearchArgs, TargetsArgs,
    UpgradeArgs, push_flag, push_optional,
};
use super::file_commands::{self, FilesArgs};
use super::root::Command;

pub(super) fn request_parts(command: &Command) -> (Vec<String>, Vec<String>) {
    match command {
        Command::A(args) | Command::Add(args) | Command::I(args) => install_parts(command, args),
        Command::Ig(args) | Command::Ib(args) => install_lane_parts(command, args),
        Command::Rm(args) => remove_parts(command, args),
        Command::U(args) => upgrade_parts(command, args),
        Command::Sync(args) => targets_parts(command, args),
        Command::Ls(args) => list_parts(command, args),
        Command::List(args) => list_detail_parts(command, args),
        Command::Check
        | Command::Doctor
        | Command::Version
        | Command::Init
        | Command::Recover
        | Command::Autoremove => (vec![command_name(command)], Vec::new()),
        Command::Search(args) => search_parts(command, args),
        Command::Info(args)
        | Command::Reverify(args)
        | Command::Why(args)
        | Command::Pin(args)
        | Command::Unpin(args)
        | Command::Unhold(args) => package_parts(command, args),
        Command::Files(args) => files_parts(args),
        Command::Verify(args) => targets_parts(command, args),
        Command::Rdeps(args) => rdeps_parts(command, args),
        Command::Versions(args) => {
            let mut operands = vec![args.target.clone()];
            if let Some(max_tags) = args.max_tags {
                operands.push("--max-tags".to_owned());
                operands.push(max_tags.to_string());
            }
            (vec!["versions".to_owned()], operands)
        }
        Command::Hold(args) => hold_parts(command, args),
        Command::Adopt(args) => adopt_parts(command, args),
        Command::Downgrade(args) => downgrade_parts(command, args),
        Command::Diff(args) => diff_parts(command, args),
        Command::Rollback(args) => rollback_parts(command, args),
        Command::FixTriggers => (vec![command_name(command)], Vec::new()),
        Command::Rmt { command } => command.request_parts(),
        Command::Rc { command } => command.request_parts(),
        Command::Ci { command } => command.request_parts(),
        Command::Vendor { command } => command.request_parts(),
        Command::Forge { command } => command.request_parts(),
        Command::Host { command } => command.request_parts(),
        Command::Publish { command } => command.request_parts(),
        Command::Git { command } => command.request_parts(),
        Command::AppImage { command } => command.request_parts(),
        Command::Pf { command } => command.request_parts(),
        Command::Fl { command } => command.request_parts(),
        Command::Mg { command } => command.request_parts(),
        Command::State { command } => command.request_parts(),
        Command::Cache { command } => command.request_parts(),
        Command::Daemon { command } => command.request_parts(),
        Command::Ext { command } => command.request_parts(),
        Command::Qa { command } => command.request_parts(),
        Command::Trigger { command } => command.request_parts(),
        Command::Maint { command } => command.request_parts(),
        Command::Review { command } => command.request_parts(),
        Command::Config { command } => command.request_parts(),
    }
}

fn install_parts(command: &Command, args: &InstallArgs) -> (Vec<String>, Vec<String>) {
    let mut operands = args.targets.clone();
    for flag in &args.use_flags {
        operands.push(format!("--use={flag}"));
    }
    push_optional(
        &mut operands,
        "--source-option",
        args.source_option.map(|value| value.to_string()).as_deref(),
    );
    push_optional(&mut operands, "--strategy", args.strategy.as_deref());
    push_optional(&mut operands, "--to-branch", args.to_branch.as_deref());
    push_optional(&mut operands, "--to-tag", args.to_tag.as_deref());
    push_optional(&mut operands, "--to-rev", args.to_rev.as_deref());
    push_flag(&mut operands, "--pick-tag", args.pick_tag);
    for provider in &args.provider {
        operands.push("--provider".to_owned());
        operands.push(provider.clone());
    }
    push_flag(&mut operands, "--prefer-source", args.prefer_source);
    push_flag(&mut operands, "--prefer-binary", args.prefer_binary);
    push_flag(&mut operands, "--replace", args.replace);
    if !args.exclude.is_empty() {
        operands.push("--exclude".to_owned());
        operands.extend(args.exclude.iter().cloned());
    }
    (vec![command_name(command)], operands)
}

fn install_lane_parts(command: &Command, args: &InstallTargetsArgs) -> (Vec<String>, Vec<String>) {
    let mut operands = args.targets.clone();
    for flag in &args.use_flags {
        operands.push(format!("--use={flag}"));
    }
    (vec![command_name(command)], operands)
}

fn remove_parts(command: &Command, args: &RemoveArgs) -> (Vec<String>, Vec<String>) {
    let mut operands = args.packages.clone();
    push_flag(&mut operands, "--cascade", args.cascade);
    push_flag(&mut operands, "--purge-conffiles", args.purge_conffiles);
    (vec![command_name(command)], operands)
}

fn upgrade_parts(command: &Command, args: &UpgradeArgs) -> (Vec<String>, Vec<String>) {
    let mut operands = args.targets.clone();
    push_flag(&mut operands, "--refresh-weak-deps", args.refresh_weak_deps);
    push_flag(
        &mut operands,
        "--rebuild-variant-drift",
        args.rebuild_variant_drift,
    );
    push_optional(&mut operands, "--to-branch", args.to_branch.as_deref());
    push_optional(&mut operands, "--to-tag", args.to_tag.as_deref());
    push_optional(&mut operands, "--to-rev", args.to_rev.as_deref());
    push_flag(&mut operands, "--pick-tag", args.pick_tag);
    (vec![command_name(command)], operands)
}

fn list_parts(command: &Command, args: &ListArgs) -> (Vec<String>, Vec<String>) {
    let mut operands = Vec::new();
    push_list_filter_operands(&mut operands, args);
    (vec![command_name(command)], operands)
}

fn list_detail_parts(command: &Command, args: &ListDetailArgs) -> (Vec<String>, Vec<String>) {
    let mut operands = args.packages.clone();
    push_list_filter_operands(&mut operands, &args.filters);
    (vec![command_name(command)], operands)
}

fn push_list_filter_operands(operands: &mut Vec<String>, args: &ListArgs) {
    push_flag(operands, "--explicit", args.explicit);
    push_flag(operands, "--deps", args.deps);
    push_flag(operands, "--held", args.held);
    push_flag(operands, "--pinned", args.pinned);
    push_optional(operands, "--source-kind", args.source_kind.as_deref());
}

fn search_parts(command: &Command, args: &SearchArgs) -> (Vec<String>, Vec<String>) {
    let mut operands = vec![args.query.clone()];
    push_flag(&mut operands, "--regex", args.regex);
    push_flag(&mut operands, "--interactive", args.interactive);
    (vec![command_name(command)], operands)
}

fn targets_parts(command: &Command, args: &TargetsArgs) -> (Vec<String>, Vec<String>) {
    (vec![command_name(command)], args.targets.clone())
}

fn package_parts(command: &Command, args: &PackageArg) -> (Vec<String>, Vec<String>) {
    (vec![command_name(command)], vec![args.package.clone()])
}

fn files_parts(args: &FilesArgs) -> (Vec<String>, Vec<String>) {
    file_commands::request_parts(args)
}

fn rdeps_parts(command: &Command, args: &RdepsArgs) -> (Vec<String>, Vec<String>) {
    let mut operands = vec![args.package.clone()];
    push_flag(&mut operands, "--all", args.all);
    push_flag(&mut operands, "--weak", args.weak);
    (vec![command_name(command)], operands)
}

fn hold_parts(command: &Command, args: &HoldArgs) -> (Vec<String>, Vec<String>) {
    let mut operands = vec![args.package.clone()];
    push_optional(&mut operands, "--source", args.source.as_deref());
    (vec![command_name(command)], operands)
}

fn adopt_parts(command: &Command, args: &AdoptArgs) -> (Vec<String>, Vec<String>) {
    (
        vec![command_name(command)],
        vec!["--from".to_owned(), args.from.clone(), args.package.clone()],
    )
}

fn downgrade_parts(command: &Command, args: &DowngradeArgs) -> (Vec<String>, Vec<String>) {
    let mut operands = vec![args.package.clone()];
    if let Some(version) = &args.version {
        operands.push(version.clone());
    }
    push_optional(&mut operands, "--to-tag", args.to_tag.as_deref());
    push_optional(&mut operands, "--to-rev", args.to_rev.as_deref());
    (vec![command_name(command)], operands)
}

fn diff_parts(command: &Command, args: &DiffArgs) -> (Vec<String>, Vec<String>) {
    let mut operands = vec![args.package.clone()];
    push_flag(&mut operands, "--candidate", args.candidate);
    (vec![command_name(command)], operands)
}

fn rollback_parts(command: &Command, args: &RollbackArgs) -> (Vec<String>, Vec<String>) {
    (
        vec![command_name(command)],
        args.state_id.clone().into_iter().collect(),
    )
}
