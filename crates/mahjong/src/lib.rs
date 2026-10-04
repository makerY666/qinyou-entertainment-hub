//! Deterministic, server-authoritative Sichuan blood-battle mahjong.
//! `Game` is private server state. Only `Game::view` may be sent during play.
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

pub const GAME_ID: &str = "sichuan-blood-battle";
pub const RULES_VERSION: &str = "family-multiplier-v1";
/// 0..=8 = 万; 9..=17 = 条; 18..=26 = 筒.
pub type Tile = u8;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RoomConfig {
    pub base_score: i64,
    pub cap: Option<u32>,
    pub flower_penalty: u32,
    pub turn_seconds: u32,
    pub response_seconds: u32,
}

mod bot;
#[cfg(test)]
mod tests;
impl Default for RoomConfig {
    fn default() -> Self {
        Self {
            base_score: 1,
            cap: None,
            flower_penalty: 16,
            turn_seconds: 30,
            response_seconds: 15,
        }
    }
}
impl RoomConfig {
    pub fn validate(&self) -> Result<(), GameError> {
        if !(1..=1_000_000_000).contains(&self.base_score)
            || !matches!(self.cap, None | Some(8 | 16 | 32 | 64))
            || !matches!(self.flower_penalty, 8 | 16 | 32)
            || self.turn_seconds > 3600
            || self.response_seconds > 3600
        {
            return Err(GameError::InvalidConfig);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    DingQue,
    Playing,
    Responding,
    Finished,
    Aborted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    DingQue { suit: u8 },
    Discard { tile: Tile },
    Pass,
    Peng,
    Kong { tile: Tile },
    Hu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeldKind {
    Peng,
    ExposedKong,
    ConcealedKong,
    SupplementalKong,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meld {
    pub tile: Tile,
    pub kind: MeldKind,
    pub from: Option<usize>,
}
impl Meld {
    pub fn len(&self) -> usize {
        if self.kind == MeldKind::Peng {
            3
        } else {
            4
        }
    }
    pub fn is_empty(&self) -> bool {
        false
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WinRecord {
    pub tile: Tile,
    pub from: Option<usize>,
    /// Source discard or supplemental-kong offer; shared by simultaneous winners.
    pub source_event_id: u64,
    pub value: HandValue,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Player {
    pub hand: Vec<Tile>,
    pub melds: Vec<Meld>,
    pub missing_suit: Option<u8>,
    pub won: bool,
    pub score: i64,
    pub win: Option<WinRecord>,
    pub last_draw: Option<Tile>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Discard {
    pub seat: usize,
    pub tile: Tile,
    pub claimed_by: Option<usize>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PendingKind {
    Discard,
    SupplementalKong,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pending {
    pub kind: PendingKind,
    pub from: usize,
    pub tile: Tile,
    pub after_kong: bool,
    pub eligible: Vec<usize>,
    pub responses: BTreeMap<usize, Action>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoreEntry {
    pub from: usize,
    pub to: usize,
    pub reason: String,
    pub multiplier: u32,
    pub amount: i64,
    pub event_id: u64,
}
/// Public events deliberately exclude draws, concealed kong tiles and hand contents.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameEvent {
    pub seq: u64,
    pub kind: String,
    pub seat: Option<usize>,
    pub tile: Option<Tile>,
    pub message: String,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct WinContext {
    pub self_draw: bool,
    pub kong_event: bool,
    pub rob_kong: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HandValue {
    pub multiplier: u32,
    pub uncapped_multiplier: u32,
    pub labels: Vec<String>,
    pub roots: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub rules_version: String,
    pub config: RoomConfig,
    pub phase: Phase,
    pub turn: usize,
    pub dealer: usize,
    pub next_dealer: usize,
    pub version: u64,
    pub players: [Player; 4],
    pub wall: Vec<Tile>,
    pub discards: Vec<Discard>,
    pub pending: Option<Pending>,
    pub ledger: Vec<ScoreEntry>,
    pub events: Vec<GameEvent>,
    pub winners: Vec<usize>,
    pub current_after_kong: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublicMeld {
    pub tile: Option<Tile>,
    pub kind: MeldKind,
    pub from: Option<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublicPlayer {
    pub hand_count: usize,
    pub hand: Option<Vec<Tile>>,
    pub melds: Vec<PublicMeld>,
    pub missing_suit: Option<u8>,
    pub won: bool,
    pub score: i64,
    pub win: Option<WinRecord>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublicPending {
    pub kind: PendingKind,
    pub from: usize,
    pub tile: Tile,
    pub responded: Vec<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerView {
    pub game_id: String,
    pub rules_version: String,
    pub config: RoomConfig,
    pub version: u64,
    pub phase: Phase,
    pub turn: usize,
    pub dealer: usize,
    pub next_dealer: usize,
    pub self_seat: usize,
    pub own_hand: Vec<Tile>,
    /// Only the requesting player's draw; never included in public player data.
    #[serde(default)]
    pub own_last_draw: Option<Tile>,
    pub players: Vec<PublicPlayer>,
    pub discards: Vec<Discard>,
    pub pending: Option<PublicPending>,
    pub legal_actions: Vec<Action>,
    pub ledger: Vec<ScoreEntry>,
    pub events: Vec<GameEvent>,
    pub winners: Vec<usize>,
    pub wall_remaining: usize,
}
#[derive(Debug, Error, PartialEq, Eq)]
pub enum GameError {
    #[error("此牌局使用其他规则版本，当前版本只能查看历史，不能继续操作")]
    UnsupportedRules,
    #[error("规则设置无效")]
    InvalidConfig,
    #[error("座位无效")]
    InvalidSeat,
    #[error("当前不能执行此操作")]
    IllegalAction,
    #[error("固定牌墙必须包含每种牌各四张")]
    InvalidWall,
}

/// Score a completed concealed hand plus exposed/concealed melds. No waiting-list
/// or wall information is used, so the same function also evaluates 查大叫.
pub fn evaluate_hand(
    hand: &[Tile],
    melds: &[Meld],
    missing_suit: Option<u8>,
    context: WinContext,
    config: &RoomConfig,
) -> Option<HandValue> {
    if hand.len() + melds.len() * 3 != 14
        || hand.iter().any(|t| *t >= 27)
        || melds.iter().any(|m| m.tile >= 27)
    {
        return None;
    }
    let mut counts = [0u8; 27];
    for &t in hand {
        counts[t as usize] += 1;
    }
    let mut all = counts;
    for m in melds {
        all[m.tile as usize] += m.len() as u8;
    }
    if all.iter().any(|c| *c > 4)
        || missing_suit
            .map(|s| {
                all.iter()
                    .enumerate()
                    .any(|(i, c)| *c > 0 && i / 9 == s as usize)
            })
            .unwrap_or(false)
    {
        return None;
    }
    let seven = melds.is_empty() && counts.iter().all(|c| c % 2 == 0);
    let mut standard = false;
    let mut pungs = false;
    for pair in 0..27 {
        if counts[pair] < 2 {
            continue;
        }
        let mut rest = counts;
        rest[pair] -= 2;
        if rest.iter().all(|c| c % 3 == 0) {
            pungs = true;
        }
        if form_sets(&mut rest) {
            standard = true;
        }
    }
    if !seven && !standard {
        return None;
    }
    let mut multiplier = if seven {
        4
    } else if pungs {
        2
    } else {
        1
    };
    let mut labels = vec![if seven {
        "七对"
    } else if pungs {
        "大对子"
    } else {
        "平胡"
    }
    .to_owned()];
    let suits = (0..3)
        .filter(|s| all[s * 9..s * 9 + 9].iter().any(|c| *c > 0))
        .count();
    if suits == 1 {
        multiplier *= 4;
        labels.push("清一色".into());
    }
    let roots = all.iter().filter(|c| **c == 4).count() as u8;
    multiplier *= 1u32 << roots;
    if roots > 0 {
        labels.push(format!("{roots}根"));
    }
    if context.rob_kong {
        multiplier *= 2;
        labels.push("抢杠胡".into());
    } else if context.kong_event {
        multiplier *= 2;
        labels.push(
            if context.self_draw {
                "杠上花"
            } else {
                "杠上炮"
            }
            .into(),
        );
    }
    if context.self_draw {
        multiplier += 1;
        labels.push("自摸加一".into());
    }
    let uncapped_multiplier = multiplier;
    if let Some(cap) = config.cap {
        if multiplier > cap {
            labels.push(format!("{cap}倍封顶"));
        }
        multiplier = multiplier.min(cap);
    }
    Some(HandValue {
        multiplier,
        uncapped_multiplier,
        labels,
        roots,
    })
}
fn form_sets(counts: &mut [u8; 27]) -> bool {
    let Some(i) = counts.iter().position(|c| *c > 0) else {
        return true;
    };
    if counts[i] >= 3 {
        counts[i] -= 3;
        let ok = form_sets(counts);
        counts[i] += 3;
        if ok {
            return true;
        }
    }
    if i % 9 <= 6 && counts[i + 1] > 0 && counts[i + 2] > 0 {
        counts[i] -= 1;
        counts[i + 1] -= 1;
        counts[i + 2] -= 1;
        let ok = form_sets(counts);
        counts[i] += 1;
        counts[i + 1] += 1;
        counts[i + 2] += 1;
        if ok {
            return true;
        }
    }
    false
}

impl Game {
    pub fn new(config: RoomConfig, seed: u64) -> Result<Self, GameError> {
        let dealer = StdRng::seed_from_u64(seed ^ 0x53c4_aa19_9437).gen_range(0..4);
        Self::new_with_dealer(config, seed, dealer)
    }
    pub fn new_with_dealer(
        config: RoomConfig,
        seed: u64,
        dealer: usize,
    ) -> Result<Self, GameError> {
        let mut wall: Vec<Tile> = (0..27).flat_map(|t| [t; 4]).collect();
        wall.shuffle(&mut StdRng::seed_from_u64(seed));
        Self::from_wall(config, wall, dealer)
    }
    /// The last vector element is drawn first. Intended for deterministic tests
    /// and trusted server tooling, never for player-supplied walls.
    pub fn from_wall(
        config: RoomConfig,
        wall: Vec<Tile>,
        dealer: usize,
    ) -> Result<Self, GameError> {
        config.validate()?;
        if dealer >= 4 {
            return Err(GameError::InvalidSeat);
        }
        let mut counts = [0; 27];
        for &t in &wall {
            if t >= 27 {
                return Err(GameError::InvalidWall);
            }
            counts[t as usize] += 1;
        }
        if counts.iter().any(|c| *c != 4) {
            return Err(GameError::InvalidWall);
        }
        let mut game = Self {
            rules_version: RULES_VERSION.into(),
            config,
            phase: Phase::DingQue,
            turn: dealer,
            dealer,
            next_dealer: dealer,
            version: 0,
            players: Default::default(),
            wall,
            discards: vec![],
            pending: None,
            ledger: vec![],
            events: vec![],
            winners: vec![],
            current_after_kong: false,
        };
        for _ in 0..13 {
            for offset in 0..4 {
                let t = game.wall.pop().unwrap();
                game.players[(dealer + offset) % 4].hand.push(t);
            }
        }
        let tile = game.wall.pop().unwrap();
        game.players[dealer].hand.push(tile);
        game.players[dealer].last_draw = Some(tile);
        for p in &mut game.players {
            p.hand.sort_unstable();
        }
        game.event("start", Some(dealer), None, "牌局开始，请选择定缺");
        Ok(game)
    }
    pub fn is_over(&self) -> bool {
        matches!(self.phase, Phase::Finished | Phase::Aborted)
    }
    pub fn acting_seats(&self) -> Vec<usize> {
        match self.phase {
            Phase::DingQue => (0..4)
                .filter(|s| self.players[*s].missing_suit.is_none())
                .collect(),
            Phase::Playing => vec![self.turn],
            Phase::Responding => self
                .pending
                .as_ref()
                .map(|p| {
                    p.eligible
                        .iter()
                        .filter(|s| !p.responses.contains_key(s))
                        .copied()
                        .collect()
                })
                .unwrap_or_default(),
            _ => vec![],
        }
    }
    pub fn legal_actions(&self, seat: usize) -> Vec<Action> {
        if self.rules_version != RULES_VERSION || seat >= 4 || self.players[seat].won {
            return vec![];
        }
        let player = &self.players[seat];
        match self.phase {
            Phase::DingQue if player.missing_suit.is_none() => {
                (0..3).map(|suit| Action::DingQue { suit }).collect()
            }
            Phase::Playing if self.turn == seat => {
                let must_clear = player
                    .hand
                    .iter()
                    .any(|t| Some(t / 9) == player.missing_suit);
                let mut tiles = player.hand.clone();
                tiles.sort_unstable();
                tiles.dedup();
                let mut actions: Vec<Action> = tiles
                    .iter()
                    .filter(|t| !must_clear || Some(**t / 9) == player.missing_suit)
                    .map(|t| Action::Discard { tile: *t })
                    .collect();
                if player.last_draw.is_some() {
                    if self
                        .hand_value(
                            seat,
                            None,
                            WinContext {
                                self_draw: true,
                                kong_event: self.current_after_kong,
                                rob_kong: false,
                            },
                        )
                        .is_some()
                    {
                        actions.push(Action::Hu);
                    }
                    if !self.wall.is_empty() {
                        for t in tiles {
                            if Some(t / 9) != player.missing_suit
                                && (player.hand.iter().filter(|x| **x == t).count() == 4
                                    || player
                                        .melds
                                        .iter()
                                        .any(|m| m.tile == t && m.kind == MeldKind::Peng))
                            {
                                actions.push(Action::Kong { tile: t });
                            }
                        }
                    }
                }
                actions
            }
            Phase::Responding => self
                .pending
                .as_ref()
                .filter(|p| p.eligible.contains(&seat) && !p.responses.contains_key(&seat))
                .map(|p| self.response_actions(seat, p))
                .unwrap_or_default(),
            _ => vec![],
        }
    }
    fn response_actions(&self, seat: usize, pending: &Pending) -> Vec<Action> {
        let mut actions = vec![Action::Pass];
        let context = WinContext {
            self_draw: false,
            kong_event: pending.after_kong,
            rob_kong: pending.kind == PendingKind::SupplementalKong,
        };
        if self.hand_value(seat, Some(pending.tile), context).is_some() {
            actions.push(Action::Hu);
        }
        if pending.kind == PendingKind::Discard
            && Some(pending.tile / 9) != self.players[seat].missing_suit
        {
            let n = self.players[seat]
                .hand
                .iter()
                .filter(|t| **t == pending.tile)
                .count();
            if n >= 2 {
                actions.push(Action::Peng);
            }
            if n >= 3 && !self.wall.is_empty() {
                actions.push(Action::Kong { tile: pending.tile });
            }
        }
        actions
    }
    pub fn apply(&mut self, seat: usize, action: Action) -> Result<(), GameError> {
        if self.rules_version != RULES_VERSION {
            return Err(GameError::UnsupportedRules);
        }
        if seat >= 4 {
            return Err(GameError::InvalidSeat);
        }
        if !self.legal_actions(seat).contains(&action) {
            return Err(GameError::IllegalAction);
        }
        self.version += 1;
        if self.phase == Phase::Responding {
            let p = self.pending.as_mut().unwrap();
            p.responses.insert(seat, action);
            if p.responses.len() == p.eligible.len() {
                self.resolve_responses();
            }
            return Ok(());
        }
        match action {
            Action::DingQue { suit } => {
                self.players[seat].missing_suit = Some(suit);
                self.event("ding_que", Some(seat), None, "已选择定缺");
                if self.players.iter().all(|p| p.missing_suit.is_some()) {
                    self.phase = Phase::Playing;
                    self.event("play", Some(self.turn), None, "定缺完成，庄家出牌");
                }
            }
            Action::Discard { tile } => {
                self.remove(seat, tile, 1);
                self.players[seat].last_draw = None;
                self.discards.push(Discard {
                    seat,
                    tile,
                    claimed_by: None,
                });
                self.event("discard", Some(seat), Some(tile), "出牌");
                let after_kong = self.current_after_kong;
                self.current_after_kong = false;
                self.open_response(Pending {
                    kind: PendingKind::Discard,
                    from: seat,
                    tile,
                    after_kong,
                    eligible: vec![],
                    responses: BTreeMap::new(),
                });
            }
            Action::Kong { tile } => {
                if self.players[seat]
                    .melds
                    .iter()
                    .any(|m| m.tile == tile && m.kind == MeldKind::Peng)
                {
                    self.event(
                        "supplemental_kong_offer",
                        Some(seat),
                        Some(tile),
                        "申请补杠",
                    );
                    self.open_response(Pending {
                        kind: PendingKind::SupplementalKong,
                        from: seat,
                        tile,
                        after_kong: false,
                        eligible: vec![],
                        responses: BTreeMap::new(),
                    });
                } else {
                    self.remove(seat, tile, 4);
                    self.players[seat].melds.push(Meld {
                        tile,
                        kind: MeldKind::ConcealedKong,
                        from: None,
                    });
                    self.event("concealed_kong", Some(seat), None, "暗杠");
                    for from in 0..4 {
                        if from != seat && !self.players[from].won {
                            self.pay(from, seat, 2, "concealed_kong");
                        }
                    }
                    self.draw(seat, true);
                }
            }
            Action::Hu => {
                let value = self
                    .hand_value(
                        seat,
                        None,
                        WinContext {
                            self_draw: true,
                            kong_event: self.current_after_kong,
                            rob_kong: false,
                        },
                    )
                    .unwrap();
                let tile = self.players[seat].last_draw.unwrap();
                self.event("self_draw", Some(seat), Some(tile), "自摸");
                for from in 0..4 {
                    if from != seat && !self.players[from].won {
                        self.pay(from, seat, value.multiplier, "self_draw");
                    }
                }
                if self.winners.is_empty() {
                    self.next_dealer = seat;
                }
                self.mark_win(seat, tile, None, self.events.len() as u64, value);
                self.advance(seat);
            }
            _ => unreachable!("validated legal action"),
        }
        Ok(())
    }
    fn hand_value(
        &self,
        seat: usize,
        extra: Option<Tile>,
        context: WinContext,
    ) -> Option<HandValue> {
        let p = &self.players[seat];
        let mut hand = p.hand.clone();
        if let Some(t) = extra {
            hand.push(t);
        }
        evaluate_hand(&hand, &p.melds, p.missing_suit, context, &self.config)
    }
    fn open_response(&mut self, mut pending: Pending) {
        pending.eligible = (1..4)
            .map(|o| (pending.from + o) % 4)
            .filter(|s| !self.players[*s].won && self.response_actions(*s, &pending).len() > 1)
            .collect();
        self.phase = Phase::Responding;
        self.pending = Some(pending);
        if self.pending.as_ref().unwrap().eligible.is_empty() {
            self.resolve_responses();
        }
    }
    fn resolve_responses(&mut self) {
        let p = self.pending.take().unwrap();
        let hu: Vec<usize> = p
            .eligible
            .iter()
            .copied()
            .filter(|s| p.responses.get(s) == Some(&Action::Hu))
            .collect();
        if !hu.is_empty() {
            if self.winners.is_empty() {
                self.next_dealer = if hu.len() > 1 { p.from } else { hu[0] };
            }
            if p.kind == PendingKind::SupplementalKong {
                self.remove(p.from, p.tile, 1);
                self.players[p.from].last_draw = None;
                self.discards.push(Discard {
                    seat: p.from,
                    tile: p.tile,
                    claimed_by: None,
                });
            }
            let source_event_id = self.events.len() as u64;
            for seat in hu {
                let value = self
                    .hand_value(
                        seat,
                        Some(p.tile),
                        WinContext {
                            self_draw: false,
                            kong_event: p.after_kong,
                            rob_kong: p.kind == PendingKind::SupplementalKong,
                        },
                    )
                    .unwrap();
                let reason = if p.kind == PendingKind::SupplementalKong {
                    "rob_kong"
                } else if p.after_kong {
                    "kong_discard"
                } else {
                    "discard_win"
                };
                self.event(reason, Some(seat), Some(p.tile), "胡牌");
                self.pay(p.from, seat, value.multiplier, reason);
                self.mark_win(seat, p.tile, Some(p.from), source_event_id, value);
            }
            self.advance(p.from);
            return;
        }
        if p.kind == PendingKind::SupplementalKong {
            self.remove(p.from, p.tile, 1);
            self.players[p.from]
                .melds
                .iter_mut()
                .find(|m| m.tile == p.tile && m.kind == MeldKind::Peng)
                .unwrap()
                .kind = MeldKind::SupplementalKong;
            self.event("supplemental_kong", Some(p.from), Some(p.tile), "补杠成立");
            for from in 0..4 {
                if from != p.from && !self.players[from].won {
                    self.pay(from, p.from, 1, "supplemental_kong");
                }
            }
            self.draw(p.from, true);
            return;
        }
        if let Some(seat) = p
            .eligible
            .iter()
            .copied()
            .find(|s| matches!(p.responses.get(s), Some(Action::Peng | Action::Kong { .. })))
        {
            let kong = matches!(p.responses.get(&seat), Some(Action::Kong { .. }));
            self.remove(seat, p.tile, if kong { 3 } else { 2 });
            self.players[seat].melds.push(Meld {
                tile: p.tile,
                kind: if kong {
                    MeldKind::ExposedKong
                } else {
                    MeldKind::Peng
                },
                from: Some(p.from),
            });
            self.discards.last_mut().unwrap().claimed_by = Some(seat);
            self.players[seat].last_draw = None;
            self.turn = seat;
            self.current_after_kong = false;
            self.event(
                if kong { "exposed_kong" } else { "peng" },
                Some(seat),
                Some(p.tile),
                if kong { "直杠" } else { "碰" },
            );
            if kong {
                self.pay(p.from, seat, 2, "exposed_kong");
                self.draw(seat, true);
            } else {
                self.phase = Phase::Playing;
            }
        } else {
            self.advance(p.from);
        }
    }
    fn remove(&mut self, seat: usize, tile: Tile, count: usize) {
        for _ in 0..count {
            let i = self.players[seat]
                .hand
                .iter()
                .position(|t| *t == tile)
                .expect("validated tile");
            self.players[seat].hand.remove(i);
        }
    }
    fn mark_win(
        &mut self,
        seat: usize,
        tile: Tile,
        from: Option<usize>,
        source_event_id: u64,
        value: HandValue,
    ) {
        self.players[seat].won = true;
        self.players[seat].win = Some(WinRecord {
            tile,
            from,
            source_event_id,
            value,
        });
        self.winners.push(seat);
    }
    fn advance(&mut self, from: usize) {
        if self.winners.len() >= 3 {
            self.finish(false);
            return;
        }
        let next = (1..=4)
            .map(|n| (from + n) % 4)
            .find(|s| !self.players[*s].won)
            .unwrap();
        self.draw(next, false);
    }
    fn draw(&mut self, seat: usize, after_kong: bool) {
        let Some(tile) = self.wall.pop() else {
            self.finish(true);
            return;
        };
        self.players[seat].hand.push(tile);
        self.players[seat].hand.sort_unstable();
        self.players[seat].last_draw = Some(tile);
        self.turn = seat;
        self.current_after_kong = after_kong;
        self.phase = Phase::Playing;
        self.event(
            "draw",
            Some(seat),
            None,
            if after_kong { "杠后补牌" } else { "摸牌" },
        );
    }
    fn event(&mut self, kind: &str, seat: Option<usize>, tile: Option<Tile>, message: &str) {
        self.events.push(GameEvent {
            seq: self.events.len() as u64 + 1,
            kind: kind.into(),
            seat,
            tile,
            message: message.into(),
        });
    }
    fn pay(&mut self, from: usize, to: usize, multiplier: u32, reason: &str) {
        let amount = self.config.base_score * multiplier as i64;
        self.players[from].score -= amount;
        self.players[to].score += amount;
        self.ledger.push(ScoreEntry {
            from,
            to,
            reason: reason.into(),
            multiplier,
            amount,
            event_id: self.events.len() as u64,
        });
    }
    /// Best legal completion, irrespective of tiles still remaining in the wall.
    pub fn max_wait_value(&self, seat: usize) -> Option<HandValue> {
        if seat >= 4 {
            return None;
        }
        (0..27)
            .filter_map(|tile| self.hand_value(seat, Some(tile), WinContext::default()))
            .max_by_key(|v| v.multiplier)
    }
    fn finish(&mut self, exhausted: bool) {
        self.phase = Phase::Finished;
        self.pending = None;
        self.event(
            "finish",
            None,
            None,
            if exhausted {
                "牌墙摸完，流局结算"
            } else {
                "三家胡牌，本局结束"
            },
        );
        if !exhausted {
            return;
        }
        let flowers: [bool; 4] = std::array::from_fn(|s| {
            !self.players[s].won
                && self.players[s]
                    .hand
                    .iter()
                    .any(|t| Some(t / 9) == self.players[s].missing_suit)
        });
        let waits: [Option<HandValue>; 4] = std::array::from_fn(|s| {
            if self.players[s].won || flowers[s] {
                None
            } else {
                self.max_wait_value(s)
            }
        });
        for from in 0..4 {
            if flowers[from] {
                for (to, flower) in flowers.iter().enumerate() {
                    if !flower {
                        self.pay(from, to, self.config.flower_penalty, "flower_penalty");
                    }
                }
            } else if !self.players[from].won && waits[from].is_none() {
                for (to, value) in waits.iter().enumerate() {
                    if let Some(value) = value {
                        self.pay(from, to, value.multiplier, "not_ready_penalty");
                    }
                }
            }
        }
        let refunds: Vec<ScoreEntry> = self
            .ledger
            .iter()
            .filter(|e| {
                matches!(
                    e.reason.as_str(),
                    "exposed_kong" | "concealed_kong" | "supplemental_kong"
                ) && !self.players[e.to].won
                    && (flowers[e.to] || waits[e.to].is_none())
            })
            .cloned()
            .collect();
        for e in refunds {
            self.pay(e.to, e.from, e.multiplier, "kong_refund");
        }
    }
    pub fn abort(&mut self) {
        if self.is_over() {
            return;
        }
        self.version += 1;
        self.phase = Phase::Aborted;
        self.pending = None;
        self.event("abort", None, None, "牌局中止，保留已发生的收付");
    }
    pub fn view(&self, seat: usize) -> PlayerView {
        let reveal = self.is_over();
        PlayerView {
            game_id: GAME_ID.into(),
            rules_version: self.rules_version.clone(),
            config: self.config.clone(),
            version: self.version,
            phase: self.phase,
            turn: self.turn,
            dealer: self.dealer,
            next_dealer: self.next_dealer,
            self_seat: seat,
            own_hand: self
                .players
                .get(seat)
                .map(|p| p.hand.clone())
                .unwrap_or_default(),
            own_last_draw: self
                .players
                .get(seat)
                .filter(|p| !p.won)
                .and_then(|p| p.last_draw),
            players: self
                .players
                .iter()
                .enumerate()
                .map(|(s, p)| PublicPlayer {
                    hand_count: p.hand.len(),
                    hand: if reveal || s == seat {
                        Some(p.hand.clone())
                    } else {
                        None
                    },
                    melds: p
                        .melds
                        .iter()
                        .map(|m| PublicMeld {
                            tile: if m.kind != MeldKind::ConcealedKong || reveal || s == seat {
                                Some(m.tile)
                            } else {
                                None
                            },
                            kind: m.kind,
                            from: m.from,
                        })
                        .collect(),
                    missing_suit: if self.phase != Phase::DingQue || s == seat {
                        p.missing_suit
                    } else {
                        None
                    },
                    won: p.won,
                    score: p.score,
                    win: p.win.clone(),
                })
                .collect(),
            discards: self.discards.clone(),
            pending: self.pending.as_ref().map(|p| PublicPending {
                kind: p.kind,
                from: p.from,
                tile: p.tile,
                responded: p.responses.keys().copied().collect(),
            }),
            legal_actions: self.legal_actions(seat),
            ledger: self.ledger.clone(),
            events: self.events.clone(),
            winners: self.winners.clone(),
            wall_remaining: self.wall.len(),
        }
    }
    /// The policy receives only the same filtered view as this player.
    pub fn bot_action(&self, seat: usize) -> Option<Action> {
        bot::choose(&self.view(seat))
    }
    /// Physical conservation excludes logical winning copies and claimed discards.
    pub fn tile_counts(&self) -> [usize; 27] {
        let mut counts = [0; 27];
        for t in &self.wall {
            counts[*t as usize] += 1;
        }
        for p in &self.players {
            for t in &p.hand {
                counts[*t as usize] += 1;
            }
            for m in &p.melds {
                counts[m.tile as usize] += m.len();
            }
        }
        for d in &self.discards {
            if d.claimed_by.is_none() {
                counts[d.tile as usize] += 1;
            }
        }
        counts
    }
}
