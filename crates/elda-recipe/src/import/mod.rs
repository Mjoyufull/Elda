mod detect;
mod legacy;
mod model;
mod render;
mod workflow;

pub use detect::{infer_recipe_name, is_git_like_target};
pub use model::ImportReport;
pub use workflow::add_recipe;

#[cfg(test)]
mod tests;
