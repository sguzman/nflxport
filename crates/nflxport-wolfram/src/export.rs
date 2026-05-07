use anyhow::{Context, Result};
use polars::prelude::*;
use camino::Utf8PathBuf;
use std::fs;
use crate::core_reexport::{Cache, DatasetLoader, Dataset};

pub struct WolframExporter {
    loader: DatasetLoader,
    export_dir: Utf8PathBuf,
}

impl WolframExporter {
    pub fn new(cache: Cache, export_dir: Utf8PathBuf) -> Self {
        Self {
            loader: DatasetLoader::new(cache),
            export_dir,
        }
    }

    pub fn export_csv(&self, dataset: Dataset) -> Result<Utf8PathBuf> {
        let mut df = self.loader.load(dataset.clone())?;
        let filename = format!("{}.csv", dataset.cache_key().replace("/", "_"));
        let path = self.export_dir.join(filename);
        
        if !self.export_dir.exists() {
            fs::create_dir_all(&self.export_dir).context("Failed to create export directory")?;
        }

        let mut file = fs::File::create(&path).context("Failed to create export file")?;
        CsvWriter::new(&mut file)
            .include_header(true)
            .finish(&mut df)
            .context("Failed to write CSV")?;
            
        Ok(path)
    }

    pub fn generate_manifest_wl(&self, datasets: &[Dataset]) -> Result<Utf8PathBuf> {
        let mut wl_content = String::new();
        wl_content.push_str("(* NFLXport Data Manifest *)\n\n");
        wl_content.push_str(&format!("NFLXportDataDirectory = \"{}\";\n\n", self.export_dir));
        
        wl_content.push_str("NFLXportImportCSV[name_String] := Import[\n");
        wl_content.push_str("  FileNameJoin[{NFLXportDataDirectory, name <> \".csv\"}],\n");
        wl_content.push_str("  \"Dataset\",\n");
        wl_content.push_str("  \"HeaderLines\" -> 1\n");
        wl_content.push_str("];\n\n");

        for ds in datasets {
            let name = ds.cache_key().replace("/", "_");
            let func_name = match ds {
                Dataset::Teams => "NFLTeams".to_string(),
                Dataset::Schedules => "NFLSchedules".to_string(),
                Dataset::Players => "NFLPlayers".to_string(),
                Dataset::Pbp(year) => format!("NFLPBP[{}]", year),
                Dataset::PlayerStats => "NFLPlayerStats".to_string(),
                Dataset::TeamStats => "NFLTeamStats".to_string(),
            };
            
            if func_name.contains('[') {
                // Handle parameterized functions separately if needed, 
                // but for now let's just do a simple mapping for the core ones
                let base_name = func_name.split('[').next().unwrap();
                let param = func_name.split('[').nth(1).unwrap().trim_end_matches(']');
                wl_content.push_str(&format!("{}[{}] := NFLXportImportCSV[\"{}\"];\n", base_name, param, name));
            } else {
                wl_content.push_str(&format!("{}[] := NFLXportImportCSV[\"{}\"];\n", func_name, name));
            }
        }

        let path = self.export_dir.join("NFLXport.wl");
        fs::write(&path, wl_content).context("Failed to write WL manifest")?;
        
        Ok(path)
    }
}
