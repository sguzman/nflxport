# nflxport

A high-performance Rust toolkit for working with `nflverse` data.

## Features

- **Blazing Fast**: Powered by Polars and Rust.
- **Idempotent Caching**: Efficiently manage large Parquet datasets under `.cache/nflxport`.
- **Analytical Query Engine**: Perform statistical queries directly from the CLI.
- **Wolfram Mathematica Bridge**: Seamlessly export data for advanced symbolic analysis.

## Installation

```bash
cargo build --release
```

The CLI binary is located at `target/release/nflx`.

## Usage

### Fetching Data

```bash
nflx fetch stats
nflx fetch pbp --season 2023
```

### Analytical Queries

#### Statistical Leaders

```bash
nflx stats leaders passing_yards --limit 5
```

#### Team Summary

```bash
nflx stats team-summary KC
```

#### Player Search

```bash
nflx stats player-search Mahomes
```

### Analytical Database (DuckDB)

Nflxport includes a built-in DuckDB engine for high-performance SQL queries.

#### Building the Database

Ingest cached Parquet files into the local DuckDB instance:

```bash
nflx db build
```

#### Running SQL Queries

Execute arbitrary SQL queries against the local database:

```bash
nflx db query "SELECT team_abbr, team_name FROM teams LIMIT 5"
```

Perform complex multi-table joins:

```bash
nflx db query "SELECT p.posteam, t.team_name, count(*) as play_count \
FROM pbp_2023 p JOIN teams t ON p.posteam = t.team_abbr \
GROUP BY ALL ORDER BY play_count DESC LIMIT 5"
```

### Mathematica Export

1. Export data (optionally for a specific season):

   ```bash
   nflx export wolfram --season 2023
   ```

2. Install the manifest to your Mathematica application folder:

   ```bash
   nflx install wolfram
   ```

3. In Mathematica, load the package and use high-level symbolic helpers:

   ```mathematica
   Needs["NFLXport`"]

   (* Get data for a specific team *)
   NFLTeam["KC"]

   (* Search for players *)
   NFLPlayerSearch["Mahomes"]

   (* Access season summaries *)
   season = NFLSeason[2023]
   season["QBLeaders"]
   ```

## Project Structure

- `crates/nflxport-core`: Core logic, data fetching, and query engine.
- `crates/nflxport-cli`: CLI interface (`nflx`).
- `crates/nflxport-wolfram`: Mathematica bridge.

## License

CC0-1.0
