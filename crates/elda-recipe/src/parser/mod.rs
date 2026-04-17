mod fields;
mod lua;
mod mapper;

use std::path::Path;

use crate::error::RecipeError;
use crate::model::RecipeDocument;

use self::lua::LuaParser;
use self::mapper::map_package_definition;

pub fn parse_pkg_lua(path: &Path, content: &str) -> Result<RecipeDocument, RecipeError> {
    let mut parser = LuaParser::new(content);
    let root_table = parser.parse_document()?;
    let package = map_package_definition(root_table)?;

    Ok(RecipeDocument {
        path: path.to_path_buf(),
        package,
    })
}
