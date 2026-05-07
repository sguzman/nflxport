use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use reqwest::blocking::Client;
use std::io::{Read, Write};
use std::fs;
use indicatif::{ProgressBar, ProgressStyle};
use crate::cache::Cache;
use crate::source::Dataset;
use crate::manifest::{DatasetEntry, DatasetStatus};
use chrono::Utc;

pub struct Fetcher {
    client: Client,
    cache: Cache,
}

impl Fetcher {
    pub fn new(cache: Cache) -> Self {
        Self {
            client: Client::new(),
            cache,
        }
    }

    pub fn fetch(&self, dataset: Dataset, force: bool) -> Result<Utf8PathBuf> {
        let mut manifest = self.cache.load_manifest()?;
        let key = dataset.cache_key();
        let url = dataset.url();
        let local_rel_path = dataset.relative_path();
        let local_path = self.cache.get_raw_path(&local_rel_path);

        if !force && local_path.exists() {
            if let Some(entry) = manifest.datasets.get(&key) {
                if entry.status == DatasetStatus::Ok {
                    tracing::info!(dataset = %key, "Dataset already in cache");
                    return Ok(local_path);
                }
            }
        }

        tracing::info!(dataset = %key, url = %url, "Fetching dataset");
        self.cache.ensure_dir(&local_path)?;

        let mut response = self.client.get(&url).send().context("Request failed")?;
        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch dataset: HTTP {}", response.status());
        }

        let total_size = response.content_length().unwrap_or(0);
        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"));

        let mut file = fs::File::create(&local_path).context("Failed to create file")?;
        let mut downloaded: u64 = 0;
        let mut buffer = [0; 8192];

        loop {
            let n = response.read(&mut buffer).context("Read error")?;
            if n == 0 { break; }
            file.write_all(&buffer[..n]).context("Write error")?;
            downloaded += n as u64;
            pb.set_position(downloaded);
        }

        pb.finish_with_message("Download complete");

        let entry = DatasetEntry {
            url,
            local_path: Utf8PathBuf::from(local_rel_path),
            downloaded_at: Utc::now(),
            etag: response.headers().get("etag").and_then(|h| h.to_str().ok().map(|s| s.to_string())),
            last_modified: response.headers().get("last-modified").and_then(|h| h.to_str().ok().map(|s| s.to_string())),
            sha256: None, // TODO: calculate sha256
            bytes: downloaded,
            status: DatasetStatus::Ok,
        };

        manifest.datasets.insert(key, entry);
        self.cache.save_manifest(&manifest)?;

        Ok(local_path)
    }
}
