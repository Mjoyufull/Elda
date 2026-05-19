#![forbid(unsafe_code)]

mod check;
mod error;
mod format;
mod import;
mod model;
mod parser;
mod validate;
mod vendor;

use elda_types::CrateBoundary;

pub use check::{CheckedRecipe, RecipeCheckReport, RecipeIssue, check_local_recipes, load_recipe};
pub use error::RecipeError;
pub use format::{
    format_recipe_file, normalize_recipe_file, render_pkg_lua, write_formatted_recipe,
};
pub use import::{
    GitRefKind, GitRefRequest, ImportOptions, ImportReport, ImportResult, SnapshotImportReport,
    SourceOptionReport, add_recipe, add_recipe_with_options, add_recipe_with_priority,
    default_release_binary_format_priority, effective_release_binary_format_priority,
    infer_recipe_name, is_git_like_target,
};
pub use model::{
    BuildDefinition, DependencyBody, DependencyEntry, FlagPredicate, FlagPredicateAtom,
    GitHubReleaseAssetDefinition, IssueSeverity, LuaValue, PackageDefinition, ProfilePolicy,
    RecipeDocument, SOURCE_LANE_BINARY, SOURCE_LANE_SOURCE, ScalarValue, SourceDefinition,
    SourceLaneDefinition, ValidationIssue,
};
pub use parser::parse_pkg_lua;
pub use validate::validate_recipe;
pub use vendor::{
    VendorExportReport, VendorImportReport, VendorLockEntry, VendorLockFile, VendorRecipeReport,
    add_vendor_recipe, export_vendor_source, import_vendor_source,
};

pub const BOUNDARY: CrateBoundary = CrateBoundary::new(
    "elda-recipe",
    "Local recipe discovery, declarative pkg.lua validation, and legacy import seams.",
);
