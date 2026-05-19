mod detect;
mod detected;
mod legacy;
mod metadata;
mod model;
mod release_options;
mod release_sidecars;
mod release_target;
mod render;
mod snapshot;
mod strategy;
mod workflow;
mod workflow_render;
mod workflow_snapshot;

pub use detect::{infer_recipe_name, is_git_like_target};
pub use model::{
    GitRefKind, GitRefRequest, ImportOptions, ImportReport, ImportResult, SnapshotImportReport,
    SourceOptionReport,
};
pub use release_options::{
    default_release_binary_format_priority, effective_release_binary_format_priority,
};
pub use workflow::{add_recipe, add_recipe_with_options, add_recipe_with_priority};

#[cfg(test)]
mod overwrite_tests;
#[cfg(test)]
mod release_format_tests;
#[cfg(test)]
mod release_tests;
#[cfg(test)]
mod tests;
