use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use std::fs;
use crate::manifest::Manifest;

#[derive(Clone)]
pub struct Cache {
    pub root: Utf8PathBuf,
    pub manifest_path: Utf8PathBuf,
}

impl Cache {
    pub fn new(root: Utf8PathBuf) -> Result<Self> {
        let manifest_path = root.join("manifest.json");
        
        if !root.exists() {
            fs::create_dir_all(&root).context("Failed to create cache root directory")?;
        }
        
        Ok(Self { root, manifest_path })
    }

    pub fn load_manifest(&self) -> Result<Manifest> {
        if !self.manifest_path.exists() {
            return Ok(Manifest::default());
        }
        
        let content = fs::read_to_string(&self.manifest_path)
            .context("Failed to read manifest file")?;
        serde_json::from_str(&content).context("Failed to parse manifest JSON")
    }

    pub fn save_manifest(&self, manifest: &Manifest) -> Result<()> {
        let content = serde_json::to_string_pretty(manifest)
            .context("Failed to serialize manifest to JSON")?;
        fs::write(&self.manifest_path, content)
            .context("Failed to write manifest file")
    }

    pub fn get_raw_path(&self, subpath: &str) -> Utf8PathBuf {
        self.root.join("raw").join(subpath)
    }

    pub fn ensure_dir(&self, path: &Utf8PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create directory")?;
        }
        Ok(())
    }
}
