use anyhow::{Context, Result};
use duckdb::Connection;
use crate::cache::Cache;
use crate::source::Dataset;

pub struct DatabaseManager {
    conn: Connection,
    cache: Cache,
}

impl DatabaseManager {
    pub fn new(cache: Cache) -> Result<Self> {
        let db_path = cache.root.join("nflxport.db");
        let conn = Connection::open(db_path).context("Failed to open DuckDB")?;
        
        // Install/load parquet extension if needed
        conn.execute_batch("INSTALL parquet; LOAD parquet;")?;
        
        Ok(Self { conn, cache })
    }

    pub fn build_from_cache(&self, datasets: &[Dataset]) -> Result<()> {
        for ds in datasets {
            let path = self.cache.get_raw_path(&ds.relative_path());
            if !path.exists() {
                tracing::warn!("Skipping dataset {} because it's not in cache at {}", ds.cache_key(), path);
                continue;
            }

            let table_name = ds.cache_key().replace("/", "_");
            tracing::info!("Ingesting {} into DuckDB table {}", ds.cache_key(), table_name);

            // Drop existing table
            self.conn.execute(&format!("DROP TABLE IF EXISTS {}", table_name), [])?;

            // Create table from parquet
            self.conn.execute(
                &format!("CREATE TABLE {} AS SELECT * FROM read_parquet('{}')", table_name, path),
                [],
            ).context(format!("Failed to ingest {}", table_name))?;
        }
        Ok(())
    }

    pub fn query(&self, sql: &str) -> Result<Vec<Vec<String>>> {
        let mut stmt = self.conn.prepare(sql)?;
        let mut rows = stmt.query([])?;
        
        let mut results: Vec<Vec<String>> = Vec::new();
        let mut headers_added = false;

        while let Some(row) = rows.next()? {
            // DuckDB metadata can be finicky before/during execution in some versions of the crate.
            // We fetch columns iteratively.
            let mut values: Vec<String> = Vec::new();
            let mut i = 0;
            loop {
                let val_res: std::result::Result<duckdb::types::Value, _> = row.get(i);
                match val_res {
                    Ok(val) => {
                        let s = match val {
                            duckdb::types::Value::Null => "NULL".to_string(),
                            duckdb::types::Value::Boolean(b) => b.to_string(),
                            duckdb::types::Value::TinyInt(i) => i.to_string(),
                            duckdb::types::Value::SmallInt(i) => i.to_string(),
                            duckdb::types::Value::Int(i) => i.to_string(),
                            duckdb::types::Value::BigInt(i) => i.to_string(),
                            duckdb::types::Value::Float(f) => f.to_string(),
                            duckdb::types::Value::Double(d) => d.to_string(),
                            duckdb::types::Value::Text(s) => s,
                            duckdb::types::Value::Blob(b) => format!("<blob {} bytes>", b.len()),
                            _ => "...".to_string(),
                        };
                        values.push(s);
                        i += 1;
                    }
                    Err(_) => break,
                }
            }

            if !headers_added {
                // We can't easily get names without the same iterative trick or stmt access.
                // For now, let's just add generic headers or skip.
                let headers: Vec<String> = (0..values.len()).map(|idx| format!("col_{}", idx)).collect();
                results.push(headers);
                headers_added = true;
            }
            results.push(values);
        }
        
        Ok(results)
    }
}
