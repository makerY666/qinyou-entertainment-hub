//! Server-authoritative three-player landlord. Card IDs are unique, never ranks.
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub const GAME_ID: &str = "doudizhu";
pub const RULES_VERSION: &str = "family-bidding-v1";
pub type Card = u8;
/// 0..51: four suits per rank, 3 through 2; 52/53: small/big joker.
pub fn rank(card: Card) -> u8 {
    if card < 52 {
        card / 4
    } else {
        card - 39
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
    Bidding,
    Playing,
    Finished,
    Aborted,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Bid { score: u8 },
    Play { cards: Vec<Card> },
    Pass,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Single,
    Pair,
    Triple,
    TripleSingle,
    TriplePair,
    Straight,
    PairStraight,
    Airplane,
    AirplaneSingle,
    AirplanePair,
    FourTwo,
    FourPairs,
    Bomb,
    Rocket,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Combination {
    pub kind: Kind,
    pub high: u8,
    pub len: usize,
}
impl Combination {
    pub fn beats(&self, other: &Self) -> bool {
        if other.kind == Kind::Rocket {
            return false;
        }
        if self.kind == Kind::Rocket {
            return true;
        }
        if self.kind == Kind::Bomb && other.kind != Kind::Bomb {
            return true;
        }
        self.kind == other.kind && self.len == other.len && self.high > other.high
    }
}
pub fn classify(cards: &[Card]) -> Option<Combination> {
    if cards.is_empty() || cards.len() > 20 || cards.iter().any(|&c| c > 53) {
        return None;
    }
    if cards.iter().copied().collect::<BTreeSet<_>>().len() != cards.len() {
        return None;
    }
    let mut counts = [0usize; 15];
    for &c in cards {
        counts[rank(c) as usize] += 1;
    }
    let ranks: Vec<usize> = (0..15).filter(|&r| counts[r] > 0).collect();
    let n = cards.len();
    let make = |kind, high| {
        Some(Combination {
            kind,
            high: high as u8,
            len: n,
        })
    };
    if n == 2 && counts[13] == 1 && counts[14] == 1 {
        return make(Kind::Rocket, 14);
    }
    if ranks.len() == 1 {
        return make(
            match n {
                1 => Kind::Single,
                2 => Kind::Pair,
                3 => Kind::Triple,
                4 => Kind::Bomb,
                _ => return None,
            },
            ranks[0],
        );
    }
    if n == 4 || n == 5 {
        if let Some(r) = (0..13).find(|&r| counts[r] == 3) {
            if n == 4 {
                return make(Kind::TripleSingle, r);
            }
            if ranks.len() == 2 {
                return make(Kind::TriplePair, r);
            }
        }
    }
    let consecutive =
        ranks.last().copied().unwrap() < 12 && ranks.windows(2).all(|w| w[1] == w[0] + 1);
    if consecutive && ranks.len() >= 5 && ranks.iter().all(|&r| counts[r] == 1) {
        return make(Kind::Straight, *ranks.last()?);
    }
    if consecutive && ranks.len() >= 3 && ranks.iter().all(|&r| counts[r] == 2) {
        return make(Kind::PairStraight, *ranks.last()?);
    }
    for (unit, kind) in [
        (3, Kind::Airplane),
        (4, Kind::AirplaneSingle),
        (5, Kind::AirplanePair),
    ] {
        if n % unit != 0 || n / unit < 2 {
            continue;
        }
        let length = n / unit;
        if length > 12 {
            continue;
        }
        for start in 0..=12 - length {
            if !(start..start + length).all(|r| counts[r] == 3) {
                continue;
            }
            let wings: Vec<_> = (0..15)
                .filter(|r| !(start..start + length).contains(r) && counts[*r] > 0)
                .collect();
            let valid = match unit {
                3 => wings.is_empty(),
                4 => wings.len() == length && wings.iter().all(|&r| counts[r] == 1),
                _ => wings.len() == length && wings.iter().all(|&r| counts[r] == 2),
            };
            if valid {
                return make(kind, start + length - 1);
            }
        }
    }
    if n == 6 || n == 8 {
        if let Some(r) = (0..13).find(|&r| counts[r] == 4) {
            if n == 6 {
                return make(Kind::FourTwo, r);
            }
            if ranks.len() == 3 && ranks.iter().filter(|&&x| x != r).all(|&x| counts[x] == 2) {
                return make(Kind::FourPairs, r);
            }
        }
    }
    None
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
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub rules_version: String,
    pub config: Config,
    pub version: u64,
    pub phase: Phase,
    pub turn: usize,
    pub dealer: usize,
    pub players: [Player; 3],
    pub bottom: Vec<Card>,
    pub bids: [Option<u8>; 3],
    pub landlord: Option<usize>,
    pub bid: u8,
    pub multiplier: u32,
    pub passes: u8,
    pub last_play: Option<Play>,
    pub winners: Vec<usize>,
    pub spring: bool,
    pub ledger: Vec<Entry>,
    pub events: Vec<Event>,
    seed: u64,
    pub deal: u32,
}
impl Game {
    pub fn new(config: Config, seed: u64, dealer: usize) -> Result<Self, String> {
        if !(1..=1_000_000).contains(&config.base_score)
            || !matches!(config.cap, None | Some(8 | 16 | 32 | 64))
            || !matches!(config.turn_seconds, 0 | 10..=180)
            || dealer >= 3
        {
            return Err("斗地主规则配置不合法".into());
        }
        let mut g = Self {
            rules_version: RULES_VERSION.into(),
            config,
            version: 1,
            phase: Phase::Bidding,
            turn: dealer,
            dealer,
            players: Default::default(),
            bottom: vec![],
            bids: [None; 3],
            landlord: None,
            bid: 0,
            multiplier: 1,
            passes: 0,
            last_play: None,
            winners: vec![],
            spring: false,
            ledger: vec![],
            events: vec![],
            seed,
            deal: 0,
        };
        g.deal_cards();
        Ok(g)
    }
    fn event(&mut self, seat: Option<usize>, kind: &str, message: String, cards: Vec<Card>) {
        self.events.push(Event {
            seq: self.events.len() as u64 + 1,
            seat,
            kind: kind.into(),
            message,
            cards,
        });
    }
    fn deal_cards(&mut self) {
        let mut deck: Vec<Card> = (0..54).collect();
        deck.shuffle(&mut StdRng::seed_from_u64(
            self.seed.wrapping_add(u64::from(self.deal)),
        ));
        self.deal += 1;
        for (i, p) in self.players.iter_mut().enumerate() {
            p.hand = deck[i * 17..(i + 1) * 17].to_vec();
            p.hand.sort_unstable();
        }
        self.bottom = deck[51..].to_vec();
        self.bids = [None; 3];
        self.bid = 0;
        self.turn = self.dealer;
        self.event(
            None,
            "deal",
            format!("第 {} 次发牌，请依次叫分", self.deal),
            vec![],
        );
    }
    pub fn running(&self) -> bool {
        matches!(self.phase, Phase::Bidding | Phase::Playing)
    }
    pub fn apply(&mut self, seat: usize, action: Action) -> Result<(), String> {
        if self.rules_version != RULES_VERSION {
            return Err("不支持此存档的斗地主规则版本".into());
        }
        if seat >= 3 || seat != self.turn || !self.running() {
            return Err("现在不是你的回合".into());
        }
        match action {
            Action::Bid { score } => {
                if self.phase != Phase::Bidding || score > 3 || (score > 0 && score <= self.bid) {
                    return Err("只能不叫或叫高于当前的 1–3 分".into());
                }
                self.bids[seat] = Some(score);
                if score > self.bid {
                    self.bid = score;
                    self.landlord = Some(seat);
                }
                self.event(
                    Some(seat),
                    "bid",
                    if score == 0 {
                        "不叫".into()
                    } else {
                        format!("叫 {score} 分")
                    },
                    vec![],
                );
                if score == 3 || self.bids.iter().all(Option::is_some) {
                    if let Some(landlord) = self.landlord {
                        self.players[landlord].hand.extend(&self.bottom);
                        self.players[landlord].hand.sort_unstable();
                        self.phase = Phase::Playing;
                        self.turn = landlord;
                        self.multiplier = u32::from(self.bid);
                        self.event(
                            Some(landlord),
                            "landlord",
                            "成为地主，收取底牌并先出牌".into(),
                            self.bottom.clone(),
                        );
                    } else {
                        self.dealer = (self.dealer + 1) % 3;
                        self.deal_cards();
                    }
                } else {
                    self.turn = (seat + 1) % 3;
                }
            }
            Action::Pass => {
                if self.phase != Phase::Playing || self.last_play.is_none() {
                    return Err("首家必须出牌，不能不出".into());
                }
                self.event(Some(seat), "pass", "不出".into(), vec![]);
                self.passes += 1;
                if self.passes == 2 {
                    self.turn = self.last_play.as_ref().unwrap().seat;
                    self.last_play = None;
                    self.passes = 0;
                } else {
                    self.turn = (seat + 1) % 3;
                }
            }
            Action::Play { mut cards } => {
                if self.phase != Phase::Playing {
                    return Err("请先完成叫分".into());
                }
                let combination = classify(&cards).ok_or("所选牌不构成合法牌型")?;
                if cards.iter().any(|c| !self.players[seat].hand.contains(c)) {
                    return Err("只能打出自己的手牌".into());
                }
                if self
                    .last_play
                    .as_ref()
                    .is_some_and(|p| !combination.beats(&p.combination))
                {
                    return Err("需要同牌型更大的牌，或炸弹、火箭".into());
                }
                cards.sort_unstable();
                self.players[seat].hand.retain(|c| !cards.contains(c));
                self.players[seat].plays += 1;
                if matches!(combination.kind, Kind::Bomb | Kind::Rocket) {
                    self.multiplier *= 2;
                }
                self.last_play = Some(Play {
                    seat,
                    cards: cards.clone(),
                    combination,
                });
                self.passes = 0;
                self.event(
                    Some(seat),
                    "play",
                    format!("打出 {} 张牌", cards.len()),
                    cards,
                );
                if self.players[seat].hand.is_empty() {
                    self.finish(seat);
                } else {
                    self.turn = (seat + 1) % 3;
                }
            }
        }
        self.version += 1;
        Ok(())
    }
    fn finish(&mut self, winner: usize) {
        let landlord = self.landlord.unwrap();
        let landlord_won = winner == landlord;
        self.spring = if landlord_won {
            (0..3)
                .filter(|&i| i != landlord)
                .all(|i| self.players[i].plays == 0)
        } else {
            self.players[landlord].plays == 1
        };
        if self.spring {
            self.multiplier *= 2;
        }
        let multiplier = self
            .config
            .cap
            .map_or(self.multiplier, |c| c.min(self.multiplier));
        let amount = self.config.base_score * i64::from(multiplier);
        self.winners = (0..3)
            .filter(|&i| (i == landlord) == landlord_won)
            .collect();
        self.event(
            Some(winner),
            "finish",
            format!(
                "{}获胜{}",
                if landlord_won { "地主" } else { "农民" },
                if self.spring { "，春天翻倍" } else { "" }
            ),
            vec![],
        );
        for farmer in (0..3).filter(|&i| i != landlord) {
            let (from, to) = if landlord_won {
                (farmer, landlord)
            } else {
                (landlord, farmer)
            };
            self.players[from].score -= amount;
            self.players[to].score += amount;
            self.ledger.push(Entry {
                from,
                to,
                amount,
                multiplier,
                reason: if landlord_won {
                    "landlord_win"
                } else {
                    "farmers_win"
                }
                .into(),
                event_id: self.events.len() as u64,
            });
        }
        self.phase = Phase::Finished;
    }
    pub fn abort(&mut self) {
        if self.running() {
            self.phase = Phase::Aborted;
            self.version += 1;
            self.event(None, "abort", "本局中止，不计分".into(), vec![]);
        }
    }
    pub fn legal_actions(&self, seat: usize) -> Vec<Action> {
        if seat >= 3 || self.turn != seat || !self.running() {
            return vec![];
        }
        if self.phase == Phase::Bidding {
            return std::iter::once(0)
                .chain(self.bid + 1..=3)
                .map(|score| Action::Bid { score })
                .collect();
        }
        let mut result = vec![];
        if self.last_play.is_some() {
            result.push(Action::Pass);
        }
        result.extend(
            self.candidates(seat)
                .into_iter()
                .take(64)
                .map(|cards| Action::Play { cards }),
        );
        result
    }
    pub fn bot_action(&self, seat: usize) -> Option<Action> {
        if seat >= 3 || self.turn != seat || !self.running() {
            return None;
        }
        if self.phase == Phase::Bidding {
            let strength: usize = self.players[seat]
                .hand
                .iter()
                .map(|&c| {
                    if c >= 52 {
                        3
                    } else if rank(c) == 12 {
                        1
                    } else {
                        0
                    }
                })
                .sum();
            let score = if strength >= 6 {
                3
            } else if strength >= 4 {
                2
            } else {
                1
            };
            return Some(Action::Bid {
                score: if score > self.bid { score } else { 0 },
            });
        }
        // Let a farmer partner retain the lead unless we can go out immediately.
        let candidates = self.candidates(seat);
        if let Some(cards) = candidates
            .iter()
            .find(|c| c.len() == self.players[seat].hand.len())
        {
            return Some(Action::Play {
                cards: cards.clone(),
            });
        }
        if self
            .last_play
            .as_ref()
            .is_some_and(|p| Some(p.seat) != self.landlord && Some(seat) != self.landlord)
        {
            return Some(Action::Pass);
        }
        candidates
            .into_iter()
            .next()
            .map(|cards| Action::Play { cards })
            .or(Some(Action::Pass))
    }
    pub fn view(&self, seat: usize) -> Value {
        let landlord = if self.phase == Phase::Bidding {
            None
        } else {
            self.landlord
        };
        json!({"game_id":GAME_ID,"rules_version":self.rules_version,"version":self.version,"phase":self.phase,"turn":self.turn,"dealer":self.dealer,"self_seat":seat,"own_hand":self.players.get(seat).map(|p|p.hand.clone()).unwrap_or_default(),"players":self.players.iter().enumerate().map(|(i,p)|json!({"hand_count":p.hand.len(),"hand":if i==seat{Some(&p.hand)}else{None},"score":p.score,"plays":p.plays})).collect::<Vec<_>>(),"bottom":if landlord.is_some(){Some(&self.bottom)}else{None},"bids":self.bids,"landlord":landlord,"bid":self.bid,"multiplier":self.multiplier,"last_play":self.last_play,"passes":self.passes,"spring":self.spring,"winners":self.winners,"ledger":self.ledger,"events":self.events,"legal_actions":self.legal_actions(seat),"config":self.config,"deal":self.deal})
    }
    /// One canonical suit choice per rank combination. Manual plays accept all suits.
    fn candidates(&self, seat: usize) -> Vec<Vec<Card>> {
        let hand = &self.players[seat].hand;
        let groups: Vec<Vec<Card>> = (0..15)
            .map(|r| hand.iter().copied().filter(|&c| rank(c) == r).collect())
            .collect();
        let mut all = BTreeSet::new();
        let mut add = |mut cards: Vec<Card>| {
            cards.sort_unstable();
            if classify(&cards).is_some_and(|c| {
                self.last_play
                    .as_ref()
                    .is_none_or(|p| c.beats(&p.combination))
            }) {
                all.insert(cards);
            }
        };
        for g in &groups {
            for n in 1..=g.len() {
                add(g[..n].to_vec());
            }
        }
        if hand.contains(&52) && hand.contains(&53) {
            add(vec![52, 53]);
        }
        for (r, g) in groups.iter().enumerate().take(13) {
            if g.len() >= 3 {
                for (s, wing) in groups.iter().enumerate() {
                    if s != r {
                        for n in 1..=2.min(wing.len()) {
                            let mut c = g[..3].to_vec();
                            c.extend(&wing[..n]);
                            add(c);
                        }
                    }
                }
            }
            if g.len() == 4 {
                let remaining: Vec<_> = hand
                    .iter()
                    .copied()
                    .filter(|&c| rank(c) as usize != r)
                    .collect();
                for a in 0..remaining.len() {
                    for b in a + 1..remaining.len() {
                        let mut c = g.clone();
                        c.extend([remaining[a], remaining[b]]);
                        add(c);
                    }
                }
                let pairs: Vec<_> = (0..13)
                    .filter(|&s| s != r && groups[s].len() >= 2)
                    .collect();
                for a in 0..pairs.len() {
                    for b in a + 1..pairs.len() {
                        let mut c = g.clone();
                        c.extend(&groups[pairs[a]][..2]);
                        c.extend(&groups[pairs[b]][..2]);
                        add(c);
                    }
                }
            }
        }
        for copies in 1..=3 {
            let min = match copies {
                1 => 5,
                2 => 3,
                _ => 2,
            };
            for start in 0..12 {
                for end in start + min..=12 {
                    if !(start..end).all(|r| groups[r].len() >= copies) {
                        continue;
                    }
                    let body: Vec<_> = (start..end)
                        .flat_map(|r| groups[r][..copies].iter().copied())
                        .collect();
                    add(body.clone());
                    if copies == 3 {
                        for wing_size in 1..=2 {
                            let options: Vec<_> = (0..15)
                                .filter(|r| {
                                    !(start..end).contains(r) && groups[*r].len() >= wing_size
                                })
                                .map(|r| groups[r][..wing_size].to_vec())
                                .collect();
                            for wings in choose(&options, end - start) {
                                let mut c = body.clone();
                                c.extend(wings);
                                add(c);
                            }
                        }
                    }
                }
            }
        }
        let mut result: Vec<_> = all.into_iter().collect();
        result.sort_by_key(|c| {
            let t = classify(c).unwrap();
            (
                matches!(t.kind, Kind::Bomb | Kind::Rocket),
                if self.last_play.is_none() {
                    20 - c.len()
                } else {
                    0
                },
                t.high,
                c.clone(),
            )
        });
        result
    }
}
fn choose(options: &[Vec<Card>], n: usize) -> Vec<Vec<Card>> {
    if n == 0 {
        return vec![vec![]];
    }
    if options.len() < n {
        return vec![];
    }
    let mut result = vec![];
    for i in 0..=options.len() - n {
        for mut tail in choose(&options[i + 1..], n - 1) {
            tail.extend(&options[i]);
            result.push(tail);
        }
    }
    result
}

#[cfg(test)]
mod tests;
