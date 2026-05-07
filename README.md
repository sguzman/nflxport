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

### Mathematica Export

1. Export data:

   ```bash
   nflx export wolfram
   ```

2. Install the manifest:

   ```bash
   nflx install wolfram
   ```

3. In Mathematica, load the data:

   ```mathematica
   Get["NFLXport.wl"]
   NFLTeams[] // TableForm
   ```

## Project Structure

- `crates/nflxport-core`: Core logic, data fetching, and query engine.
- `crates/nflxport-cli`: CLI interface (`nflx`).
- `crates/nflxport-wolfram`: Mathematica bridge.

## License

CC0-1.0
