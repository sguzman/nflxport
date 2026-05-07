#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dataset {
    Teams,
    Schedules,
    Players,
    Pbp(i32),
    PlayerStats,
    TeamStats,
}

impl Dataset {
    pub fn url(&self) -> String {
        let base = "https://github.com/nflverse/nflverse-data/releases/download";
        match self {
            Dataset::Teams => format!("{}/teams/teams_colors_logos.parquet", base),
            Dataset::Schedules => format!("{}/schedules/games.parquet", base),
            Dataset::Players => format!("{}/players/players.parquet", base),
            Dataset::Pbp(year) => format!("{}/pbp/play_by_play_{}.parquet", base, year),
            Dataset::PlayerStats => format!("{}/player_stats/player_stats.parquet", base),
            Dataset::TeamStats => format!("{}/team_stats/team_stats.parquet", base),
        }
    }

    pub fn cache_key(&self) -> String {
        match self {
            Dataset::Teams => "teams".to_string(),
            Dataset::Schedules => "schedules".to_string(),
            Dataset::Players => "players".to_string(),
            Dataset::Pbp(year) => format!("pbp/{}", year),
            Dataset::PlayerStats => "player_stats".to_string(),
            Dataset::TeamStats => "team_stats".to_string(),
        }
    }

    pub fn relative_path(&self) -> String {
        match self {
            Dataset::Teams => "teams/teams.parquet".to_string(),
            Dataset::Schedules => "schedules/games.parquet".to_string(),
            Dataset::Players => "players/players.parquet".to_string(),
            Dataset::Pbp(year) => format!("pbp/{}.parquet", year),
            Dataset::PlayerStats => "player_stats/player_stats.parquet".to_string(),
            Dataset::TeamStats => "team_stats/team_stats.parquet".to_string(),
        }
    }
}
