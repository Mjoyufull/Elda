use std::path::Path;

use crate::error::RecipeError;

use super::detect::infer_recipe_name;

pub(super) fn remote_probe_error(
    recipes_dir: &Path,
    source_url: &str,
    error: RecipeError,
) -> RecipeError {
    let recipe_name = infer_recipe_name(source_url);
    let recipe_dir = recipes_dir.join(&recipe_name);
    if recipe_dir.join("pkg.lua").is_file() {
        return RecipeError::InvalidInput(format!(
            "remote source `{source_url}` could not be probed: {error}; local recipe `{recipe_name}` exists at {}; install `{recipe_name}` explicitly to use local metadata",
            recipe_dir.display(),
        ));
    }

    RecipeError::InvalidInput(format!(
        "remote source `{source_url}` could not be probed: {error}"
    ))
}
