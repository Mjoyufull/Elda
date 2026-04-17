use crate::app::{AppContext, ResolvedInstallTarget};
use crate::app_install::dependency::constraint::parse_dependency_constraint;
use crate::error::CoreError;

impl AppContext {
    pub(crate) fn planned_replacements(
        &self,
        resolved: &ResolvedInstallTarget,
    ) -> Result<Vec<String>, CoreError> {
        let mut replaced = Vec::new();

        for replacement in &resolved.recipe.package.replaces {
            let constraint = parse_dependency_constraint(replacement)?;
            let Some(installed) = self.database.installed_package(&constraint.name)? else {
                continue;
            };
            if !constraint.matches_name(&installed.pkgname) {
                continue;
            }
            if installed.pkgname == resolved.recipe.package.name {
                continue;
            }
            self.validate_replacement_target(resolved, &installed)?;
            replaced.push(installed.pkgname);
        }

        replaced.sort();
        replaced.dedup();

        Ok(replaced)
    }

    fn validate_replacement_target(
        &self,
        resolved: &ResolvedInstallTarget,
        replaced: &elda_db::InstalledPackageDetails,
    ) -> Result<(), CoreError> {
        if replacement_crosses_source_boundary(replaced, resolved) {
            return Err(CoreError::Operator(format!(
                "replacement of installed package `{}` across source boundaries is rejected in non-interactive mode",
                replaced.pkgname,
            )));
        }

        for reverse in self
            .database
            .reverse_dependencies(&replaced.pkgname, false)?
        {
            return Err(CoreError::Operator(format!(
                "package `{}` cannot replace `{}` because installed package `{}` depends on `{}`",
                resolved.recipe.package.name, replaced.pkgname, reverse.pkgname, reverse.raw_expr,
            )));
        }

        Ok(())
    }
}

fn replacement_crosses_source_boundary(
    replaced: &elda_db::InstalledPackageDetails,
    resolved: &ResolvedInstallTarget,
) -> bool {
    replaced.source_kind != resolved.persisted_source_kind
        || replaced.remote_name != resolved.remote_name
}
