#![forbid(unsafe_code)]

mod check;
mod error;
mod import;
mod model;
mod parser;
mod validate;
mod vendor;

use elda_types::CrateBoundary;

pub use check::{CheckedRecipe, RecipeCheckReport, RecipeIssue, check_local_recipes, load_recipe};
pub use error::RecipeError;
pub use import::{ImportReport, add_recipe, infer_recipe_name, is_git_like_target};
pub use model::{
    BuildDefinition, DependencyEntry, GitHubReleaseAssetDefinition, IssueSeverity, LuaValue,
    PackageDefinition, ProfilePolicy, RecipeDocument, SOURCE_LANE_BINARY, SOURCE_LANE_SOURCE,
    ScalarValue, SourceDefinition, SourceLaneDefinition, ValidationIssue,
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
