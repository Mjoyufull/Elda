use crate::error::CoreError;
use elda_db::InstalledPackageDetails;
use elda_recipe::PackageDefinition;
use elda_types::{ConstraintVersion, NamedConstraint};

pub(crate) fn parse_dependency_constraint(value: &str) -> Result<NamedConstraint, CoreError> {
    NamedConstraint::parse_dependency(value).map_err(|error| {
        CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(format!(
            "invalid dependency constraint `{value}`: {error}"
        )))
    })
}

pub(crate) fn package_satisfies_constraint(
    package: &PackageDefinition,
    constraint: &NamedConstraint,
) -> bool {
    constraint.matches_name(&package.name)
        && constraint.matches_version(&ConstraintVersion::from_parts(
            package.epoch,
            package.version.clone(),
            Some(package.rel),
        ))
}

pub(crate) fn installed_package_satisfies_constraint(
    package: &InstalledPackageDetails,
    constraint: &NamedConstraint,
) -> bool {
    constraint.matches_name(&package.pkgname)
        && constraint.matches_version(&ConstraintVersion::from_parts(
            package.epoch,
            package.pkgver.clone(),
            Some(package.pkgrel),
        ))
}

pub(crate) fn provide_satisfies_constraint(
    provide: &str,
    constraint: &NamedConstraint,
) -> Result<bool, CoreError> {
    let provided = elda_types::NamedConstraint::parse_provide(provide).map_err(|error| {
        CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(format!(
            "invalid provide `{provide}`: {error}"
        )))
    })?;
    if !constraint.matches_name(&provided.name) {
        return Ok(false);
    }
    if !constraint.is_versioned() {
        return Ok(true);
    }
    let Some(version) = provided.version.as_ref() else {
        return Ok(false);
    };

    Ok(constraint.matches_version(version))
}

pub(crate) fn provides_satisfy_constraint(
    provides: &[String],
    constraint: &NamedConstraint,
) -> Result<bool, CoreError> {
    for provide in provides {
        if provide_satisfies_constraint(provide, constraint)? {
            return Ok(true);
        }
    }

    Ok(false)
}
