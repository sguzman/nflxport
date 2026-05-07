use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use camino::Utf8PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub datasets: HashMap<String, DatasetEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetEntry {
    pub url: String,
    pub local_path: Utf8PathBuf,
    pub downloaded_at: DateTime<Utc>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub sha256: Option<String>,
    pub bytes: u64,
    pub status: DatasetStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DatasetStatus {
    Ok,
    Error,
    Pending,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            datasets: HashMap::new(),
        }
    }
}
