pub mod cache;
pub mod manifest;
pub mod source;
pub mod fetcher;
pub mod dataset;
pub mod types;
pub mod query;
pub mod db;

pub use cache::Cache;
pub use fetcher::Fetcher;
pub use source::Dataset;
pub use dataset::DatasetLoader;
pub use query::StatsQuery;
pub use db::DatabaseManager;
pub use types::*;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
