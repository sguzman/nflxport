use anyhow::{Context, Result};
use polars::prelude::*;
use crate::cache::Cache;
use crate::source::Dataset;

pub struct DatasetLoader {
    cache: Cache,
}

impl DatasetLoader {
    pub fn new(cache: Cache) -> Self {
        Self { cache }
    }

    pub fn load(&self, dataset: Dataset) -> Result<DataFrame> {
        let rel_path = dataset.relative_path();
        let path = self.cache.get_raw_path(&rel_path);
        
        if !path.exists() {
            anyhow::bail!("Dataset not found in cache: {}. Run fetch first.", dataset.cache_key());
        }

        tracing::info!(path = %path, "Loading parquet dataset");
        
        ParquetReader::new(std::fs::File::open(&path)?)
            .finish()
            .context("Failed to read parquet file")
    }

    pub fn schema(&self, dataset: Dataset) -> Result<SchemaRef> {
        let rel_path = dataset.relative_path();
        let path = self.cache.get_raw_path(&rel_path);
        
        if !path.exists() {
            anyhow::bail!("Dataset not found in cache: {}. Run fetch first.", dataset.cache_key());
        }

        let arrow_schema = ParquetReader::new(std::fs::File::open(&path)?).schema()?;
        Ok(Schema::from_arrow_schema(&arrow_schema).into())
    }
}
