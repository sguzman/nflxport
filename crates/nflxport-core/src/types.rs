use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Team {
    pub team_abbr: String,
    pub team_name: String,
    pub team_id: String,
    pub team_conf: String,
    pub team_division: String,
    pub team_color: Option<String>,
    pub team_logo_espn: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Game {
    pub game_id: String,
    pub season: i32,
    pub game_type: String,
    pub week: i32,
    pub gameday: String,
    pub home_team: String,
    pub away_team: String,
    pub home_score: Option<i32>,
    pub away_score: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Player {
    pub gsis_id: String,
    pub display_name: String,
    pub position: String,
    pub current_team_id: Option<String>,
}
