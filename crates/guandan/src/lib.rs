//! Four-seat partnership Guandan. Two decks use distinct IDs, including jokers.
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
pub const GAME_ID: &str = "guandan";
pub const RULES_VERSION: &str = "family-partnership-v1";
pub type Card = u8;
pub fn rank(c: Card) -> u8 {
    let c = c % 54;
    if c < 52 {
        c / 4
    } else {
        c - 39
    }
}
pub fn number(r: u8) -> u8 {
    if r == 12 {
        2
    } else {
        r + 3
    }
}
fn from_number(n: u8) -> u8 {
    if n == 14 || n == 1 {
        11
    } else if n == 2 {
        12
    } else {
        n - 3
    }
}
pub fn wild(c: Card, level: u8) -> bool {
    c % 54 < 52 && (c % 54) % 4 == 1 && number(rank(c)) == level
}
// Deck two's suit is also measured within that deck (54 is not divisible by 4).
fn suit(c: Card) -> u8 {
    (c % 54) % 4
}
pub fn strength(r: u8, level: u8) -> u8 {
    if r >= 13 {
        r + 3
    } else if number(r) == level {
        15
    } else {
        number(r)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub base_score: i64,
    pub cap: Option<u32>,
    pub turn_seconds: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Returning,
    Playing,
    Finished,
    Aborted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Single,
    Pair,
    Triple,
    TriplePair,
    Straight,
    PairStraight,
    Airplane,
    Bomb,
    StraightFlush,
    Rocket,
}
impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Single => "单张",
            Self::Pair => "对子",
            Self::Triple => "三张",
            Self::TriplePair => "三带两",
            Self::Straight => "顺子",
            Self::PairStraight => "三连对",
            Self::Airplane => "钢板",
            Self::Bomb => "炸弹",
            Self::StraightFlush => "同花顺",
            Self::Rocket => "四大天王",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Combination {
    pub kind: Kind,
    pub high: u8,
    pub len: usize,
    pub power: u8,
}
impl Combination {
    pub fn beats(&self, other: &Self) -> bool {
        if self.power != other.power {
            return self.power > other.power;
        }
        self.kind == other.kind && self.len == other.len && self.high > other.high
    }
}
#[derive(Clone)]
struct Pattern {
    combo: Combination,
    counts: [usize; 15],
    suit: Option<u8>,
}
fn patterns(level: u8) -> Vec<Pattern> {
    let mut out = vec![];
    let mut add = |kind, high, groups: &[(u8, usize)], suit| {
        let mut counts = [0; 15];
        for &(r, n) in groups {
            counts[r as usize] += n;
        }
        let len = counts.iter().sum();
        let power = match kind {
            Kind::Rocket => 20,
            Kind::StraightFlush => 3,
            Kind::Bomb => {
                if len == 4 {
                    1
                } else if len == 5 {
                    2
                } else {
                    len as u8 - 2
                }
            }
            _ => 0,
        };
        out.push(Pattern {
            combo: Combination {
                kind,
                high,
                len,
                power,
            },
            counts,
            suit,
        });
    };
    for r in 0..15 {
        add(Kind::Single, strength(r, level), &[(r, 1)], None);
        add(Kind::Pair, strength(r, level), &[(r, 2)], None);
        if r < 13 {
            add(Kind::Triple, strength(r, level), &[(r, 3)], None);
            for n in 4..=10 {
                add(Kind::Bomb, strength(r, level), &[(r, n)], None);
            }
            for s in 0..15 {
                if s != r {
                    add(
                        Kind::TriplePair,
                        strength(r, level),
                        &[(r, 3), (s, 2)],
                        None,
                    );
                }
            }
        }
    }
    for (kind, length, unit) in [
        (Kind::Straight, 5, 1),
        (Kind::PairStraight, 3, 2),
        (Kind::Airplane, 2, 3),
    ] {
        // Ace can lead A2345 / AA2233 / AAA222, or end at Ace; no wrapping.
        for start in 1..=15 - length {
            let groups: Vec<_> = (start..start + length)
                .map(|n| (from_number(n), unit))
                .collect();
            let high = start + length - 1;
            add(kind, high, &groups, None);
            if kind == Kind::Straight {
                for s in 0..4 {
                    add(Kind::StraightFlush, high, &groups, Some(s));
                }
            }
        }
    }
    add(Kind::Rocket, 18, &[(13, 2), (14, 2)], None);
    out
}
fn extract(hand: &[Card], p: &Pattern, level: u8) -> Option<Vec<Card>> {
    let mut selected = vec![];
    let mut deficit = 0;
    for r in 0..15 {
        let natural: Vec<_> = hand
            .iter()
            .copied()
            .filter(|&c| {
                !wild(c, level) && rank(c) as usize == r && p.suit.is_none_or(|s| suit(c) == s)
            })
            .take(p.counts[r])
            .collect();
        let missing = p.counts[r] - natural.len();
        if r >= 13 && missing > 0 {
            return None;
        }
        deficit += missing;
        selected.extend(natural);
    }
    let wilds: Vec<_> = hand
        .iter()
        .copied()
        .filter(|&c| wild(c, level))
        .take(deficit)
        .collect();
    if wilds.len() != deficit {
        return None;
    }
    // A wildcard played alone is always its actual level rank.
    if p.combo.kind == Kind::Single && deficit > 0 && p.combo.high != 15 {
        return None;
    }
    selected.extend(wilds);
    selected.sort_unstable();
    Some(selected)
}
pub fn combinations(cards: &[Card], level: u8) -> Vec<Combination> {
    if cards.is_empty()
        || cards.len() > 10
        || cards.iter().any(|&c| c >= 108)
        || cards.iter().copied().collect::<BTreeSet<_>>().len() != cards.len()
        || !(2..=14).contains(&level)
    {
        return vec![];
    }
    let mut out: Vec<_> = patterns(level)
        .iter()
        .filter(|p| p.combo.len == cards.len() && extract(cards, p, level).is_some())
        .map(|p| p.combo.clone())
        .collect();
    out.sort_by_key(|c| (c.power, c.high));
    out.dedup();
    out.reverse();
    out
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Play {
        cards: Vec<Card>,
        #[serde(default)]
        combination: Option<Combination>,
    },
    Pass,
    Return {
        card: Card,
    },
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Player {
    pub hand: Vec<Card>,
    pub score: i64,
    pub plays: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Play {
    pub seat: usize,
    pub cards: Vec<Card>,
    pub combination: Combination,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub from: usize,
    pub to: usize,
    pub amount: i64,
    pub multiplier: u32,
    pub reason: String,
    pub event_id: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub seq: u64,
    pub seat: Option<usize>,
    pub kind: String,
    pub message: String,
    pub cards: Vec<Card>,
    pub combination: Option<Combination>,
    pub voice: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Return {
    pub from: usize,
    pub to: usize,
    pub tribute: Card,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub rules_version: String,
    pub config: Config,
    pub version: u64,
    pub phase: Phase,
    pub turn: usize,
    pub players: [Player; 4],
    pub level: u8,
    pub levels: [u8; 2],
    pub next_level: u8,
    pub finish_order: Vec<usize>,
    pub winners: Vec<usize>,
    pub last_play: Option<Play>,
    pub passed: Vec<usize>,
    pub returns: Vec<Return>,
    pub opening_seat: usize,
    pub match_winner: Option<usize>,
    pub upgrade: u8,
    pub ledger: Vec<Entry>,
    pub events: Vec<Event>,
}
impl Game {
    pub fn new(config: Config, seed: u64, previous: Option<&Self>) -> Result<Self, String> {
        if !(1..=1_000_000).contains(&config.base_score)
            || !matches!(config.cap, None | Some(8 | 16 | 32 | 64))
            || !matches!(config.turn_seconds, 0 | 10..=180)
        {
            return Err("掼蛋规则配置不合法".into());
        }
        let previous = previous.filter(|g| g.phase == Phase::Finished && g.match_winner.is_none());
        let mut g = Self {
            rules_version: RULES_VERSION.into(),
            config,
            version: 1,
            phase: Phase::Playing,
            turn: (seed % 4) as usize,
            players: Default::default(),
            level: previous.map_or(2, |g| g.next_level),
            levels: previous.map_or([2, 2], |g| g.levels),
            next_level: 2,
            finish_order: vec![],
            winners: vec![],
            last_play: None,
            passed: vec![],
            returns: vec![],
            opening_seat: 0,
            match_winner: None,
            upgrade: 0,
            ledger: vec![],
            events: vec![],
        };
        g.next_level = g.level;
        let mut deck: Vec<Card> = (0..108).collect();
        deck.shuffle(&mut StdRng::seed_from_u64(seed));
        for i in 0..4 {
            g.players[i].hand = deck[i * 27..(i + 1) * 27].to_vec();
            g.players[i].hand.sort_unstable();
        }
        g.event(
            None,
            "deal",
            format!("两副牌发牌，每人27张，本局打{}", g.level_name()),
            vec![],
            None,
            Some("deal"),
        );
        if let Some(old) = previous {
            g.tribute(old);
        }
        g.opening_seat = g.turn;
        if !g.returns.is_empty() {
            g.phase = Phase::Returning;
            g.turn = g.returns[0].from;
        }
        Ok(g)
    }
    pub fn level_name(&self) -> String {
        match self.level {
            11 => "J".into(),
            12 => "Q".into(),
            13 => "K".into(),
            14 => "A".into(),
            n => n.to_string(),
        }
    }
    fn event(
        &mut self,
        seat: Option<usize>,
        kind: &str,
        message: String,
        cards: Vec<Card>,
        combination: Option<Combination>,
        voice: Option<&str>,
    ) {
        self.events.push(Event {
            seq: self.events.len() as u64 + 1,
            seat,
            kind: kind.into(),
            message,
            cards,
            combination,
            voice: voice.map(|v| format!("gd/{v}")),
        });
    }
    fn tribute(&mut self, old: &Self) {
        let head = old.finish_order[0];
        let double = old.finish_order[1] % 2 == head % 2;
        let donors = if double {
            old.finish_order[2..].to_vec()
        } else {
            vec![old.finish_order[3]]
        };
        if donors
            .iter()
            .flat_map(|&i| &self.players[i].hand)
            .filter(|&&c| rank(c) == 14)
            .count()
            == 2
        {
            self.turn = head;
            self.event(
                None,
                "anti-tribute",
                "两张大王抗贡，头游领出".into(),
                vec![],
                None,
                Some("anti-tribute"),
            );
            return;
        }
        let mut gifts: Vec<_> = donors
            .iter()
            .map(|&i| {
                let card = *self.players[i]
                    .hand
                    .iter()
                    .filter(|&&c| !wild(c, self.level))
                    .max_by_key(|&&c| (strength(rank(c), self.level), c))
                    .unwrap();
                (i, card)
            })
            .collect();
        gifts.sort_by_key(|&(i, c)| {
            (
                std::cmp::Reverse(strength(rank(c), self.level)),
                (i + 4 - head) % 4,
            )
        });
        self.turn = gifts[0].0;
        for (index, (donor, card)) in gifts.into_iter().enumerate() {
            let recipient = if index == 0 { head } else { (head + 2) % 4 };
            self.players[donor].hand.retain(|&c| c != card);
            self.players[recipient].hand.push(card);
            self.returns.push(Return {
                from: recipient,
                to: donor,
                tribute: card,
            });
            self.event(
                Some(donor),
                "tribute",
                format!("座位{}向座位{}进贡", donor + 1, recipient + 1),
                vec![card],
                None,
                Some("tribute"),
            );
        }
    }
    pub fn running(&self) -> bool {
        matches!(self.phase, Phase::Playing | Phase::Returning)
    }
    fn next_active(&self, from: usize) -> usize {
        (1..=4)
            .map(|d| (from + d) % 4)
            .find(|&i| !self.players[i].hand.is_empty())
            .unwrap_or(from)
    }
    pub fn return_cards(&self, seat: usize) -> Vec<Card> {
        let hand = &self.players[seat].hand;
        let low: Vec<_> = hand
            .iter()
            .copied()
            .filter(|&c| {
                rank(c) < 13
                    && number(rank(c)) <= 10
                    && number(rank(c)) != self.level
                    && !wild(c, self.level)
            })
            .collect();
        if !low.is_empty() {
            return low;
        }
        let min = hand
            .iter()
            .filter(|&&c| !wild(c, self.level))
            .map(|&c| strength(rank(c), self.level))
            .min();
        hand.iter()
            .copied()
            .filter(|&c| !wild(c, self.level) && Some(strength(rank(c), self.level)) == min)
            .collect()
    }
    pub fn legal_actions(&self, seat: usize) -> Vec<Action> {
        if seat >= 4 || seat != self.turn || !self.running() {
            return vec![];
        }
        if self.phase == Phase::Returning {
            return self
                .return_cards(seat)
                .into_iter()
                .map(|card| Action::Return { card })
                .collect();
        }
        let mut out = vec![];
        let mut seen = BTreeSet::new();
        for p in patterns(self.level) {
            if self
                .last_play
                .as_ref()
                .is_some_and(|last| !p.combo.beats(&last.combination))
            {
                continue;
            }
            if let Some(cards) = extract(&self.players[seat].hand, &p, self.level) {
                let key = (cards.clone(), format!("{:?}", p.combo.kind), p.combo.high);
                if seen.insert(key) {
                    out.push(Action::Play {
                        cards,
                        combination: Some(p.combo),
                    });
                }
            }
        }
        out.sort_by_key(|a| match a {
            Action::Play {
                cards,
                combination: Some(c),
            } => (c.power, std::cmp::Reverse(cards.len()), c.high),
            _ => (0, std::cmp::Reverse(0), 0),
        });
        if self.last_play.is_some() {
            out.push(Action::Pass);
        }
        out
    }
    pub fn bot_action(&self, seat: usize) -> Option<Action> {
        let actions = self.legal_actions(seat);
        if self.phase == Phase::Returning {
            return actions.into_iter().min_by_key(|a| match a {
                Action::Return { card } => number(rank(*card)),
                _ => 255,
            });
        }
        let plays: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, Action::Play { .. }))
            .collect();
        if let Some(a) = plays.iter().find(
            |a| matches!(a,Action::Play{cards,..} if cards.len()==self.players[seat].hand.len()),
        ) {
            return Some((*a).clone());
        }
        if self
            .last_play
            .as_ref()
            .is_some_and(|p| p.seat % 2 == seat % 2)
        {
            return Some(Action::Pass);
        }
        plays
            .into_iter()
            .min_by_key(|a| match a {
                Action::Play {
                    cards,
                    combination: Some(c),
                } => {
                    let broken = cards
                        .iter()
                        .filter(|&&card| {
                            self.players[seat]
                                .hand
                                .iter()
                                .filter(|&&x| rank(x) == rank(card) && !wild(x, self.level))
                                .count()
                                >= 4
                        })
                        .count();
                    (
                        c.power as usize * 100
                            + broken * 12
                            + cards.iter().filter(|&&c| wild(c, self.level)).count() * 5
                            + 30usize.saturating_sub(cards.len() * 3),
                        c.high,
                    )
                }
                _ => (usize::MAX, 255),
            })
            .cloned()
            .or_else(|| actions.into_iter().find(|a| matches!(a, Action::Pass)))
    }
    pub fn apply(&mut self, seat: usize, action: Action) -> Result<(), String> {
        if self.rules_version != RULES_VERSION {
            return Err("不支持此存档的掼蛋规则版本".into());
        }
        if seat >= 4 || !self.running() || seat != self.turn {
            return Err("尚未轮到你操作".into());
        }
        match action {
            Action::Return { card } => {
                if self.phase != Phase::Returning || !self.return_cards(seat).contains(&card) {
                    return Err("请还一张不超过10的非级牌；没有时还最小的非逢人配牌".into());
                }
                let exchange = self.returns.remove(0);
                self.players[seat].hand.retain(|&c| c != card);
                self.players[exchange.to].hand.push(card);
                self.event(
                    Some(seat),
                    "return",
                    format!("座位{}向座位{}还贡", seat + 1, exchange.to + 1),
                    vec![card],
                    None,
                    Some("return"),
                );
                if self.returns.is_empty() {
                    self.phase = Phase::Playing;
                    self.turn = self.opening_seat;
                } else {
                    self.turn = self.returns[0].from;
                }
            }
            Action::Pass => {
                if self.phase != Phase::Playing || self.last_play.is_none() {
                    return Err("领出时不能不出".into());
                }
                self.passed.push(seat);
                self.event(
                    Some(seat),
                    "pass",
                    format!("座位{}不出", seat + 1),
                    vec![],
                    None,
                    Some("pass"),
                );
                let last = self.last_play.as_ref().unwrap().seat;
                let needed = (0..4)
                    .filter(|&i| i != last && !self.players[i].hand.is_empty())
                    .count();
                if self.passed.len() >= needed {
                    self.turn = if !self.players[last].hand.is_empty() {
                        last
                    } else if !self.players[(last + 2) % 4].hand.is_empty() {
                        (last + 2) % 4
                    } else {
                        self.next_active(last)
                    };
                    if self.players[last].hand.is_empty() {
                        self.event(
                            Some(self.turn),
                            "wind",
                            "对家接风领出".into(),
                            vec![],
                            None,
                            Some("wind"),
                        );
                    }
                    self.last_play = None;
                    self.passed.clear();
                } else {
                    self.turn = self.next_active(seat);
                }
            }
            Action::Play { cards, combination } => {
                if self.phase != Phase::Playing
                    || cards.iter().any(|c| !self.players[seat].hand.contains(c))
                {
                    return Err("只能打出自己的手牌".into());
                }
                let options = combinations(&cards, self.level);
                let c = if let Some(c) = combination {
                    options.into_iter().find(|v| *v == c)
                } else {
                    options.into_iter().find(|c| {
                        self.last_play
                            .as_ref()
                            .is_none_or(|p| c.beats(&p.combination))
                    })
                }
                .ok_or("不能组成合法掼蛋牌型")?;
                if self
                    .last_play
                    .as_ref()
                    .is_some_and(|p| !c.beats(&p.combination))
                {
                    return Err("这手牌压不过上家".into());
                }
                self.players[seat].hand.retain(|c| !cards.contains(c));
                self.players[seat].plays += 1;
                let kind = serde_json::to_value(c.kind)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string();
                let voice = if matches!(c.kind, Kind::Single | Kind::Pair | Kind::Triple) {
                    format!(
                        "{kind}-{}",
                        if c.high == 15 {
                            from_number(self.level)
                        } else if c.high >= 16 {
                            c.high - 3
                        } else {
                            from_number(c.high)
                        }
                    )
                } else {
                    kind
                };
                self.event(
                    Some(seat),
                    "play",
                    format!("座位{}出{} · {}张", seat + 1, c.kind.label(), cards.len()),
                    cards.clone(),
                    Some(c.clone()),
                    Some(&voice),
                );
                self.last_play = Some(Play {
                    seat,
                    cards,
                    combination: c,
                });
                self.passed.clear();
                if self.players[seat].hand.is_empty() {
                    self.finish_order.push(seat);
                    let place = self.finish_order.len();
                    self.event(
                        Some(seat),
                        "rank",
                        format!("座位{}第{}名出完", seat + 1, place),
                        vec![],
                        None,
                        Some(if place == 1 { "first" } else { "out" }),
                    );
                    if place == 3 || place == 2 && self.finish_order[0] % 2 == seat % 2 {
                        for i in 0..4 {
                            if !self.finish_order.contains(&i) {
                                self.finish_order.push(i);
                            }
                        }
                        self.settle();
                    }
                } else if self.players[seat].hand.len() <= 2 {
                    let count = self.players[seat].hand.len();
                    self.event(
                        Some(seat),
                        "warning",
                        format!("只剩{count}张"),
                        vec![],
                        None,
                        Some(if count == 1 { "one-left" } else { "two-left" }),
                    );
                }
                if self.running() {
                    self.turn = self.next_active(seat);
                }
            }
        }
        self.version += 1;
        Ok(())
    }
    fn settle(&mut self) {
        let head = self.finish_order[0];
        let team = head % 2;
        let partner = self
            .finish_order
            .iter()
            .position(|&i| i == (head + 2) % 4)
            .unwrap();
        self.upgrade = (4 - partner) as u8;
        self.winners = vec![team, team + 2];
        self.phase = Phase::Finished;
        if self.level == 14 && self.levels[team] == 14 && partner <= 2 {
            self.match_winner = Some(team);
        }
        self.levels[team] = (self.levels[team] + self.upgrade).min(14);
        self.next_level = self.levels[team];
        self.event(
            Some(head),
            "finish",
            format!(
                "{}队获胜 · {} · 升{}级{}",
                if team == 0 { "一" } else { "二" },
                if partner == 1 {
                    "双上"
                } else if partner == 2 {
                    "一三名"
                } else {
                    "一四名"
                },
                self.upgrade,
                if self.match_winner.is_some() {
                    " · 打过A，本轮完成"
                } else {
                    ""
                }
            ),
            vec![],
            None,
            Some(if self.match_winner.is_some() {
                "match-win"
            } else {
                "team-win"
            }),
        );
        let multiplier = self
            .config
            .cap
            .map_or(self.upgrade as u32, |n| n.min(self.upgrade as u32));
        for loser in 0..4 {
            if loser % 2 == team {
                continue;
            }
            for winner in self.winners.clone() {
                let amount = self.config.base_score * multiplier as i64;
                self.players[loser].score -= amount;
                self.players[winner].score += amount;
                self.ledger.push(Entry {
                    from: loser,
                    to: winner,
                    amount,
                    multiplier,
                    reason: "掼蛋队伍获胜".into(),
                    event_id: self.events.len() as u64,
                });
            }
        }
    }
    pub fn abort(&mut self) {
        if self.running() {
            self.phase = Phase::Aborted;
            self.version += 1;
            self.event(None, "abort", "本局中止，不计分".into(), vec![], None, None);
        }
    }
    pub fn view(&self, seat: usize) -> Value {
        json!({"game_id":GAME_ID,"rules_version":self.rules_version,"version":self.version,"phase":self.phase,"turn":self.turn,"self_seat":seat,"own_hand":self.players.get(seat).map(|p|&p.hand),"players":self.players.iter().enumerate().map(|(i,p)|json!({"hand_count":p.hand.len(),"hand":if i==seat{Some(&p.hand)}else{None},"score":p.score,"plays":p.plays})).collect::<Vec<_>>(),"level":self.level,"levels":self.levels,"next_level":self.next_level,"finish_order":self.finish_order,"winners":self.winners,"last_play":self.last_play,"passed":self.passed,"returns":self.returns,"opening_seat":self.opening_seat,"match_winner":self.match_winner,"upgrade":self.upgrade,"ledger":self.ledger,"events":self.events,"legal_actions":self.legal_actions(seat),"config":self.config})
    }
}

#[cfg(test)]
mod tests;
