use super::*;
fn cards(ranks: &[u8]) -> Vec<Card> {
    let mut counts = [0u8; 15];
    ranks
        .iter()
        .map(|&r| {
            let c = if r < 13 {
                r * 4 + counts[r as usize]
            } else {
                r + 39
            };
            counts[r as usize] += 1;
            c
        })
        .collect()
}
fn new(seed: u64) -> Game {
    Game::new(
        Config {
            base_score: 2,
            cap: None,
            turn_seconds: 30,
        },
        seed,
        0,
    )
    .unwrap()
}
#[test]
fn all_combinations_and_comparisons() {
    for (r, k) in [
        (vec![0], Kind::Single),
        (vec![0, 0], Kind::Pair),
        (vec![0, 0, 0], Kind::Triple),
        (vec![0, 0, 0, 1], Kind::TripleSingle),
        (vec![0, 0, 0, 1, 1], Kind::TriplePair),
        (vec![0, 1, 2, 3, 4], Kind::Straight),
        (vec![0, 0, 1, 1, 2, 2], Kind::PairStraight),
        (vec![0, 0, 0, 1, 1, 1], Kind::Airplane),
        (vec![0, 0, 0, 1, 1, 1, 3, 4], Kind::AirplaneSingle),
        (vec![0, 0, 0, 1, 1, 1, 3, 3, 4, 4], Kind::AirplanePair),
        (vec![0, 0, 0, 0, 1, 1], Kind::FourTwo),
        (vec![0, 0, 0, 0, 1, 1, 2, 2], Kind::FourPairs),
        (vec![0, 0, 0, 0], Kind::Bomb),
        (vec![13, 14], Kind::Rocket),
    ] {
        assert_eq!(classify(&cards(&r)).unwrap().kind, k, "{r:?}");
    }
    for r in [
        vec![0, 0, 1],
        vec![8, 9, 10, 11, 12],
        vec![0, 1, 2, 3],
        vec![0, 0, 0, 1, 1, 1, 3, 3],
        vec![0, 0, 1, 1],
    ] {
        assert!(classify(&cards(&r)).is_none(), "{r:?}");
    }
    assert!(classify(&[0, 0]).is_none());
    assert!(classify(&[54]).is_none());
    let bomb = classify(&cards(&[0, 0, 0, 0])).unwrap();
    assert!(bomb.beats(&classify(&cards(&[12])).unwrap()));
    assert!(classify(&[52, 53]).unwrap().beats(&bomb));
    assert!(!bomb.beats(&classify(&[52, 53]).unwrap()));
    assert!(!classify(&cards(&[1, 2, 3, 4, 5, 6]))
        .unwrap()
        .beats(&classify(&cards(&[0, 1, 2, 3, 4])).unwrap()));
}
#[test]
fn deal_privacy_bidding_redeal_and_roundtrip() {
    let mut g = new(42);
    let mut deck = g.bottom.clone();
    for p in &g.players {
        assert_eq!(p.hand.len(), 17);
        deck.extend(&p.hand);
    }
    deck.sort_unstable();
    assert_eq!(deck, (0..54).collect::<Vec<_>>());
    let v = g.view(0);
    assert!(v["bottom"].is_null());
    assert!(v["players"][1]["hand"].is_null());
    assert!(v.get("seed").is_none());
    for i in 0..3 {
        g.apply(i, Action::Bid { score: 0 }).unwrap();
    }
    assert_eq!(g.deal, 2);
    assert_eq!(g.turn, 1);
    assert_eq!(g.phase, Phase::Bidding);
    g.apply(1, Action::Bid { score: 1 }).unwrap();
    // A provisional highest bidder is not yet the landlord. Even that bidder
    // must not see the three bottom cards until bidding actually ends.
    for viewer in 0..3 {
        let view = g.view(viewer);
        assert!(view["bottom"].is_null());
        assert!(view["landlord"].is_null());
        assert_eq!(view["own_hand"].as_array().unwrap().len(), 17);
    }
    let before = serde_json::to_string(&g).unwrap();
    assert!(g.apply(2, Action::Bid { score: 1 }).is_err());
    assert_eq!(before, serde_json::to_string(&g).unwrap());
    g.apply(2, Action::Bid { score: 3 }).unwrap();
    assert_eq!(g.landlord, Some(2));
    assert_eq!(g.players[2].hand.len(), 20);
    assert_eq!(g.view(0)["bottom"].as_array().unwrap().len(), 3);
    let restored: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
    assert_eq!(g.view(1), restored.view(1));
}
#[test]
fn rejected_actions_are_atomic_and_passes_reset_lead() {
    let mut g = new(3);
    g.apply(0, Action::Bid { score: 3 }).unwrap();
    for a in [
        Action::Pass,
        Action::Play {
            cards: vec![g.players[0].hand[0]; 2],
        },
        Action::Play {
            cards: vec![g.players[1].hand[0]],
        },
        Action::Bid { score: 0 },
    ] {
        let before = serde_json::to_string(&g).unwrap();
        assert!(g.apply(0, a).is_err());
        assert_eq!(before, serde_json::to_string(&g).unwrap());
    }
    g.apply(
        0,
        Action::Play {
            cards: vec![g.players[0].hand[0]],
        },
    )
    .unwrap();
    assert!(g.apply(2, Action::Pass).is_err());
    g.apply(1, Action::Pass).unwrap();
    g.apply(2, Action::Pass).unwrap();
    assert_eq!(g.turn, 0);
    assert!(g.last_play.is_none());
    assert!(g.apply(0, Action::Pass).is_err());
}
#[test]
fn spring_bombs_cap_and_farmer_team_scoring() {
    let mut g = new(3);
    g.apply(0, Action::Bid { score: 3 }).unwrap();
    g.players[0].hand = cards(&[0, 0, 0, 0]);
    g.apply(
        0,
        Action::Play {
            cards: cards(&[0, 0, 0, 0]),
        },
    )
    .unwrap();
    assert_eq!(g.multiplier, 12);
    assert!(g.spring);
    assert_eq!(g.players[0].score, 48);
    assert_eq!(g.winners, vec![0]);
    assert_eq!(g.ledger.len(), 2);
    let mut g = new(3);
    g.config.cap = Some(8);
    g.apply(0, Action::Bid { score: 3 }).unwrap();
    g.players[0].hand = cards(&[0, 1]);
    g.players[1].hand = vec![52, 53];
    g.apply(0, Action::Play { cards: cards(&[0]) }).unwrap();
    g.apply(
        1,
        Action::Play {
            cards: vec![52, 53],
        },
    )
    .unwrap();
    assert_eq!(g.winners, vec![1, 2]);
    assert!(g.spring);
    assert_eq!(g.players[0].score, -32);
    assert_eq!(g.players[2].score, 16);
    assert_eq!(g.players.iter().map(|p| p.score).sum::<i64>(), 0);
}
#[test]
fn bots_complete_seeded_games_and_conserve_cards_and_scores() {
    for seed in 0..100 {
        let mut g = new(seed);
        for _ in 0..500 {
            if !g.running() {
                break;
            }
            let action = g.bot_action(g.turn).expect("bot must act");
            g.apply(g.turn, action).unwrap();
            let played: Vec<_> = g
                .events
                .iter()
                .filter(|e| e.kind == "play")
                .flat_map(|e| e.cards.iter().copied())
                .collect();
            let mut in_game: Vec<_> = g
                .players
                .iter()
                .flat_map(|p| p.hand.iter().copied())
                .chain(played)
                .collect();
            if g.phase == Phase::Bidding {
                in_game.extend(&g.bottom);
            }
            assert_eq!(in_game.len(), 54);
            assert_eq!(in_game.into_iter().collect::<BTreeSet<_>>().len(), 54);
        }
        assert_eq!(g.phase, Phase::Finished, "seed {seed}");
        assert_eq!(g.players.iter().map(|p| p.score).sum::<i64>(), 0);
    }
}
