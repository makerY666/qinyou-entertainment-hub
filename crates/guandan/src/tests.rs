use super::*;
fn config() -> Config {
    Config {
        base_score: 2,
        cap: None,
        turn_seconds: 0,
    }
}
fn game() -> Game {
    Game::new(config(), 24, None).unwrap()
}
fn cards(r: u8, n: usize) -> Vec<Card> {
    (0..n)
        .map(|i| {
            if r < 13 {
                r * 4 + (i % 4) as u8 + 54 * (i / 4) as u8
            } else {
                r + 39 + 54 * i as u8
            }
        })
        .collect()
}
fn play(g: &mut Game, seat: usize, cards: Vec<Card>) {
    g.apply(
        seat,
        Action::Play {
            cards,
            combination: None,
        },
    )
    .unwrap();
}
fn combo(c: &[Card], level: u8, kind: Kind) -> Combination {
    combinations(c, level)
        .into_iter()
        .find(|c| c.kind == kind)
        .unwrap()
}
#[test]
fn two_decks_and_privacy() {
    let g = game();
    assert_eq!(
        g.players
            .iter()
            .flat_map(|p| &p.hand)
            .copied()
            .collect::<BTreeSet<_>>()
            .len(),
        108
    );
    for seat in 0..4 {
        let v = g.view(seat);
        assert_eq!(v["own_hand"].as_array().unwrap().len(), 27);
        for i in 0..4 {
            assert_eq!(v["players"][i]["hand"].is_null(), i != seat);
        }
        assert!(v.get("seed").is_none());
    }
    assert!(g.view(4)["own_hand"].is_null());
    assert!(wild(49, 2));
    assert!(wild(103, 2));
    assert!(!wild(51, 2));
}
#[test]
fn singles_level_and_jokers() {
    let level = combo(&[20], 8, Kind::Single);
    assert_eq!(level.high, 15);
    assert!(level.beats(&combo(&[44], 8, Kind::Single)));
    assert!(combo(&[52], 8, Kind::Single).beats(&level));
    assert_eq!(combinations(&[49], 2).len(), 1);
    assert_eq!(combinations(&[49], 2)[0].high, 15);
    assert_eq!(combo(&[52, 106], 2, Kind::Pair).high, 16);
    assert!(combinations(&[52, 49], 2).is_empty());
}
#[test]
fn wildcards_shapes_and_bomb_hierarchy() {
    let mut bomb = cards(2, 8);
    bomb.extend([49, 103]);
    assert!(combo(&bomb, 2, Kind::Bomb).beats(&combo(&cards(11, 8), 2, Kind::Bomb)));
    let five = combo(&cards(0, 5), 2, Kind::Bomb);
    let six = combo(&cards(0, 6), 2, Kind::Bomb);
    let flush = combo(&[0, 4, 8, 12, 16], 2, Kind::StraightFlush);
    assert!(flush.beats(&five));
    assert!(six.beats(&flush));
    let king = combo(&[52, 53, 106, 107], 2, Kind::Rocket);
    assert!(king.beats(&combo(&bomb, 2, Kind::Bomb)));
    assert!(!king.beats(&king));
    assert!(combinations(&[0, 4, 8, 12, 49], 2)
        .iter()
        .any(|c| c.kind == Kind::StraightFlush));
    let house = combo(&[0, 1, 49, 4, 58], 2, Kind::TriplePair);
    assert_eq!(house.len, 5);
}
#[test]
fn fixed_sequences_and_ace_low() {
    assert_eq!(combo(&[44, 48, 0, 4, 8], 7, Kind::Straight).high, 5);
    assert_eq!(combo(&[28, 32, 36, 40, 44], 10, Kind::Straight).high, 14);
    assert!(combinations(&[40, 44, 48, 0, 4], 7).is_empty());
    assert!(combinations(&[0, 4, 8, 12, 16, 20], 7).is_empty());
    let pairs = [cards(11, 2), cards(12, 2), cards(0, 2)].concat();
    assert_eq!(combo(&pairs, 7, Kind::PairStraight).high, 3);
    assert!(combinations(&[cards(0, 3), cards(1, 3)].concat(), 2)
        .iter()
        .any(|c| c.kind == Kind::Airplane));
    assert!(combinations(&[cards(0, 3), vec![4]].concat(), 2).is_empty());
}
#[test]
fn malformed_inputs_and_rejected_actions_are_atomic() {
    for c in [
        vec![],
        vec![0, 0],
        vec![108],
        vec![52, 53],
        vec![52, 106, 53],
    ] {
        assert!(combinations(&c, 2).is_empty());
    }
    let mut g = game();
    g.turn = 0;
    let before = serde_json::to_value(&g).unwrap();
    for (seat, a) in [
        (1, Action::Pass),
        (0, Action::Pass),
        (
            0,
            Action::Play {
                cards: vec![108],
                combination: None,
            },
        ),
        (4, Action::Return { card: 0 }),
    ] {
        assert!(g.apply(seat, a).is_err());
        assert_eq!(before, serde_json::to_value(&g).unwrap());
    }
    let own = g.players[0].hand[0];
    assert!(g
        .apply(
            0,
            Action::Play {
                cards: vec![own, own],
                combination: None
            }
        )
        .is_err());
    assert_eq!(before, serde_json::to_value(&g).unwrap());
}

#[test]
fn unsupported_rules_version_cannot_mutate_saved_game() {
    let mut g = game();
    g.rules_version = "future-rules".into();
    let before = serde_json::to_value(&g).unwrap();
    assert!(g.apply(g.turn, g.bot_action(g.turn).unwrap()).is_err());
    assert_eq!(before, serde_json::to_value(&g).unwrap());
}
#[test]
fn pass_cycle_and_finished_partner_wind() {
    let mut g = game();
    g.turn = 0;
    g.players[0].hand = vec![0];
    g.players[1].hand = vec![4, 8];
    g.players[2].hand = vec![12, 16];
    g.players[3].hand = vec![20, 24];
    play(&mut g, 0, vec![0]);
    assert_eq!(g.finish_order, vec![0]);
    for seat in [1, 2, 3] {
        g.apply(seat, Action::Pass).unwrap();
    }
    assert_eq!(g.turn, 2);
    assert!(g.last_play.is_none());
    assert_eq!(g.events.last().unwrap().kind, "wind");
    play(&mut g, 2, vec![12]);
    for seat in [3, 1] {
        g.apply(seat, Action::Pass).unwrap();
    }
    assert_eq!(g.turn, 2);
    assert!(g.last_play.is_none());
}
#[test]
fn opponents_can_beat_an_out_player() {
    let mut g = game();
    g.turn = 0;
    g.players[0].hand = vec![0];
    g.players[1].hand = vec![4, 8];
    play(&mut g, 0, vec![0]);
    play(&mut g, 1, vec![4]);
    assert_eq!(g.last_play.as_ref().unwrap().seat, 1);
    assert_eq!(g.turn, 2);
}
#[test]
fn double_up_and_zero_sum() {
    let mut g = game();
    g.turn = 0;
    g.players[0].hand = vec![0];
    g.players[2].hand = vec![8];
    play(&mut g, 0, vec![0]);
    g.apply(1, Action::Pass).unwrap();
    play(&mut g, 2, vec![8]);
    assert_eq!(g.phase, Phase::Finished);
    assert_eq!(g.upgrade, 3);
    assert_eq!(g.levels, [5, 2]);
    assert_eq!(g.winners, vec![0, 2]);
    assert_eq!(g.ledger.len(), 4);
    assert_eq!(g.players.iter().map(|p| p.score).sum::<i64>(), 0);
    assert_eq!(g.players[0].score, 12);
}
#[test]
fn ranking_upgrade_and_passing_ace() {
    for (order, upgrade) in [(vec![0, 1, 2, 3], 2), (vec![0, 1, 3, 2], 1)] {
        let mut g = game();
        g.finish_order = order;
        g.level = 14;
        g.levels = [14, 12];
        g.settle();
        assert_eq!(g.upgrade, upgrade);
        assert_eq!(g.match_winner, if upgrade == 2 { Some(0) } else { None });
    }
    let mut old = game();
    old.levels = [13, 2];
    old.finish_order = vec![0, 2, 1, 3];
    old.settle();
    assert!(old.match_winner.is_none());
    assert_eq!(old.next_level, 14);
    old.match_winner = Some(0);
    let new = Game::new(config(), 38, Some(&old)).unwrap();
    assert_eq!(new.level, 2);
    assert_eq!(new.levels, [2, 2]);
    assert!(new.returns.is_empty());
}
#[test]
fn winning_opponents_low_level_does_not_pass_ace() {
    let mut g = game();
    g.level = 5;
    g.levels = [14, 5];
    g.finish_order = vec![0, 2, 1, 3];
    g.settle();
    assert_eq!(g.upgrade, 3);
    assert_eq!(g.next_level, 14);
    assert_eq!(g.match_winner, None);
    let next = Game::new(config(), 38, Some(&g)).unwrap();
    assert_eq!(next.level, 14);
    assert_eq!(next.levels, [14, 5]);
}
#[test]
fn tribute_and_return_keep_108_unique_cards() {
    let mut old = game();
    old.finish_order = vec![0, 1, 2, 3];
    old.settle();
    let mut next = (0..100)
        .map(|seed| Game::new(config(), seed, Some(&old)).unwrap())
        .find(|g| g.phase == Phase::Returning)
        .unwrap();
    assert_eq!(next.returns.len(), 1);
    assert_eq!(next.returns[0].from, 0);
    assert_eq!(next.returns[0].to, 3);
    assert!(!wild(next.returns[0].tribute, next.level));
    assert_eq!(next.players[0].hand.len(), 28);
    assert_eq!(next.players[3].hand.len(), 26);
    let ret = next.bot_action(0).unwrap();
    next.apply(0, ret).unwrap();
    assert_eq!(next.phase, Phase::Playing);
    assert_eq!(next.turn, 3);
    assert!(next.players.iter().all(|p| p.hand.len() == 27));
    assert_eq!(
        next.players
            .iter()
            .flat_map(|p| &p.hand)
            .copied()
            .collect::<BTreeSet<_>>()
            .len(),
        108
    );
}
#[test]
fn double_tribute_and_anti_tribute() {
    let mut old = game();
    old.finish_order = vec![0, 2, 1, 3];
    old.settle();
    let mut next = game();
    next.level = 5;
    next.players[1].hand = vec![53, 4, 8];
    next.players[3].hand = vec![107, 12, 16];
    next.tribute(&old);
    assert!(next.returns.is_empty());
    assert_eq!(next.turn, 0);
    assert_eq!(next.events.last().unwrap().kind, "anti-tribute");
    next.players[1].hand = vec![52, 4, 8];
    next.players[3].hand = vec![53, 12, 16];
    next.tribute(&old);
    assert_eq!(next.returns.len(), 2);
    assert_eq!(next.returns[0].to, 3);
    assert_eq!(next.returns[0].from, 0);
    assert_eq!(next.turn, 3);
    let mut single = game();
    let mut old = old;
    old.finish_order = vec![0, 1, 2, 3];
    single.players[3].hand = vec![53, 107, 0];
    single.tribute(&old);
    assert!(single.returns.is_empty());
    assert_eq!(single.turn, 0);
}
#[test]
fn return_excludes_level_and_wild_with_high_card_fallback() {
    let mut g = game();
    g.level = 7;
    g.players[0].hand = vec![16, 17, 32, 36, 44];
    assert_eq!(g.return_cards(0), vec![32]);
    g.players[0].hand = vec![17, 32, 36, 44];
    assert_eq!(g.return_cards(0), vec![32]);
    g.players[0].hand = vec![0, 4, 16, 17, 28];
    assert_eq!(g.return_cards(0), vec![0, 4, 28]);
}
#[test]
fn abort_is_scoreless_and_does_not_upgrade() {
    let mut g = game();
    g.abort();
    assert_eq!(g.phase, Phase::Aborted);
    assert!(g.ledger.is_empty());
    assert!(g.winners.is_empty());
    let next = Game::new(config(), 42, Some(&g)).unwrap();
    assert_eq!(next.level, 2);
}
#[test]
fn bots_respect_partners_and_finish_many_complete_rounds() {
    let mut g = game();
    g.turn = 0;
    g.players[0].hand = vec![4, 8];
    g.players[2].hand = vec![12, 16];
    play(&mut g, 0, vec![4]);
    g.apply(1, Action::Pass).unwrap();
    assert!(matches!(g.bot_action(2), Some(Action::Pass)));
    let mut old = None;
    for seed in 0..40 {
        let mut g = Game::new(config(), seed, old.as_ref()).unwrap();
        for _ in 0..800 {
            if !g.running() {
                break;
            }
            let action = g
                .bot_action(g.turn)
                .expect("live turn must have a bot action");
            if let Action::Play {
                cards,
                combination: Some(c),
            } = &action
            {
                assert!(combinations(cards, g.level).contains(c));
            }
            g.apply(g.turn, action).unwrap();
        }
        assert_eq!(g.phase, Phase::Finished, "seed {seed}");
        assert_eq!(g.players.iter().map(|p| p.score).sum::<i64>(), 0);
        assert_eq!(g.finish_order.len(), 4);
        let raw = serde_json::to_string(&g).unwrap();
        let restored: Game = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            serde_json::to_value(&g).unwrap(),
            serde_json::to_value(&restored).unwrap()
        );
        old = Some(g);
    }
}
