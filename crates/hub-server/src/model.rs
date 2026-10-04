use crate::game::RoomGame;
use mahjong::RoomConfig;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSession {
    pub player_id: String,
    pub name: String,
    pub avatar: String,
    #[serde(skip_serializing)]
    pub token: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Seat {
    pub player_id: String,
    pub name: String,
    pub avatar: String,
    pub bot: bool,
    pub ready: bool,
    pub score: i64,
    pub offline_since: Option<i64>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Standing {
    pub player_id: String,
    pub name: String,
    pub score: i64,
    pub win_count: u32,
    pub self_draw_count: u32,
    pub discard_win_count: u32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    pub id: String,
    pub name: String,
    pub game_id: String,
    pub owner_id: String,
    pub invite: String,
    pub version: u64,
    pub round: u32,
    pub config: RoomConfig,
    pub seats: Vec<Option<Seat>>,
    pub game: Option<RoomGame>,
    pub paused: bool,
    pub archived: bool,
    pub created_at: i64,
    pub round_started_at: Option<i64>,
    pub deadline: Option<i64>,
    pub next_bot_at: i64,
    pub history_id: Option<String>,
    pub accounted_ledger: usize,
    #[serde(default)]
    pub accounted_wins: usize,
    #[serde(default)]
    pub totals: HashMap<String, Standing>,
    #[serde(default)]
    pub round_players: Vec<Option<String>>,
    #[serde(default)]
    pub settlement_confirmed: Option<Vec<bool>>,
    // Stable identity of everyone who took part; allows history after leaving a seat.
    pub participants: HashSet<String>,
}
impl Room {
    pub fn seat_of(&self, id: &str) -> Option<usize> {
        self.seats
            .iter()
            .position(|s| s.as_ref().is_some_and(|s| s.player_id == id))
    }
    pub fn running(&self) -> bool {
        self.game.as_ref().is_some_and(RoomGame::running)
    }
    pub fn status(&self) -> &'static str {
        if self.archived {
            "archived"
        } else if self.running() {
            "playing"
        } else if self.game.is_some() {
            "finished"
        } else {
            "waiting"
        }
    }
}

pub struct Store {
    pub db: rusqlite::Connection,
    pub rooms: HashMap<String, Room>,
    pub sessions: HashMap<String, PlayerSession>,
    pub connections: HashMap<(String, String), usize>,
    pub admin_token: String,
}

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
pub fn secret() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}
