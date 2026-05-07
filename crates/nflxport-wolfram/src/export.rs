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
        wl_content.push_str("BeginPackage[\"NFLXport`\"]\n\n");
        
        // Exported symbols
        wl_content.push_str("NFLTeams::usage = \"NFLTeams[] returns the teams dataset.\";\n");
        wl_content.push_str("NFLSchedules::usage = \"NFLSchedules[] returns the schedules dataset.\";\n");
        wl_content.push_str("NFLPlayers::usage = \"NFLPlayers[] returns the players dataset.\";\n");
        wl_content.push_str("NFLPlayerStats::usage = \"NFLPlayerStats[] returns the player stats dataset.\";\n");
        wl_content.push_str("NFLPBP::usage = \"NFLPBP[year] returns the play-by-play dataset for a specific year.\";\n");
        wl_content.push_str("NFLTeam::usage = \"NFLTeam[abbr] returns data for a specific team.\";\n");
        wl_content.push_str("NFLPlayerSearch::usage = \"NFLPlayerSearch[name] searches for players by name.\";\n");
        wl_content.push_str("NFLGamesByTeam::usage = \"NFLGamesByTeam[abbr] returns all games for a specific team.\";\n\n");

        wl_content.push_str("Begin[\"`Private`\"]\n\n");
        
        wl_content.push_str(&format!("NFLXportDataDirectory = \"{}\";\n\n", self.export_dir));
        
        wl_content.push_str("NFLXportImportCSV[name_String] := Import[\n");
        wl_content.push_str("  FileNameJoin[{NFLXportDataDirectory, name <> \".csv\"}],\n");
        wl_content.push_str("  \"Dataset\",\n");
        wl_content.push_str("  \"HeaderLines\" -> 1\n");
        wl_content.push_str("];\n\n");

        // Base datasets
        for ds in datasets {
            let name = ds.cache_key().replace("/", "_");
            match ds {
                Dataset::Teams => wl_content.push_str(&format!("NFLTeams[] := NFLTeams[] = NFLXportImportCSV[\"{}\"];\n", name)),
                Dataset::Schedules => wl_content.push_str(&format!("NFLSchedules[] := NFLSchedules[] = NFLXportImportCSV[\"{}\"];\n", name)),
                Dataset::Players => wl_content.push_str(&format!("NFLPlayers[] := NFLPlayers[] = NFLXportImportCSV[\"{}\"];\n", name)),
                Dataset::PlayerStats => wl_content.push_str(&format!("NFLPlayerStats[] := NFLPlayerStats[] = NFLXportImportCSV[\"{}\"];\n", name)),
                Dataset::Pbp(year) => wl_content.push_str(&format!("NFLPBP[{}] := NFLPBP[{}] = NFLXportImportCSV[\"{}\"];\n", year, year, name)),
                Dataset::TeamStats => wl_content.push_str(&format!("NFLTeamStats[] := NFLTeamStats[] = NFLXportImportCSV[\"{}\"];\n", name)),
            }
        }

        // Higher-level helpers
        wl_content.push_str("\n(* Helpers *)\n");
        wl_content.push_str("NFLTeam[abbr_String] := NFLTeams[][SelectFirst[#team_abbr == abbr &]];\n");
        wl_content.push_str("NFLPlayerSearch[name_String] := NFLPlayers[][Select[StringContainsQ[#display_name, name, IgnoreCase -> True] &]];\n");
        wl_content.push_str("NFLGamesByTeam[abbr_String] := NFLSchedules[][Select[#home_team == abbr || #away_team == abbr &]];\n");
        
        wl_content.push_str("\nEnd[]\n");
        wl_content.push_str("EndPackage[]\n");

        let path = self.export_dir.join("NFLXport.wl");
        fs::write(&path, wl_content).context("Failed to write WL manifest")?;
        
        Ok(path)
    }
}
