use anyhow::Result;
use polars::prelude::*;
use crate::dataset::DatasetLoader;
use crate::source::Dataset;

pub struct StatsQuery {
    loader: DatasetLoader,
}

impl StatsQuery {
    pub fn new(cache: crate::cache::Cache) -> Self {
        Self { loader: DatasetLoader::new(cache) }
    }

    pub fn leaders(&self, category: &str, limit: usize) -> Result<DataFrame> {
        let df = self.loader.load(Dataset::PlayerStats)?;
        
        let res = df.lazy()
            .select([
                col("player_display_name"),
                col("recent_team"),
                col(category),
            ])
            .filter(col(category).is_not_null())
            .sort([category], SortMultipleOptions {
                descending: vec![true],
                nulls_last: vec![true],
                ..Default::default()
            })
            .limit(limit as u32)
            .collect()?;
            
        Ok(res)
    }

    pub fn team_summary(&self, team: &str) -> Result<DataFrame> {
        let df = self.loader.load(Dataset::PlayerStats)?;
        
        let res = df.lazy()
            .filter(col("recent_team").eq(lit(team)))
            .group_by([col("player_display_name")])
            .agg([
                col("passing_yards").sum().alias("pass_yds"),
                col("rushing_yards").sum().alias("rush_yds"),
                col("receptions").sum().alias("rec"),
                (col("passing_tds") + col("rushing_tds") + col("receiving_tds")).sum().alias("total_tds"),
            ])
            .sort(["pass_yds"], SortMultipleOptions {
                descending: vec![true],
                ..Default::default()
            })
            .limit(10)
            .collect()?;
            
        Ok(res)
    }

    pub fn player_search(&self, name: &str) -> Result<DataFrame> {
        let df = self.loader.load(Dataset::Players)?;
        
        let res = df.lazy()
            .filter(col("display_name").str().contains(lit(name), true))
            .select([
                col("display_name"),
                col("position"),
                col("latest_team"),
                col("gsis_id"),
            ])
            .collect()?;
            
        Ok(res)
    }
}
