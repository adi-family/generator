pub mod schema;
pub mod loader;

pub use schema::GenerationConfig;
pub use loader::{load_config, merge_with_cli_args};
