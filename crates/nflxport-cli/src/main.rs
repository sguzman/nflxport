use anyhow::{Context, Result};
use std::fs;
use clap::{Parser, Subcommand};
use nflxport_core::{Cache, Fetcher, Dataset, DatabaseManager};
use camino::Utf8PathBuf;

#[derive(Parser)]
#[command(name = "nflx")]
#[command(version, about = "nflxport CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, default_value = ".cache/nflxport")]
    cache_dir: Utf8PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch a dataset
    Fetch {
        /// Dataset to fetch (teams, schedules, players, pbp, stats)
        dataset: String,
        
        /// Season year (for pbp)
        #[arg(short, long)]
        season: Option<i32>,
        
        /// Force re-download
        #[arg(short, long)]
        force: bool,
    },
    /// Manage the cache
    Cache {
        #[command(subcommand)]
        command: CacheCommands,
    },
    /// Inspect a dataset
    Inspect {
        #[command(subcommand)]
        command: InspectCommands,
    },
    /// Export data to external formats
    Export {
        #[command(subcommand)]
        command: ExportCommands,
    },
    /// Install components
    Install {
        #[command(subcommand)]
        command: InstallCommands,
    },
    /// Perform analytical queries
    Stats {
        #[command(subcommand)]
        command: StatsCommands,
    },
    /// Analytical database commands
    Db {
        #[command(subcommand)]
        command: DbCommands,
    },
}

#[derive(Subcommand)]
enum DbCommands {
    /// Build DuckDB database from cached parquet files
    Build {
        /// Datasets to ingest (comma separated or "all")
        #[arg(short, long, default_value = "all")]
        datasets: String,
        /// Season year for PBP data
        #[arg(short, long)]
        season: Option<i32>,
    },
    /// Run a SQL query against the local database
    Query {
        /// SQL query string
        sql: String,
    },
}

#[derive(Subcommand)]
enum StatsCommands {
    /// Show statistical leaders
    Leaders {
        /// Category (e.g. passing_yards, rushing_yards, receptions)
        category: String,
        /// Number of players to show
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
    },
    /// Show summary for a specific team
    TeamSummary {
        /// Team abbreviation (e.g. KC, SF, PHI)
        team: String,
    },
    /// Search for a player by name
    PlayerSearch {
        /// Player name fragment
        name: String,
    },
}

#[derive(Subcommand)]
enum InstallCommands {
    /// Install Wolfram Language package to Mathematica Applications folder
    Wolfram {
        /// Custom installation path
        #[arg(short, long)]
        path: Option<String>,
    },
}

#[derive(Subcommand)]
enum ExportCommands {
    /// Export datasets to CSV and generate Wolfram Language manifest
    Wolfram {
        /// Datasets to export (comma separated or "all")
        #[arg(short, long, default_value = "all")]
        datasets: String,
        /// Export directory
        #[arg(short = 'o', long, default_value = "exports")]
        dir: String,
        /// Season for PBP data
        #[arg(short, long)]
        season: Option<i32>,
    },
}

#[derive(Subcommand)]
enum InspectCommands {
    /// Show schema of a dataset
    Schema {
        dataset: String,
        #[arg(short, long)]
        season: Option<i32>,
    },
    /// Show first N rows of a dataset
    Head {
        dataset: String,
        #[arg(short, long)]
        season: Option<i32>,
        #[arg(short, long, default_value_t = 10)]
        rows: usize,
    },
}

#[derive(Subcommand)]
enum CacheCommands {
    /// Show status of cached datasets
    Status,
}

fn main() -> Result<()> {
    // Initialize dual-layer tracing
    use tracing_subscriber::{fmt, prelude::*, Registry};
    
    let file_appender = tracing_appender::rolling::never("logs", "nflxport.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    
    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false);
    
    let stdout_layer = fmt::layer()
        .with_target(false);
        
    Registry::default()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "nflxport=info,nflx=info".into()))
        .with(stdout_layer)
        .with(file_layer)
        .init();
    
    let cli = Cli::parse();
    tracing::info!("Starting nflx CLI with cache at: {}", cli.cache_dir);
    
    let cache = Cache::new(cli.cache_dir.clone())?;
    let fetcher = Fetcher::new(cache.clone());

    match cli.command {
        Commands::Fetch { dataset, season, force } => {
            let ds = parse_dataset(&dataset, season)?;
            fetcher.fetch(ds, force)?;
        },
        Commands::Cache { command } => match command {
            // ... (keep existing cache status logic)
            CacheCommands::Status => {
                let manifest = cache.load_manifest()?;
                println!("{:<20} {:<10} {:<20} {:<10}", "Dataset", "Status", "Downloaded At", "Size");
                println!("{}", "-".repeat(65));
                for (key, entry) in manifest.datasets {
                    println!("{:<20} {:<10?} {:<20} {:<10}", 
                        key, 
                        entry.status, 
                        entry.downloaded_at.to_rfc3339(),
                        format!("{} MB", entry.bytes / 1024 / 1024)
                    );
                }
            }
        },
        Commands::Inspect { command } => {
            let loader = nflxport_core::DatasetLoader::new(cache);
            match command {
                InspectCommands::Schema { dataset, season } => {
                    let ds = parse_dataset(&dataset, season)?;
                    let schema = loader.schema(ds)?;
                    println!("Schema for {}:", dataset);
                    for (name, dtype) in schema.iter() {
                        println!("  {:<30} {:?}", name, dtype);
                    }
                },
                InspectCommands::Head { dataset, season, rows } => {
                    let ds = parse_dataset(&dataset, season)?;
                    let df = loader.load(ds)?;
                    println!("{}", df.head(Some(rows)));
                }
            }
        },
        Commands::Export { command } => {
            match command {
                ExportCommands::Wolfram { datasets, dir, season } => {
                    let export_dir = Utf8PathBuf::from(dir);
                    let exporter = nflxport_wolfram::WolframExporter::new(cache.clone(), export_dir);
                    
                    let ds_to_export = if datasets == "all" {
                        let mut all = vec![Dataset::Teams, Dataset::Schedules, Dataset::Players, Dataset::PlayerStats, Dataset::TeamStats];
                        
                        // Scan cache for PBP seasons
                        let pbp_dir = cache.root.join("raw/pbp");
                        if let Ok(entries) = std::fs::read_dir(pbp_dir) {
                            for entry in entries.flatten() {
                                if let Some(name) = entry.file_name().to_str() {
                                    if name.ends_with(".parquet") {
                                        if let Ok(year) = name.trim_end_matches(".parquet").parse::<i32>() {
                                            all.push(Dataset::Pbp(year));
                                        }
                                    }
                                }
                            }
                        }
                        
                        // If a specific season was requested, ensure it's included (even if not cached yet, though exporter will skip)
                        if let Some(s) = season {
                            if !all.iter().any(|d| matches!(d, Dataset::Pbp(y) if *y == s)) {
                                all.push(Dataset::Pbp(s));
                            }
                        }
                        all
                    } else {
                        let mut list = Vec::new();
                        for name in datasets.split(',') {
                            list.push(parse_dataset(name.trim(), season)?);
                        }
                        list
                    };

                    println!("Exporting datasets to {}...", exporter_dir_path(&exporter));
                    for ds in &ds_to_export {
                        print!("  Exporting {:<20} ... ", ds.cache_key());
                        match exporter.export_csv(ds.clone()) {
                            Ok(path) => println!("Ok ({})", path.file_name().unwrap()),
                            Err(e) => println!("Error: {}", e),
                        }
                    }

                    let manifest_path = exporter.generate_manifest_wl(&ds_to_export)?;
                    println!("\nGenerated Wolfram Language manifest: {}", manifest_path);
                    println!("To load in Mathematica, run: Get[\"{}\"]", manifest_path);
                }
            }
        },
        Commands::Install { command } => {
            match command {
                InstallCommands::Wolfram { path } => {
                    let source = Utf8PathBuf::from("exports/NFLXport.wl");
                    if !source.exists() {
                        anyhow::bail!("NFLXport.wl not found in exports/. Run 'export wolfram' first.");
                    }

                    let dest_dir = if let Some(p) = path {
                        Utf8PathBuf::from(p)
                    } else {
                        // Default Linux path
                        let home = std::env::var("HOME").context("HOME not set")?;
                        Utf8PathBuf::from(home).join(".Mathematica/Applications/NFLXport")
                    };

                    if !dest_dir.exists() {
                        fs::create_dir_all(&dest_dir).context("Failed to create installation directory")?;
                    }

                    let dest = dest_dir.join("NFLXport.wl");
                    fs::copy(&source, &dest).context("Failed to copy manifest")?;
                    println!("Successfully installed NFLXport.wl to {}", dest);
                }
            }
        },
        Commands::Stats { command } => {
            let query_engine = nflxport_core::query::StatsQuery::new(cache.clone());
            match command {
                StatsCommands::Leaders { category, limit } => {
                    let df = query_engine.leaders(&category, limit)?;
                    println!("{}", df);
                }
                StatsCommands::TeamSummary { team } => {
                    let df = query_engine.team_summary(&team)?;
                    println!("{}", df);
                }
                StatsCommands::PlayerSearch { name } => {
                    let df = query_engine.player_search(&name)?;
                    println!("{}", df);
                }
            }
        }
        Commands::Db { command } => {
            let db = DatabaseManager::new(cache.clone())?;
            match command {
                DbCommands::Build { datasets, season } => {
                    let ds_list = if datasets == "all" {
                        let mut all = vec![
                            Dataset::Teams,
                            Dataset::Players,
                            Dataset::Schedules,
                            Dataset::PlayerStats,
                            Dataset::TeamStats,
                        ];

                        // Scan cache for PBP seasons
                        let pbp_dir = cache.root.join("raw/pbp");
                        if let Ok(entries) = std::fs::read_dir(pbp_dir) {
                            for entry in entries.flatten() {
                                if let Some(name) = entry.file_name().to_str() {
                                    if name.ends_with(".parquet") {
                                        if let Ok(year) = name.trim_end_matches(".parquet").parse::<i32>() {
                                            all.push(Dataset::Pbp(year));
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(s) = season {
                            if !all.iter().any(|d| matches!(d, Dataset::Pbp(y) if *y == s)) {
                                all.push(Dataset::Pbp(s));
                            }
                        }
                        all
                    } else {
                        let s = season.unwrap_or(2023);
                        datasets.split(',')
                            .filter_map(|s_name| match s_name.trim() {
                                "teams" => Some(Dataset::Teams),
                                "players" => Some(Dataset::Players),
                                "schedules" => Some(Dataset::Schedules),
                                "pstats" => Some(Dataset::PlayerStats),
                                "tstats" => Some(Dataset::TeamStats),
                                "pbp" => Some(Dataset::Pbp(s)),
                                _ => None,
                            })
                            .collect()
                    };
                    db.build_from_cache(&ds_list)?;
                    println!("Successfully built database at {}", cache.root.join("nflxport.db"));
                }
                DbCommands::Query { sql } => {
                    let results = db.query(&sql)?;
                    for row in results {
                        println!("{}", row.join(" | "));
                    }
                }
            }
        }
    }

    Ok(())
}

fn exporter_dir_path(_exporter: &nflxport_wolfram::WolframExporter) -> &str {
    // This is a hack because we don't expose export_dir publicly 
    // but we can just use the one passed in. 
    "exports" // default for now
}

fn parse_dataset(name: &str, season: Option<i32>) -> Result<Dataset> {
    match name {
        "teams" => Ok(Dataset::Teams),
        "schedules" => Ok(Dataset::Schedules),
        "players" => Ok(Dataset::Players),
        "pbp" => {
            let s = season.context("Season is required for pbp")?;
            Ok(Dataset::Pbp(s))
        },
        "stats" => Ok(Dataset::PlayerStats),
        _ => anyhow::bail!("Unknown dataset: {}", name),
    }
}
