use anyhow::{Context, Result};
use polars::prelude::*;
use polars::prelude::JsonWriter;
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

    pub fn generate_manifest_wl(&self, datasets: &[Dataset], standalone: bool) -> Result<Utf8PathBuf> {
        let mut wl_content = String::new();
        wl_content.push_str("BeginPackage[\"NFLXport`\"]\n\n");
        
        // Exported symbols
        wl_content.push_str("NFLTeams::usage = \"NFLTeams[] returns the teams dataset.\";\n");
        wl_content.push_str("NFLSchedules::usage = \"NFLSchedules[] returns the schedules dataset.\";\n");
        wl_content.push_str("NFLPlayers::usage = \"NFLPlayers[] returns the players dataset.\";\n");
        wl_content.push_str("NFLPlayerStats::usage = \"NFLPlayerStats[] returns the player stats dataset.\";\n");
        wl_content.push_str("NFLTeamStats::usage = \"NFLTeamStats[] returns the team stats dataset.\";\n");
        wl_content.push_str("NFLPBP::usage = \"NFLPBP[year] returns the play-by-play dataset for a specific year.\";\n");
        wl_content.push_str("NFLTeam::usage = \"NFLTeam[abbr] returns data for a specific team.\";\n");
        wl_content.push_str("NFLPlayer::usage = \"NFLPlayer[id] returns data for a specific player by GSIS ID.\";\n");
        wl_content.push_str("NFLGame::usage = \"NFLGame[id] returns data for a specific game by game ID.\";\n");
        wl_content.push_str("NFLSeason::usage = \"NFLSeason[year] returns a summary of a specific season.\";\n");
        wl_content.push_str("NFLPlayerSearch::usage = \"NFLPlayerSearch[name] searches for players by name.\";\n");
        wl_content.push_str("NFLGamesByTeam::usage = \"NFLGamesByTeam[abbr] returns all games for a specific team.\";\n\n");

        wl_content.push_str("Begin[\"`Private`\"]\n\n");
        
        if !standalone {
            let abs_export_dir = fs::canonicalize(&self.export_dir)
                .unwrap_or_else(|_| self.export_dir.as_std_path().to_path_buf());
            wl_content.push_str(&format!("NFLXportDataDirectory = \"{}\";\n\n", abs_export_dir.display().to_string().replace("\\", "/")));
            
            wl_content.push_str("NFLXportImportCSV[name_String] := Module[{path = FileNameJoin[{NFLXportDataDirectory, name <> \".csv\"}]},\n");
            wl_content.push_str("  If[FileExistsQ[path],\n");
            wl_content.push_str("    Import[path, \"Dataset\", \"HeaderLines\" -> 1],\n");
            wl_content.push_str("    $Failed\n");
            wl_content.push_str("  ]\n");
            wl_content.push_str("];\n\n");

            wl_content.push_str("NFLPBP[year_Integer] := NFLPBP[year] = NFLXportImportCSV[\"pbp_\" <> ToString[year]];\n\n");
        } else {
            // Default PBP if missing
            wl_content.push_str("NFLPBP[year_Integer] := $Failed;\n\n");
        }

        // Base datasets
        for ds in datasets {
            let sym_name = match ds {
                Dataset::Teams => "NFLTeams",
                Dataset::Schedules => "NFLSchedules",
                Dataset::Players => "NFLPlayers",
                Dataset::PlayerStats => "NFLPlayerStats",
                Dataset::TeamStats => "NFLTeamStats",
                Dataset::Pbp(year) => {
                    if standalone {
                        // Embed PBP as a dynamic definition
                        match self.loader.load(ds.clone()) {
                            Ok(mut df) => {
                                let mut buf = Vec::new();
                                JsonWriter::new(&mut buf).finish(&mut df)?;
                                let json = String::from_utf8(buf)?;
                                let escaped_json = escape_wolfram_string(&json);
                                wl_content.push_str(&format!("NFLPBP[{}] := NFLPBP[{}] = Dataset[ImportString[\"{}\", \"JSON\"]];\n", year, year, escaped_json));
                            },
                            Err(_) => {
                                tracing::warn!("Skipping PBP {} - not in cache", year);
                            }
                        }
                    }
                    continue;
                }
            };

            if standalone {
                match self.loader.load(ds.clone()) {
                    Ok(mut df) => {
                        let mut buf = Vec::new();
                        JsonWriter::new(&mut buf).finish(&mut df)?;
                        let json = String::from_utf8(buf)?;
                        let escaped_json = escape_wolfram_string(&json);
                        wl_content.push_str(&format!("{}[] := {}[] = Dataset[ImportString[\"{}\", \"JSON\"]];\n", sym_name, sym_name, escaped_json));
                    },
                    Err(_) => {
                        tracing::warn!("Skipping {} - not in cache", ds.cache_key());
                        wl_content.push_str(&format!("{}[] := $Failed;\n", sym_name));
                    }
                }
            } else {
                let name = ds.cache_key().replace("/", "_");
                wl_content.push_str(&format!("{}[] := {}[] = NFLXportImportCSV[\"{}\"];\n", sym_name, sym_name, name));
            }
        }

        // Higher-level helpers
        wl_content.push_str("\n(* Helpers *)\n");
        wl_content.push_str("NFLTeam[abbr_String] := With[{data = NFLTeams[]}, If[Head[data] === Dataset, data[SelectFirst[#[\"team_abbr\"] == abbr &]], $Failed]];\n");
        wl_content.push_str("NFLPlayer[id_String] := With[{data = NFLPlayers[]}, If[Head[data] === Dataset, data[SelectFirst[#[\"gsis_id\"] == id &]], $Failed]];\n");
        wl_content.push_str("NFLGame[id_String] := With[{data = NFLSchedules[]}, If[Head[data] === Dataset, data[SelectFirst[#[\"game_id\"] == id &]], $Failed]];\n");
        wl_content.push_str("NFLSeason[year_Integer] := <|\n");
        wl_content.push_str("  \"Year\" -> year,\n");
        wl_content.push_str("  \"Games\" -> With[{data = NFLSchedules[]}, If[Head[data] === Dataset, data[Select[#[\"season\"] == year &]], $Failed]],\n");
        wl_content.push_str("  \"PBP\" -> NFLPBP[year],\n");
        wl_content.push_str("  \"Stats\" -> With[{data = NFLPlayerStats[]}, If[Head[data] === Dataset, data[Select[#[\"season\"] == year &]], $Failed]],\n");
        wl_content.push_str("  \"QBLeaders\" -> With[{data = NFLPlayerStats[]}, If[Head[data] === Dataset, data[Select[#[\"season\"] == year && #[\"position\"] == \"QB\" &]][SortBy[#[\"passing_yards\"] &]][Reverse], $Failed]]\n");
        wl_content.push_str("|>;\n\n");
        wl_content.push_str("NFLPlayerSearch[name_String] := With[{data = NFLPlayers[]}, If[Head[data] === Dataset, data[Select[StringContainsQ[#[\"display_name\"], name, IgnoreCase -> True] &]], $Failed]];\n");
        wl_content.push_str("NFLGamesByTeam[abbr_String] := With[{data = NFLSchedules[]}, If[Head[data] === Dataset, data[Select[#[\"home_team\"] == abbr || #[\"away_team\"] == abbr &]], $Failed]];\n");
        
        wl_content.push_str("\nEnd[]\n");
        wl_content.push_str("EndPackage[]\n");

        if !self.export_dir.exists() {
            fs::create_dir_all(&self.export_dir).context("Failed to create export directory")?;
        }

        let path = self.export_dir.join("NFLXport.wl");
        fs::write(&path, wl_content).context("Failed to write WL manifest")?;
        
        Ok(path)
    }
}

fn escape_wolfram_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
