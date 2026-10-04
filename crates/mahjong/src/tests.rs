use super::*;

fn plain() -> Vec<Tile> {
    vec![0, 1, 2, 3, 4, 5, 9, 10, 11, 15, 16, 17, 13, 13]
}
fn wait() -> Vec<Tile> {
    let mut h = plain();
    h.pop();
    h
}
fn pure() -> Vec<Tile> {
    vec![0, 1, 2, 2, 3, 4, 5, 6, 7, 6, 7, 8, 4, 4]
}
fn score(hand: &[Tile], ctx: WinContext, cap: Option<u32>) -> u32 {
    evaluate_hand(
        hand,
        &[],
        Some(2),
        ctx,
        &RoomConfig {
            cap,
            ..Default::default()
        },
    )
    .unwrap()
    .multiplier
}
fn fixture(hands: [Vec<Tile>; 4], melds: [Vec<Meld>; 4]) -> Game {
    let mut g = Game::new_with_dealer(RoomConfig::default(), 42, 0).unwrap();
    g.phase = Phase::Playing;
    g.events.clear();
    g.wall.clear();
    let mut counts = [0usize; 27];
    for s in 0..4 {
        g.players[s] = Player {
            hand: hands[s].clone(),
            melds: melds[s].clone(),
            missing_suit: Some(2),
            last_draw: hands[s].last().copied(),
            ..Default::default()
        };
        for &t in &hands[s] {
            counts[t as usize] += 1;
        }
        for m in &melds[s] {
            counts[m.tile as usize] += m.len();
        }
    }
    for (t, count) in counts.into_iter().enumerate() {
        assert!(count <= 4, "fixture has too many {t}");
        for _ in count..4 {
            g.wall.push(t as u8);
        }
    }
    g
}
#[test]
fn acceptance_multipliers() {
    let normal = WinContext::default();
    let zimo = WinContext {
        self_draw: true,
        ..normal
    };
    assert_eq!(score(&plain(), normal, None), 1);
    assert_eq!(score(&plain(), zimo, None), 2);
    assert_eq!(
        score(
            &[0, 0, 0, 2, 2, 2, 9, 9, 9, 13, 13, 13, 15, 15],
            normal,
            None
        ),
        2
    );
    assert_eq!(score(&pure(), normal, None), 4);
    assert_eq!(score(&pure(), zimo, None), 5);
    assert_eq!(
        score(&[0, 0, 0, 2, 2, 2, 4, 4, 4, 6, 6, 6, 8, 8], normal, None),
        8
    );
    assert_eq!(
        score(
            &[0, 0, 2, 2, 4, 4, 9, 9, 11, 11, 13, 13, 15, 15],
            normal,
            None
        ),
        4
    );
    assert_eq!(
        score(
            &[0, 0, 0, 0, 2, 2, 9, 9, 11, 11, 13, 13, 15, 15],
            normal,
            None
        ),
        8
    );
    assert_eq!(
        score(&[0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6], normal, None),
        16
    );
    assert_eq!(
        score(
            &pure(),
            WinContext {
                self_draw: true,
                kong_event: true,
                rob_kong: false
            },
            None
        ),
        9
    );
    assert_eq!(
        score(
            &pure(),
            WinContext {
                self_draw: true,
                kong_event: true,
                rob_kong: false
            },
            Some(8)
        ),
        8
    );
}
#[test]
fn roots_caps_and_contexts() {
    let h = [0, 0, 0, 0, 9, 9, 9, 9, 11, 11, 11, 11, 12, 12];
    assert_eq!(score(&h, WinContext::default(), None), 32);
    assert_eq!(
        score(
            &h,
            WinContext {
                self_draw: true,
                ..Default::default()
            },
            None
        ),
        33
    );
    assert_eq!(
        score(
            &h,
            WinContext {
                self_draw: true,
                ..Default::default()
            },
            Some(32)
        ),
        32
    );
    for cap in [8, 16, 32, 64] {
        assert_eq!(
            score(
                &h,
                WinContext {
                    self_draw: true,
                    kong_event: true,
                    rob_kong: false
                },
                Some(cap)
            ),
            65.min(cap)
        );
    }
    assert_eq!(
        score(
            &plain(),
            WinContext {
                rob_kong: true,
                ..Default::default()
            },
            None
        ),
        2
    );
    assert_eq!(
        score(
            &plain(),
            WinContext {
                kong_event: true,
                ..Default::default()
            },
            None
        ),
        2
    );
    let melds = vec![
        Meld {
            tile: 0,
            kind: MeldKind::ConcealedKong,
            from: None,
        },
        Meld {
            tile: 9,
            kind: MeldKind::Peng,
            from: Some(1),
        },
        Meld {
            tile: 11,
            kind: MeldKind::Peng,
            from: Some(2),
        },
        Meld {
            tile: 13,
            kind: MeldKind::Peng,
            from: Some(3),
        },
    ];
    // 金钩钓 has no extra multiplier: 大对子 × one root.
    assert_eq!(
        evaluate_hand(
            &[15, 15],
            &melds,
            Some(2),
            WinContext::default(),
            &RoomConfig::default()
        )
        .unwrap()
        .multiplier,
        4
    );
}
#[test]
fn invalid_and_missing_suit_hands_do_not_win() {
    assert!(evaluate_hand(
        &plain(),
        &[],
        Some(0),
        WinContext::default(),
        &RoomConfig::default()
    )
    .is_none());
    assert!(evaluate_hand(
        &[0; 14],
        &[],
        Some(2),
        WinContext::default(),
        &RoomConfig::default()
    )
    .is_none());
    assert!(evaluate_hand(
        &[27; 14],
        &[],
        Some(2),
        WinContext::default(),
        &RoomConfig::default()
    )
    .is_none());
    assert!(evaluate_hand(
        &wait(),
        &[],
        Some(2),
        WinContext::default(),
        &RoomConfig::default()
    )
    .is_none());
}
#[test]
fn dingque_is_required_and_private_until_everyone_chooses() {
    let mut g = Game::new(RoomConfig::default(), 2).unwrap();
    assert!(g
        .legal_actions(g.dealer)
        .iter()
        .all(|a| matches!(a, Action::DingQue { .. })));
    g.apply(0, Action::DingQue { suit: 2 }).unwrap();
    assert_eq!(g.view(1).players[0].missing_suit, None);
    assert_eq!(g.view(0).players[0].missing_suit, Some(2));
    for s in 1..4 {
        g.apply(s, Action::DingQue { suit: 1 }).unwrap();
    }
    assert_eq!(g.phase, Phase::Playing);
    assert_eq!(g.view(1).players[0].missing_suit, Some(2));
    let missing = g.players[g.dealer].missing_suit.unwrap();
    let has_missing = g.players[g.dealer]
        .hand
        .iter()
        .any(|tile| tile / 9 == missing);
    assert!(g
        .legal_actions(g.dealer)
        .iter()
        .filter_map(|a| if let Action::Discard { tile } = a {
            Some(tile / 9)
        } else {
            None
        })
        .all(|s| !has_missing || s == missing));
}
#[test]
fn multiple_hu_settle_before_continuation() {
    let mut g = fixture([vec![13], wait(), wait(), vec![]], Default::default());
    g.apply(0, Action::Discard { tile: 13 }).unwrap();
    assert_eq!(g.phase, Phase::Responding);
    g.apply(1, Action::Hu).unwrap();
    assert!(g.ledger.is_empty());
    g.apply(2, Action::Hu).unwrap();
    assert_eq!(g.winners, vec![1, 2]);
    assert_eq!(g.next_dealer, 0);
    assert_eq!(g.turn, 3);
    assert_eq!(g.players[0].score, -2);
    assert_eq!(g.players[1].score, 1);
    assert_eq!(g.players[2].score, 1);
    let source = g.players[1].win.as_ref().unwrap().source_event_id;
    assert_eq!(source, g.players[2].win.as_ref().unwrap().source_event_id);
    assert_eq!(g.events[(source - 1) as usize].kind, "discard");
    assert_eq!(g.tile_counts(), [4; 27]);
}

#[test]
fn repeated_same_tile_discards_have_distinct_win_source_events() {
    let mut g = fixture([vec![13], wait(), wait(), vec![13]], Default::default());
    // Two ordinary draws keep both waiting hands unchanged for this fixture.
    for _ in 0..2 {
        let position = g.wall.iter().position(|tile| *tile == 0).unwrap();
        g.wall.remove(position);
    }
    g.wall.extend([0, 0]);
    g.apply(0, Action::Discard { tile: 13 }).unwrap();
    g.apply(1, Action::Hu).unwrap();
    g.apply(2, Action::Pass).unwrap();
    g.apply(2, Action::Discard { tile: 0 }).unwrap();
    g.apply(3, Action::Discard { tile: 13 }).unwrap();
    g.apply(2, Action::Hu).unwrap();
    assert_ne!(
        g.players[1].win.as_ref().unwrap().source_event_id,
        g.players[2].win.as_ref().unwrap().source_event_id
    );
    assert_eq!(g.tile_counts(), [4; 27]);
}
#[test]
fn hu_has_priority_over_peng_regardless_of_response_order() {
    let mut g = fixture([vec![13], wait(), vec![13, 13], vec![]], Default::default());
    g.apply(0, Action::Discard { tile: 13 }).unwrap();
    g.apply(2, Action::Peng).unwrap();
    g.apply(1, Action::Hu).unwrap();
    assert!(g.players[2].melds.is_empty());
    assert!(g.players[1].won);
    assert_eq!(g.next_dealer, 1);
    assert_eq!(g.tile_counts(), [4; 27]);
}
#[test]
fn supplemental_kong_can_be_robbed_and_collects_no_kong_fee() {
    let h = vec![9, 10, 11, 11, 12, 15, 16, 17, 0, 1, 2, 8, 8];
    let meld = Meld {
        tile: 13,
        kind: MeldKind::Peng,
        from: Some(2),
    };
    let mut g = fixture(
        [vec![13], h, vec![], vec![]],
        [vec![meld], vec![], vec![], vec![]],
    );
    g.apply(0, Action::Kong { tile: 13 }).unwrap();
    assert_eq!(g.phase, Phase::Responding);
    assert_eq!(g.legal_actions(1), vec![Action::Pass, Action::Hu]);
    g.apply(1, Action::Hu).unwrap();
    assert_eq!(g.players[0].melds[0].kind, MeldKind::Peng);
    assert_eq!(g.ledger.len(), 1);
    assert_eq!(g.ledger[0].reason, "rob_kong");
    assert_eq!(g.ledger[0].multiplier, 2);
    assert_eq!(g.tile_counts(), [4; 27]);
}
#[test]
fn kong_payments_and_concealment() {
    let mut g = fixture(
        [vec![0, 0, 0, 0], vec![], vec![], vec![]],
        Default::default(),
    );
    g.players[3].won = true;
    g.apply(0, Action::Kong { tile: 0 }).unwrap();
    assert_eq!(g.ledger.len(), 2);
    assert_eq!(g.players[0].score, 4);
    assert_eq!(g.view(1).players[0].melds[0].tile, None);
    assert_eq!(g.view(0).players[0].melds[0].tile, Some(0));
    assert!(g
        .events
        .iter()
        .filter(|e| e.kind == "concealed_kong")
        .all(|e| e.tile.is_none()));
    assert!(g.current_after_kong);
    assert_eq!(g.tile_counts(), [4; 27]);
    let mut g = fixture([vec![9], vec![9, 9, 9], vec![], vec![]], Default::default());
    g.apply(0, Action::Discard { tile: 9 }).unwrap();
    g.apply(1, Action::Kong { tile: 9 }).unwrap();
    assert_eq!(g.ledger.len(), 1);
    assert_eq!(
        (g.ledger[0].from, g.ledger[0].to, g.ledger[0].amount),
        (0, 1, 2)
    );
    assert_eq!(g.tile_counts(), [4; 27]);
}
#[test]
fn successful_supplemental_kong_and_gangshanghua() {
    let meld = Meld {
        tile: 9,
        kind: MeldKind::Peng,
        from: Some(2),
    };
    let mut g = fixture(
        [vec![9], vec![], vec![], vec![]],
        [vec![meld], vec![], vec![], vec![]],
    );
    g.apply(0, Action::Kong { tile: 9 }).unwrap();
    assert_eq!(g.players[0].melds[0].kind, MeldKind::SupplementalKong);
    assert_eq!(g.ledger.len(), 3);
    assert!(g.ledger.iter().all(|e| e.multiplier == 1));
    assert_eq!(g.tile_counts(), [4; 27]);
}
#[test]
fn kong_context_applies_to_immediate_win_only() {
    let mut g = fixture([vec![13], wait(), vec![], vec![]], Default::default());
    g.current_after_kong = true;
    g.apply(0, Action::Discard { tile: 13 }).unwrap();
    assert!(!g.current_after_kong);
    g.apply(1, Action::Hu).unwrap();
    assert_eq!(g.ledger[0].reason, "kong_discard");
    assert_eq!(g.ledger[0].multiplier, 2);
    assert!(!g.current_after_kong);
    let mut g = fixture([plain(), vec![], vec![], vec![]], Default::default());
    g.current_after_kong = true;
    g.apply(0, Action::Hu).unwrap();
    assert_eq!(g.ledger.len(), 3);
    assert!(g.ledger.iter().all(|e| e.multiplier == 3));
    assert!(!g.current_after_kong);
}
#[test]
fn pending_rob_kong_response_restores_without_early_settlement() {
    let h = vec![9, 10, 11, 11, 12, 15, 16, 17, 0, 1, 2, 8, 8];
    let meld = Meld {
        tile: 13,
        kind: MeldKind::Peng,
        from: Some(3),
    };
    let mut g = fixture(
        [vec![13], h.clone(), h, vec![]],
        [vec![meld], vec![], vec![], vec![]],
    );
    g.apply(0, Action::Kong { tile: 13 }).unwrap();
    g.apply(1, Action::Hu).unwrap();
    assert!(g.ledger.is_empty());
    assert!(g.winners.is_empty());
    let mut restored: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
    g.apply(2, Action::Pass).unwrap();
    restored.apply(2, Action::Pass).unwrap();
    assert_eq!(
        serde_json::to_value(&g).unwrap(),
        serde_json::to_value(restored).unwrap()
    );
    assert_eq!(g.winners, vec![1]);
    assert_eq!(g.ledger.len(), 1);
    assert_eq!(g.tile_counts(), [4; 27]);
}
#[test]
fn exhausted_wall_flowers_dajiao_and_tax_refund() {
    let mut g = fixture([vec![18], wait(), vec![7], vec![]], Default::default());
    g.players[3].won = true;
    g.winners = vec![3];
    g.pay(1, 0, 2, "concealed_kong");
    g.pay(1, 2, 1, "supplemental_kong");
    g.finish(true);
    let flowers: Vec<_> = g
        .ledger
        .iter()
        .filter(|e| e.reason == "flower_penalty")
        .collect();
    assert_eq!(flowers.len(), 3);
    assert!(flowers.iter().all(|e| e.from == 0 && e.multiplier == 16));
    let dajiao: Vec<_> = g
        .ledger
        .iter()
        .filter(|e| e.reason == "not_ready_penalty")
        .collect();
    assert_eq!(dajiao.len(), 1);
    assert_eq!((dajiao[0].from, dajiao[0].to), (2, 1));
    let refunds: Vec<_> = g
        .ledger
        .iter()
        .filter(|e| e.reason == "kong_refund")
        .collect();
    assert_eq!(refunds.len(), 2);
    assert_eq!(refunds.iter().map(|e| e.amount).sum::<i64>(), 3);
    assert_eq!(g.players.iter().map(|p| p.score).sum::<i64>(), 0);
}
#[test]
fn waiting_value_ignores_wall_and_applies_cap_without_self_draw() {
    let mut g = fixture([wait(), vec![], vec![], vec![]], Default::default());
    g.wall.clear();
    assert_eq!(g.max_wait_value(0).unwrap().multiplier, 1);
    g.players[0].hand = vec![0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3];
    g.config.cap = Some(16);
    assert_eq!(g.max_wait_value(0).unwrap().multiplier, 16);
}
#[test]
fn aborted_game_has_no_flowers_or_refunds_and_is_idempotent() {
    let mut g = Game::new(RoomConfig::default(), 2).unwrap();
    g.pay(1, 0, 2, "concealed_kong");
    g.abort();
    let version = g.version;
    g.abort();
    assert_eq!(g.version, version);
    assert_eq!(g.ledger.len(), 1);
    assert_eq!(g.phase, Phase::Aborted);
}
#[test]
fn invalid_actions_never_mutate_game() {
    let mut g = Game::new(RoomConfig::default(), 2).unwrap();
    let before = serde_json::to_string(&g).unwrap();
    assert_eq!(g.apply(4, Action::Hu), Err(GameError::InvalidSeat));
    assert_eq!(
        g.apply(0, Action::Discard { tile: 255 }),
        Err(GameError::IllegalAction)
    );
    assert_eq!(
        g.apply(0, Action::DingQue { suit: 3 }),
        Err(GameError::IllegalAction)
    );
    assert_eq!(serde_json::to_string(&g).unwrap(), before);
}

#[test]
fn rules_version_is_frozen_and_unsupported_rules_cannot_resume() {
    let g = Game::new(RoomConfig::default(), 2).unwrap();
    assert_eq!(g.rules_version, RULES_VERSION);
    let mut restored: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
    restored.rules_version = "older-incompatible-rules".into();
    assert_eq!(restored.view(0).rules_version, "older-incompatible-rules");
    assert!(restored.legal_actions(0).is_empty());
    assert_eq!(
        restored.apply(0, Action::DingQue { suit: 2 }),
        Err(GameError::UnsupportedRules)
    );
    assert_eq!(restored.version, 0);
}
#[test]
fn config_and_fixed_wall_are_validated() {
    assert!(Game::new(
        RoomConfig {
            base_score: 0,
            ..Default::default()
        },
        0
    )
    .is_err());
    assert!(Game::new(
        RoomConfig {
            cap: Some(9),
            ..Default::default()
        },
        0
    )
    .is_err());
    assert!(Game::new(
        RoomConfig {
            flower_penalty: 64,
            ..Default::default()
        },
        0
    )
    .is_err());
    assert!(Game::new_with_dealer(RoomConfig::default(), 0, 4).is_err());
    assert!(Game::from_wall(RoomConfig::default(), vec![0; 108], 0).is_err());
    let wall = (0..27).flat_map(|t| [t; 4]).collect();
    let g = Game::from_wall(RoomConfig::default(), wall, 2).unwrap();
    assert_eq!(g.players[2].hand.len(), 14);
    assert_eq!(g.wall.len(), 55);
    assert_eq!(g.tile_counts(), [4; 27]);
}
#[test]
fn third_winner_ends_without_flowers_and_already_won_do_not_pay() {
    let mut g = fixture([plain(), vec![], vec![18], vec![]], Default::default());
    g.players[1].won = true;
    g.players[3].won = true;
    g.winners = vec![1, 3];
    g.apply(0, Action::Hu).unwrap();
    assert_eq!(g.phase, Phase::Finished);
    assert_eq!(g.ledger.len(), 1);
    assert_eq!(g.ledger[0].from, 2);
    assert_eq!(g.ledger[0].reason, "self_draw");
    assert_eq!(g.ledger[0].amount, 2);
}
#[test]
fn missing_suit_discard_is_forced_and_forbidden_to_claim() {
    let mut g = fixture(
        [vec![0, 18], vec![18, 18], vec![], vec![]],
        Default::default(),
    );
    assert_eq!(g.legal_actions(0), vec![Action::Discard { tile: 18 }]);
    g.apply(0, Action::Discard { tile: 18 }).unwrap();
    assert!(g.pending.is_none());
    assert!(g.players[1].melds.is_empty());
    assert_eq!(g.tile_counts(), [4; 27]);
}
#[test]
fn client_views_never_expose_opponent_hands_or_draws() {
    let g = Game::new(RoomConfig::default(), 42).unwrap();
    for seat in 0..4 {
        let view = g.view(seat);
        assert_eq!(view.own_hand, g.players[seat].hand);
        assert_eq!(view.own_last_draw, g.players[seat].last_draw);
        for other in 0..4 {
            assert_eq!(view.players[other].hand.is_some(), seat == other);
        }
        let json = serde_json::to_value(view).unwrap();
        assert!(json.get("wall").is_none());
        assert!(json.get("last_draw").is_none());
        assert!(json["players"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p.get("last_draw").is_none()));
    }
}
#[test]
fn seeded_simulations_conserve_tiles_scores_and_restore_exactly() {
    let mut won_count = 0;
    for seed in 0..160 {
        let mut g = Game::new(RoomConfig::default(), seed).unwrap();
        for step in 0..1000 {
            assert_eq!(
                g.tile_counts(),
                [4; 27],
                "tile conservation seed {seed} step {step}"
            );
            assert_eq!(g.players.iter().map(|p| p.score).sum::<i64>(), 0);
            for seat in 0..4 {
                let view = g.view(seat);
                let expected = if g.players[seat].won {
                    None
                } else {
                    g.players[seat].last_draw
                };
                assert_eq!(view.own_last_draw, expected);
                if let Some(tile) = view.own_last_draw {
                    assert!(view.own_hand.contains(&tile));
                }
                for event in view.events.iter().filter(|e| e.kind == "draw") {
                    assert!(
                        event.tile.is_none(),
                        "draw must never reveal a private tile"
                    );
                }
            }
            if g.is_over() {
                break;
            }
            let seat = g.acting_seats()[0];
            let action = g.bot_action(seat).expect("actor has action");
            assert!(g.legal_actions(seat).contains(&action));
            if step == 30 {
                let mut restored: Game =
                    serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
                restored.apply(seat, action.clone()).unwrap();
                g.apply(seat, action).unwrap();
                assert_eq!(
                    serde_json::to_value(restored).unwrap(),
                    serde_json::to_value(&g).unwrap()
                );
            } else {
                g.apply(seat, action).unwrap();
            }
        }
        assert!(g.is_over(), "seed {seed} did not terminate");
        won_count += g.winners.len();
        for seat in 0..4 {
            let received: i64 = g
                .ledger
                .iter()
                .filter(|e| e.to == seat)
                .map(|e| e.amount)
                .sum();
            let paid: i64 = g
                .ledger
                .iter()
                .filter(|e| e.from == seat)
                .map(|e| e.amount)
                .sum();
            assert_eq!(g.players[seat].score, received - paid);
        }
    }
    assert!(won_count > 10, "bots must actually complete hands");
}
