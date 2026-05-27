#[path = "../../build_support/build_metadata.rs"]
mod build_metadata;

fn main() {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    build_metadata::emit_build_metadata(&workspace);
}
