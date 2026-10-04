//! Deterministic tile-efficiency policy. Its only input is a player's public view.
//! Unknown tiles are weighted by remaining unseen copies, never the actual wall.
use super::*;
use std::collections::HashMap;

#[derive(Default)]
struct Evaluator {
    cache: HashMap<(u64, u8), i8>,
}
fn counts(hand: &[Tile]) -> [u8; 27] {
    let mut result = [0; 27];
    for &t in hand {
        result[t as usize] += 1;
    }
    result
}
impl Evaluator {
    fn distance(&mut self, original: &[u8; 27], open: u8, missing: Option<u8>) -> i8 {
        let mut c = *original;
        let mut missing_count = 0;
        if let Some(suit) = missing {
            for n in &mut c[suit as usize * 9..suit as usize * 9 + 9] {
                missing_count += *n as i8;
                *n = 0;
            }
        }
        let key = (c.iter().fold(0u64, |key, &n| key * 5 + n as u64), open);
        let structural = *self.cache.entry(key).or_insert_with(|| {
            let mut best = 8;
            search(&mut c, 0, open as i8, 0, 0, &mut best);
            // Sichuan permits a quad to count as two pairs (unlike Japanese seven pairs).
            if open == 0 {
                best = best.min(6 - c.iter().map(|n| (n / 2) as i8).sum::<i8>());
            }
            best
        });
        structural.max(missing_count - 1)
    }
}
fn search(c: &mut [u8; 27], mut i: usize, melds: i8, partial: i8, pair: i8, best: &mut i8) {
    if *best == -1 {
        return;
    }
    while i < 27 && c[i] == 0 {
        i += 1;
    }
    if i == 27 {
        *best = (*best).min(8 - 2 * melds - partial.min(4 - melds) - pair);
        return;
    }
    if melds < 4 {
        if c[i] >= 3 {
            c[i] -= 3;
            search(c, i, melds + 1, partial, pair, best);
            c[i] += 3;
        }
        if i % 9 < 7 && c[i + 1] > 0 && c[i + 2] > 0 {
            c[i] -= 1;
            c[i + 1] -= 1;
            c[i + 2] -= 1;
            search(c, i, melds + 1, partial, pair, best);
            c[i] += 1;
            c[i + 1] += 1;
            c[i + 2] += 1;
        }
    }
    if c[i] >= 2 {
        c[i] -= 2;
        if pair == 0 {
            search(c, i, melds, partial, 1, best);
        }
        if partial < 4 {
            search(c, i, melds, partial + 1, pair, best);
        }
        c[i] += 2;
    }
    if partial < 4 {
        for offset in [1, 2] {
            if i % 9 + offset < 9 && c[i + offset] > 0 {
                c[i] -= 1;
                c[i + offset] -= 1;
                search(c, i, melds, partial + 1, pair, best);
                c[i] += 1;
                c[i + offset] += 1;
            }
        }
    }
    c[i] -= 1;
    search(c, i, melds, partial, pair, best);
    c[i] += 1;
}

struct Policy<'a> {
    view: &'a PlayerView,
    known: [u8; 27],
    missing: Option<u8>,
    evaluator: Evaluator,
}
impl<'a> Policy<'a> {
    fn new(view: &'a PlayerView) -> Self {
        let mut known = counts(&view.own_hand);
        for player in &view.players {
            for meld in &player.melds {
                if let Some(tile) = meld.tile {
                    known[tile as usize] += if meld.kind == MeldKind::Peng { 3 } else { 4 };
                }
            }
            if let Some(win) = &player.win {
                if win.from.is_none() {
                    known[win.tile as usize] += 1;
                }
            }
        }
        for d in &view.discards {
            if d.claimed_by.is_none() {
                known[d.tile as usize] += 1;
            }
        }
        Self {
            view,
            known,
            missing: view.players[view.self_seat].missing_suit,
            evaluator: Evaluator::default(),
        }
    }
    // This is a risk estimate, not a Japanese furiten/suji safety guarantee.
    fn danger(&self, tile: Tile) -> i32 {
        self.view
            .players
            .iter()
            .enumerate()
            .filter(|(s, p)| {
                *s != self.view.self_seat && !p.won && p.missing_suit != Some(tile / 9)
            })
            .map(|(seat, p)| {
                let pressure =
                    p.melds.len() as i32 + if self.view.wall_remaining < 18 { 2 } else { 0 };
                let central = if (2..=6).contains(&(tile % 9)) { 3 } else { 1 };
                let seen_discard = self
                    .view
                    .discards
                    .iter()
                    .any(|d| d.seat == seat && d.tile == tile);
                pressure * central - if seen_discard { pressure / 2 } else { 0 }
            })
            .sum()
    }
    fn quality(&mut self, c: &[u8; 27], melds: &[Meld]) -> (i8, i32) {
        let distance = self.evaluator.distance(c, melds.len() as u8, self.missing);
        let mut utility = 0;
        for tile in 0..27 {
            let unseen = 4u8.saturating_sub(self.known[tile]);
            if unseen == 0 || c[tile] >= 4 || self.missing == Some(tile as u8 / 9) {
                continue;
            }
            let mut next = *c;
            next[tile] += 1;
            if self
                .evaluator
                .distance(&next, melds.len() as u8, self.missing)
                < distance
            {
                utility += unseen as i32 * 12;
                if distance == 0 {
                    let hand: Vec<Tile> = next
                        .iter()
                        .enumerate()
                        .flat_map(|(t, n)| std::iter::repeat(t as u8).take(*n as usize))
                        .collect();
                    if let Some(value) = evaluate_hand(
                        &hand,
                        melds,
                        self.missing,
                        WinContext::default(),
                        &self.view.config,
                    ) {
                        utility += unseen as i32 * value.multiplier.min(16) as i32 * 2;
                    }
                }
            }
        }
        // A nominal tenpai with no unseen winning tile cannot finish unchanged.
        // Treat it as needing a repair instead of preferring it to live progress.
        if distance == 0 && utility == 0 {
            (1, 48)
        } else {
            (distance, -utility)
        }
    }
    fn best_discard(
        &mut self,
        c: &[u8; 27],
        melds: &[Meld],
        allowed: Option<&[Action]>,
    ) -> Option<(Tile, (i8, i32))> {
        let forced = self.missing.filter(|s| {
            c[*s as usize * 9..*s as usize * 9 + 9]
                .iter()
                .any(|n| *n > 0)
        });
        let mut best = None;
        for t in 0..27 {
            if c[t] == 0 || forced.is_some_and(|s| t / 9 != s as usize) {
                continue;
            }
            if allowed.is_some_and(|actions| !actions.contains(&Action::Discard { tile: t as u8 }))
            {
                continue;
            }
            let mut next = *c;
            next[t] -= 1;
            let mut q = self.quality(&next, melds);
            q.1 += self.danger(t as u8)
                * if q.0 > 1 || self.view.wall_remaining < 12 {
                    3
                } else {
                    1
                };
            if best.as_ref().is_none_or(|(_, old)| q < *old) {
                best = Some((t as u8, q));
            }
        }
        best
    }
}

pub(super) fn choose(view: &PlayerView) -> Option<Action> {
    let actions = &view.legal_actions;
    if actions.is_empty() {
        return None;
    }
    if actions.contains(&Action::Hu) {
        return Some(Action::Hu);
    }
    let mut policy = Policy::new(view);
    let c = counts(&view.own_hand);
    let melds: Vec<Meld> = view.players[view.self_seat]
        .melds
        .iter()
        .filter_map(|m| {
            m.tile.map(|tile| Meld {
                tile,
                kind: m.kind,
                from: m.from,
            })
        })
        .collect();
    if view.phase == Phase::DingQue {
        return actions
            .iter()
            .filter_map(|a| {
                if let Action::DingQue { suit } = a {
                    let n: i32 = c[*suit as usize * 9..*suit as usize * 9 + 9]
                        .iter()
                        .map(|n| *n as i32)
                        .sum();
                    let d = policy.evaluator.distance(&c, 0, Some(*suit));
                    Some((d as i32 * 100 + n * 16, a.clone()))
                } else {
                    None
                }
            })
            .min_by_key(|(score, _)| *score)
            .map(|(_, a)| a);
    }
    let discard = policy.best_discard(&c, &melds, Some(actions));
    let baseline = discard
        .map(|(_, q)| q)
        .unwrap_or_else(|| policy.quality(&c, &melds));
    let mut chosen = discard
        .map(|(tile, _)| Action::Discard { tile })
        .unwrap_or(Action::Pass);
    let mut best = baseline;
    for a in actions {
        let mut next = c;
        let mut next_melds = melds.clone();
        let candidate = match a {
            Action::Peng => {
                let tile = view.pending.as_ref()?.tile;
                next[tile as usize] -= 2;
                next_melds.push(Meld {
                    tile,
                    kind: MeldKind::Peng,
                    from: view.pending.as_ref().map(|p| p.from),
                });
                policy
                    .best_discard(&next, &next_melds, None)
                    .map(|(_, q)| q)
            }
            Action::Kong { tile } => {
                if let Some(m) = next_melds
                    .iter_mut()
                    .find(|m| m.tile == *tile && m.kind == MeldKind::Peng)
                {
                    next[*tile as usize] -= 1;
                    m.kind = MeldKind::SupplementalKong;
                } else {
                    let responding = view.phase == Phase::Responding;
                    next[*tile as usize] -= if responding { 3 } else { 4 };
                    next_melds.push(Meld {
                        tile: *tile,
                        kind: if responding {
                            MeldKind::ExposedKong
                        } else {
                            MeldKind::ConcealedKong
                        },
                        from: view.pending.as_ref().map(|p| p.from),
                    });
                }
                let mut q = policy.quality(&next, &next_melds);
                q.1 -= 8; // modest guaranteed payment; never trade away a shanten step for it
                Some(q)
            }
            _ => None,
        };
        if let Some(q) = candidate {
            if q < best {
                best = q;
                chosen = a.clone();
            }
        }
    }
    Some(chosen)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(hand: Vec<Tile>) -> PlayerView {
        let mut game = Game::new_with_dealer(RoomConfig::default(), 5, 0).unwrap();
        game.phase = Phase::Playing;
        game.turn = 0;
        game.players[0].hand = hand;
        game.players[0].missing_suit = Some(2);
        game.players[0].last_draw = game.players[0].hand.last().copied();
        game.view(0)
    }
    #[test]
    fn distance_recognizes_sequences_seven_pairs_and_sichuan_quad_pairs() {
        let mut e = Evaluator::default();
        assert_eq!(
            e.distance(
                &counts(&[0, 1, 2, 3, 4, 5, 9, 10, 11, 13, 14, 15, 17, 17]),
                0,
                Some(2)
            ),
            -1
        );
        assert_eq!(
            e.distance(
                &counts(&[0, 1, 2, 3, 4, 5, 9, 10, 11, 13, 14, 16, 16]),
                0,
                Some(2)
            ),
            0
        );
        assert_eq!(
            e.distance(
                &counts(&[0, 0, 0, 0, 3, 3, 6, 6, 9, 9, 12, 12, 15, 15]),
                0,
                Some(2)
            ),
            -1
        );
        assert_eq!(
            e.distance(
                &counts(&[0, 0, 3, 3, 6, 6, 9, 9, 12, 12, 15, 15, 17]),
                0,
                Some(2)
            ),
            0
        );
        assert!(
            e.distance(
                &counts(&[0, 1, 2, 3, 4, 5, 9, 10, 11, 13, 14, 15, 18, 18]),
                0,
                Some(2)
            ) >= 1
        );
    }
    #[test]
    fn keeps_live_two_sided_wait_instead_of_local_neighbor_greed() {
        let v = view(vec![0, 1, 2, 3, 4, 5, 9, 10, 11, 13, 14, 16, 16, 17]);
        assert_eq!(choose(&v), Some(Action::Discard { tile: 17 }));
    }
    #[test]
    fn refuses_peng_that_breaks_seven_pairs_tenpai() {
        let mut v = view(vec![0, 0, 3, 3, 6, 6, 9, 9, 12, 12, 15, 15, 17]);
        v.phase = Phase::Responding;
        v.pending = Some(PublicPending {
            kind: PendingKind::Discard,
            from: 1,
            tile: 0,
            responded: vec![],
        });
        v.discards = vec![Discard {
            seat: 1,
            tile: 0,
            claimed_by: None,
        }];
        v.legal_actions = vec![Action::Peng, Action::Pass];
        assert_eq!(choose(&v), Some(Action::Pass));
    }
    #[test]
    fn refuses_kong_that_breaks_dragon_seven_pairs() {
        let v = view(vec![0, 0, 0, 0, 3, 3, 6, 6, 9, 9, 12, 12, 15, 16]);
        assert!(v.legal_actions.contains(&Action::Kong { tile: 0 }));
        assert!(matches!(
            choose(&v),
            Some(Action::Discard { tile: 15 | 16 })
        ));
    }
    #[test]
    fn hidden_opponent_hands_and_wall_order_do_not_affect_decision() {
        let mut game = Game::new_with_dealer(RoomConfig::default(), 91, 0).unwrap();
        for s in 0..4 {
            game.apply(s, Action::DingQue { suit: 2 }).unwrap();
        }
        let expected = game.bot_action(0);
        game.wall.reverse();
        for p in game.players.iter_mut().skip(1) {
            p.hand.reverse();
            p.last_draw = Some(26);
        }
        assert_eq!(game.bot_action(0), expected);
        // Even accidental extra hidden data in a view is ignored by the policy.
        let mut v = game.view(0);
        for p in v.players.iter_mut().skip(1) {
            p.hand = Some(vec![0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 4]);
        }
        assert_eq!(choose(&v), expected);
    }

    #[test]
    fn exhausted_public_waits_are_not_counted_as_live_outs() {
        let v = view(vec![0, 1, 2, 3, 4, 5, 9, 10, 11, 13, 14, 16, 16]);
        let mut policy = Policy::new(&v);
        let c = counts(&v.own_hand);
        assert_eq!(policy.quality(&c, &[]).0, 0);
        policy.known[12] = 4;
        policy.known[15] = 4;
        assert_eq!(policy.quality(&c, &[]), (1, 48));
    }

    #[test]
    fn defensive_risk_respects_missing_suits_and_finished_players() {
        let mut v = view(vec![0, 1, 2, 3, 4, 5, 9, 10, 11, 13, 14, 16, 16]);
        v.wall_remaining = 8;
        for p in v.players.iter_mut().skip(1) {
            p.missing_suit = Some(0);
        }
        assert_eq!(Policy::new(&v).danger(4), 0);
        assert!(Policy::new(&v).danger(13) > 0);
        for p in v.players.iter_mut().skip(1) {
            p.won = true;
        }
        assert_eq!(Policy::new(&v).danger(13), 0);
    }

    fn legacy(game: &Game, seat: usize) -> Action {
        let actions = game.legal_actions(seat);
        if actions.contains(&Action::Hu) {
            return Action::Hu;
        }
        if game.phase == Phase::DingQue {
            return Action::DingQue {
                suit: (0..3)
                    .min_by_key(|s| {
                        game.players[seat]
                            .hand
                            .iter()
                            .filter(|t| **t / 9 == *s)
                            .count()
                    })
                    .unwrap(),
            };
        }
        if let Some(a) = actions.iter().find(|a| matches!(a, Action::Kong { .. })) {
            return a.clone();
        }
        if actions.contains(&Action::Peng) {
            return Action::Peng;
        }
        let hand = &game.players[seat].hand;
        actions
            .iter()
            .filter_map(|a| {
                if let Action::Discard { tile } = a {
                    let same = hand.iter().filter(|t| **t == *tile).count() as i32;
                    let neighbors: i32 = hand
                        .iter()
                        .filter(|t| **t / 9 == *tile / 9 && **t != *tile)
                        .map(|t| match t.abs_diff(*tile) {
                            1 => 3,
                            2 => 1,
                            _ => 0,
                        })
                        .sum();
                    Some(((same - 1) * 8 + neighbors, a))
                } else {
                    None
                }
            })
            .min_by_key(|(score, _)| *score)
            .map(|(_, a)| a.clone())
            .unwrap_or(Action::Pass)
    }
    #[test]
    #[ignore = "deterministic comparison and timing; run explicitly in release"]
    fn compare_with_legacy_policy() {
        let started = std::time::Instant::now();
        let (mut score, mut wins, mut old_wins, mut decisions) = (0i64, 0, 0, 0);
        let mut max_decision = std::time::Duration::ZERO;
        for seed in 0..64 {
            let new_seat = seed as usize % 4;
            let mut game = Game::new(RoomConfig::default(), seed + 2000).unwrap();
            for _ in 0..1000 {
                if game.is_over() {
                    break;
                }
                let seat = game.acting_seats()[0];
                let action = if seat == new_seat {
                    let now = std::time::Instant::now();
                    let a = game.bot_action(seat).unwrap();
                    max_decision = max_decision.max(now.elapsed());
                    decisions += 1;
                    a
                } else {
                    legacy(&game, seat)
                };
                game.apply(seat, action).unwrap();
            }
            assert!(game.is_over());
            assert_eq!(game.tile_counts(), [4; 27]);
            score += game.players[new_seat].score;
            wins += usize::from(game.players[new_seat].won);
            old_wins += game
                .players
                .iter()
                .enumerate()
                .filter(|(s, p)| *s != new_seat && p.won)
                .count();
        }
        println!("64 seeded games: new score={score}, new wins={wins}/64, legacy wins={old_wins}/192, decisions={decisions}, slowest={max_decision:?}, elapsed={:?}", started.elapsed());
    }
}
