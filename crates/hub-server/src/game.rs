//! Dispatch room lifecycle operations while retaining the old Mahjong save format.
use mahjong::{Game, Phase, RoomConfig, ScoreEntry};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Serialize)]
#[serde(untagged)]
pub enum RoomGame {
    Mahjong(Game),
    Doudizhu(doudizhu::Game),
    Guandan(guandan::Game),
}
// Serde's untagged Content buffer cannot decode JSON string keys into the
// integer keys in Mahjong's pending.responses. Deserialize through Value so
// saved mid-response frames remain readable, including pre-Hub Mahjong saves.
impl<'de> Deserialize<'de> for RoomGame {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        if value.get("levels").is_some() {
            serde_json::from_value(value)
                .map(Self::Guandan)
                .map_err(serde::de::Error::custom)
        } else if value.get("bids").is_some() {
            serde_json::from_value(value)
                .map(Self::Doudizhu)
                .map_err(serde::de::Error::custom)
        } else {
            serde_json::from_value(value)
                .map(Self::Mahjong)
                .map_err(serde::de::Error::custom)
        }
    }
}
impl RoomGame {
    pub fn new(
        game_id: &str,
        config: RoomConfig,
        seed: u64,
        previous: Option<&Self>,
    ) -> Result<Self, String> {
        if game_id == guandan::GAME_ID {
            guandan::Game::new(
                guandan::Config {
                    base_score: config.base_score,
                    cap: config.cap,
                    turn_seconds: config.turn_seconds,
                },
                seed,
                match previous {
                    Some(Self::Guandan(g)) => Some(g),
                    _ => None,
                },
            )
            .map(Self::Guandan)
        } else if game_id == doudizhu::GAME_ID {
            let dealer = match previous {
                Some(Self::Doudizhu(g)) => (g.dealer + 1) % 3,
                _ => (seed % 3) as usize,
            };
            doudizhu::Game::new(
                doudizhu::Config {
                    base_score: config.base_score,
                    cap: config.cap,
                    turn_seconds: config.turn_seconds,
                },
                seed,
                dealer,
            )
            .map(Self::Doudizhu)
        } else {
            let result = match previous {
                Some(Self::Mahjong(g)) => Game::new_with_dealer(config, seed, g.next_dealer),
                _ => Game::new(config, seed),
            };
            result.map(Self::Mahjong).map_err(|e| e.to_string())
        }
    }
    pub fn running(&self) -> bool {
        match self {
            Self::Mahjong(g) => !matches!(g.phase, Phase::Finished | Phase::Aborted),
            Self::Doudizhu(g) => g.running(),
            Self::Guandan(g) => g.running(),
        }
    }
    pub fn version(&self) -> u64 {
        match self {
            Self::Mahjong(g) => g.version,
            Self::Doudizhu(g) => g.version,
            Self::Guandan(g) => g.version,
        }
    }
    pub fn view(&self, seat: usize) -> Value {
        match self {
            Self::Mahjong(g) => json!(g.view(seat)),
            Self::Doudizhu(g) => g.view(seat),
            Self::Guandan(g) => g.view(seat),
        }
    }
    pub fn abort(&mut self) {
        match self {
            Self::Mahjong(g) => g.abort(),
            Self::Doudizhu(g) => g.abort(),
            Self::Guandan(g) => g.abort(),
        }
    }
    pub fn seconds(&self) -> u32 {
        match self {
            Self::Mahjong(g) => {
                if g.phase == Phase::Responding {
                    g.config.response_seconds
                } else {
                    g.config.turn_seconds
                }
            }
            Self::Doudizhu(g) => g.config.turn_seconds,
            Self::Guandan(g) => g.config.turn_seconds,
        }
    }
    pub fn acting_seats(&self) -> Vec<usize> {
        match self {
            Self::Guandan(g) => {
                if g.running() {
                    vec![g.turn]
                } else {
                    vec![]
                }
            }
            Self::Mahjong(g) => g.acting_seats(),
            Self::Doudizhu(g) => {
                if g.running() {
                    vec![g.turn]
                } else {
                    vec![]
                }
            }
        }
    }
    pub fn bot_action(&self, seat: usize, timed_out: bool) -> Option<Value> {
        match self {
            Self::Mahjong(g) => {
                if timed_out && g.phase == Phase::Responding {
                    Some(json!(mahjong::Action::Pass))
                } else {
                    g.bot_action(seat).map(|a| json!(a))
                }
            }
            Self::Doudizhu(g) => g.bot_action(seat).map(|a| json!(a)),
            Self::Guandan(g) => g.bot_action(seat).map(|a| json!(a)),
        }
    }
    /// True if the shared turn deadline should be renewed.
    pub fn apply(&mut self, seat: usize, action: Value) -> Result<bool, String> {
        match self {
            Self::Guandan(g) => {
                let action = serde_json::from_value(action).map_err(|_| "掼蛋操作不合法")?;
                g.apply(seat, action)?;
                Ok(true)
            }
            Self::Mahjong(g) => {
                let action = serde_json::from_value(action).map_err(|_| "麻将操作不合法")?;
                let (phase, turn) = (g.phase, g.turn);
                g.apply(seat, action).map_err(|e| e.to_string())?;
                Ok(g.phase != phase
                    || g.turn != turn
                    || !matches!(g.phase, Phase::Responding | Phase::DingQue))
            }
            Self::Doudizhu(g) => {
                let action = serde_json::from_value(action).map_err(|_| "斗地主操作不合法")?;
                g.apply(seat, action)?;
                Ok(true)
            }
        }
    }
    pub fn ledger(&self) -> Vec<ScoreEntry> {
        match self {
            Self::Guandan(g) => g
                .ledger
                .iter()
                .map(|e| ScoreEntry {
                    from: e.from,
                    to: e.to,
                    amount: e.amount,
                    multiplier: e.multiplier,
                    reason: e.reason.clone(),
                    event_id: e.event_id,
                })
                .collect(),
            Self::Mahjong(g) => g.ledger.clone(),
            Self::Doudizhu(g) => g
                .ledger
                .iter()
                .map(|e| ScoreEntry {
                    from: e.from,
                    to: e.to,
                    amount: e.amount,
                    multiplier: e.multiplier,
                    reason: e.reason.clone(),
                    event_id: e.event_id,
                })
                .collect(),
        }
    }
    pub fn winners(&self) -> &[usize] {
        match self {
            Self::Mahjong(g) => &g.winners,
            Self::Doudizhu(g) => &g.winners,
            Self::Guandan(g) => &g.winners,
        }
    }
    pub fn self_draw(&self, seat: usize) -> bool {
        matches!(self, Self::Mahjong(g) if g.players[seat].win.as_ref().is_some_and(|w|w.from.is_none()))
    }
    pub fn discard_wins(&self, skip: usize, end: usize) -> Vec<(usize, u64)> {
        match self {
            Self::Mahjong(g) => g
                .winners
                .iter()
                .take(end)
                .skip(skip)
                .filter_map(|&i| g.players[i].win.as_ref())
                .filter_map(|w| w.from.map(|i| (i, w.source_event_id)))
                .collect(),
            _ => vec![],
        }
    }
    pub fn player_summary(&self, seat: usize) -> Value {
        match self {
            Self::Guandan(g) => {
                json!({"score":g.players[seat].score,"winCount":usize::from(g.winners.contains(&seat)),"selfDrawCount":0,"discardWinCount":0})
            }
            Self::Mahjong(g) => {
                let p = &g.players[seat];
                json!({"score":p.score,"winCount":usize::from(p.won),"selfDrawCount":usize::from(self.self_draw(seat)),"discardWinCount":self.discard_wins(0, usize::MAX).into_iter().filter(|(i,_)|*i==seat).collect::<std::collections::HashSet<_>>().len()})
            }
            Self::Doudizhu(g) => {
                json!({"score":g.players[seat].score,"winCount":usize::from(g.winners.contains(&seat)),"selfDrawCount":0,"discardWinCount":0})
            }
        }
    }
    pub fn replay(&self, seat: usize) -> (Value, Value) {
        match self {
            Self::Guandan(g) => {
                let mut v = g.view(seat);
                v["legal_actions"] = json!([]);
                for i in 0..4 {
                    v["players"][i]["hand"] = json!(g.players[i].hand);
                }
                (
                    v,
                    json!(g.players.iter().map(|p| &p.hand).collect::<Vec<_>>()),
                )
            }
            Self::Mahjong(g) => {
                let mut v = g.view(seat);
                for (i, p) in v.players.iter_mut().enumerate() {
                    p.hand = Some(g.players[i].hand.clone());
                    for (j, m) in p.melds.iter_mut().enumerate() {
                        m.tile = Some(g.players[i].melds[j].tile);
                    }
                }
                v.legal_actions.clear();
                (
                    json!(v),
                    json!(g.players.iter().map(|p| &p.hand).collect::<Vec<_>>()),
                )
            }
            Self::Doudizhu(g) => {
                let mut v = g.view(seat);
                v["legal_actions"] = json!([]);
                for i in 0..3 {
                    v["players"][i]["hand"] = json!(g.players[i].hand);
                }
                (
                    v,
                    json!(g.players.iter().map(|p| &p.hand).collect::<Vec<_>>()),
                )
            }
        }
    }
    pub fn config(&self) -> Value {
        match self {
            Self::Mahjong(g) => json!(g.config),
            Self::Doudizhu(g) => json!(g.config),
            Self::Guandan(g) => json!(g.config),
        }
    }
    pub fn phase(&self) -> Value {
        match self {
            Self::Mahjong(g) => json!(g.phase),
            Self::Doudizhu(g) => json!(g.phase),
            Self::Guandan(g) => json!(g.phase),
        }
    }
    pub fn rules_version(&self) -> &str {
        match self {
            Self::Mahjong(g) => &g.rules_version,
            Self::Doudizhu(g) => &g.rules_version,
            Self::Guandan(g) => &g.rules_version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_mahjong_numeric_response_keys_roundtrip() {
        let mut game = Game::new(RoomConfig::default(), 42).unwrap();
        game.phase = Phase::Responding;
        game.pending = Some(mahjong::Pending {
            kind: mahjong::PendingKind::Discard,
            from: 0,
            tile: 3,
            after_kong: false,
            eligible: vec![1, 2],
            responses: std::collections::BTreeMap::from([(1, mahjong::Action::Pass)]),
        });
        let legacy = serde_json::to_string(&game).unwrap();
        let restored: RoomGame = serde_json::from_str(&legacy).unwrap();
        assert!(matches!(&restored, RoomGame::Mahjong(_)));
        assert_eq!(
            serde_json::to_value(game).unwrap(),
            serde_json::to_value(restored).unwrap()
        );
    }
}
