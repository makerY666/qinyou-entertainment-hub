//! Bundled game registry. Add another module here in an application release.
use serde::Serialize;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDefinition {
    pub game_id: &'static str,
    pub rules_version: &'static str,
    pub name: &'static str,
    pub seats: u8,
    pub supports_bots: bool,
}
pub const MAHJONG_ID: &str = "sichuan-blood-battle";
pub const RULES_VERSION: &str = "family-multiplier-v1";
pub fn games() -> Vec<GameDefinition> {
    vec![
        GameDefinition {
            game_id: guandan::GAME_ID,
            rules_version: guandan::RULES_VERSION,
            name: "掼蛋 · 四人组队",
            seats: 4,
            supports_bots: true,
        },
        GameDefinition {
            game_id: MAHJONG_ID,
            rules_version: RULES_VERSION,
            name: "四川麻将 · 血战到底",
            seats: 4,
            supports_bots: true,
        },
        GameDefinition {
            game_id: doudizhu::GAME_ID,
            rules_version: doudizhu::RULES_VERSION,
            name: "斗地主 · 三人叫分",
            seats: 3,
            supports_bots: true,
        },
    ]
}
