use futures_util::{SinkExt, StreamExt};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use tempfile::TempDir;
use tokio_tungstenite::tungstenite::Message;

struct Hub {
    dir: TempDir,
    host: Option<hub_server::HostHandle>,
    base: String,
    client: Client,
    tokens: Vec<String>,
}
impl Hub {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let host = hub_server::spawn_host(dir.path().into(), 0).await.unwrap();
        let mut h = Self {
            base: format!("http://127.0.0.1:{}", host.info().port),
            dir,
            host: Some(host),
            client: Client::new(),
            tokens: vec![],
        };
        for i in 0..5 {
            let r: Value = h
                .client
                .post(format!("{}/api/v1/session", h.base))
                .json(&json!({"name":format!("亲友{i}")}))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            h.tokens.push(r["token"].as_str().unwrap().into());
        }
        h
    }
    async fn get(&self, path: &str, p: usize) -> reqwest::Response {
        self.client
            .get(format!("{}/api/v1{path}", self.base))
            .bearer_auth(&self.tokens[p])
            .send()
            .await
            .unwrap()
    }
    async fn post(&self, path: &str, p: usize, body: Value) -> reqwest::Response {
        self.client
            .post(format!("{}/api/v1{path}", self.base))
            .bearer_auth(&self.tokens[p])
            .json(&body)
            .send()
            .await
            .unwrap()
    }
    async fn room(&self, id: &str, p: usize) -> Value {
        self.get(&format!("/rooms/{id}"), p)
            .await
            .json()
            .await
            .unwrap()
    }
    async fn command(&self, id: &str, p: usize, action: Value) -> Value {
        let r = self.room(id, p).await;
        let res=self.post(&format!("/rooms/{id}/command"),p,json!({"id":uuid::Uuid::new_v4().to_string(),"version":r["version"],"action":action})).await;
        let status = res.status();
        let result: Value = res.json().await.unwrap();
        assert!(status.is_success(), "{status} {result}");
        result
    }
    async fn create(&self, game: &str) -> String {
        let r: Value = self
            .post(
                "/rooms",
                0,
                json!({"name":game,"gameId":game,"config":{"turn_seconds":0,"response_seconds":0}}),
            )
            .await
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        r["id"].as_str().unwrap().into()
    }
    async fn start_humans(&self, id: &str, seats: usize) {
        for i in 1..seats {
            self.post(&format!("/rooms/{id}/join"), i, json!({"seat":i}))
                .await
                .error_for_status()
                .unwrap();
            self.command(id, i, json!({"type":"ready","ready":true}))
                .await;
        }
        self.command(id, 0, json!({"type":"start"})).await;
    }
    async fn restart(&mut self) {
        self.host.take().unwrap().shutdown().await.unwrap();
        let host = hub_server::spawn_host(self.dir.path().into(), 0)
            .await
            .unwrap();
        self.base = format!("http://127.0.0.1:{}", host.info().port);
        self.host = Some(host);
    }
    async fn close(mut self) {
        self.host.take().unwrap().shutdown().await.unwrap();
    }
    fn private_game(&self, id: &str) -> Value {
        let db = rusqlite::Connection::open(self.dir.path().join("hub.sqlite3")).unwrap();
        let raw: String = db
            .query_row("SELECT state FROM rooms WHERE id=?1", [id], |r| r.get(0))
            .unwrap();
        serde_json::from_str::<Value>(&raw).unwrap()["game"].clone()
    }
}

#[tokio::test]
async fn mixed_rooms_capacity_privacy_dispatch_and_restart() {
    let mut h = Hub::new().await;
    let games: Value = h.get("/games", 0).await.json().await.unwrap();
    assert_eq!(games.as_array().unwrap().len(), 3);
    assert_eq!(
        h.post("/rooms", 0, json!({"name":"未安装","gameId":"unknown"}))
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    let mj = h.create("sichuan-blood-battle").await;
    let ddz = h.create("doudizhu").await;
    let gd = h.create("guandan").await;
    assert_eq!(h.room(&mj, 0).await["seats"].as_array().unwrap().len(), 4);
    assert_eq!(h.room(&ddz, 0).await["seats"].as_array().unwrap().len(), 3);
    for seat in [3, 4, usize::MAX] {
        assert_eq!(
            h.post(&format!("/rooms/{ddz}/join"), 1, json!({"seat":seat}))
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
        let r = h.room(&ddz, 0).await;
        assert_eq!(h.post(&format!("/rooms/{ddz}/command"),0,json!({"id":format!("bad-{seat}"),"version":r["version"],"action":{"type":"addBot","seat":seat}})).await.status(),StatusCode::BAD_REQUEST);
    }
    h.start_humans(&mj, 4).await;
    h.start_humans(&ddz, 3).await;
    h.start_humans(&gd, 4).await;
    let before_gd = h.room(&gd, 0).await;
    assert_eq!(
        h.post(&format!("/rooms/{ddz}/join"), 3, json!({}))
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    let list: Value = h.get("/rooms", 0).await.json().await.unwrap();
    assert!(list
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["status"] == "playing"));
    let public = h.room(&ddz, 4).await;
    assert!(public["game"].is_null());
    assert!(public["invite"].is_null());
    let r = h.room(&ddz, 0).await;
    let turn = r["game"]["turn"].as_u64().unwrap() as usize;
    assert!(r["game"]["bottom"].is_null());
    assert!(r["game"]["seed"].is_null());
    assert!(r["game"]["players"][1]["hand"].is_null());
    let action = json!({"id":"bid-once","version":r["version"],"action":{"type":"game","action":{"type":"bid","score":3}}});
    let route = format!("/rooms/{ddz}/command");
    let before_mj = h.room(&mj, 0).await;
    let first: Value = h
        .post(&route, turn, action.clone())
        .await
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let repeated: Value = h
        .post(&route, turn, action.clone())
        .await
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(first["version"], repeated["version"]);
    assert_eq!(first["game"], repeated["game"]);
    let invalid = json!({"id":"wrong-game","version":first["version"],"action":{"type":"game","action":{"type":"ding_que","suit":0}}});
    assert_eq!(
        h.post(&route, turn, invalid).await.status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(h.room(&ddz, turn).await["game"], first["game"]);
    assert_eq!(h.room(&mj, 0).await["game"], before_mj["game"]);
    let bad_mj = json!({"id":"wrong-game-2","version":before_mj["version"],"action":{"type":"game","action":{"type":"bid","score":3}}});
    assert_eq!(
        h.post(&format!("/rooms/{mj}/command"), 0, bad_mj)
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    h.restart().await;
    for id in [&mj, &ddz, &gd] {
        let r = h.room(id, 0).await;
        assert_eq!(r["paused"], true);
        assert!(r["deadline"].is_null());
    }
    assert_eq!(h.room(&ddz, turn).await["game"], first["game"]);
    assert_eq!(h.room(&gd, 0).await["game"], before_gd["game"]);
    h.command(&ddz, 0, json!({"type":"resume"})).await;
    let card = first["game"]["own_hand"][0].clone();
    h.command(
        &ddz,
        turn,
        json!({"type":"game","action":{"type":"play","cards":[card]}}),
    )
    .await;
    assert_eq!(h.room(&mj, 0).await["paused"], true);
    h.close().await;
}

#[tokio::test]
async fn mixed_rounds_finish_export_replay_and_next_round() {
    let h = Hub::new().await;
    let mj = h.create("sichuan-blood-battle").await;
    let ddz = h.create("doudizhu").await;
    h.start_humans(&mj, 4).await;
    h.start_humans(&ddz, 3).await;
    for _ in 0..600 {
        let mut active = false;
        for id in [&ddz, &mj] {
            let state = h.private_game(id);
            let next = if id == &ddz {
                let g: doudizhu::Game = serde_json::from_value(state).unwrap();
                if !g.running() {
                    None
                } else {
                    Some((
                        g.turn,
                        serde_json::to_value(g.bot_action(g.turn).unwrap()).unwrap(),
                    ))
                }
            } else {
                let g: mahjong::Game = serde_json::from_value(state).unwrap();
                g.acting_seats().first().and_then(|&s| {
                    g.bot_action(s)
                        .map(|a| (s, serde_json::to_value(a).unwrap()))
                })
            };
            if let Some((seat, action)) = next {
                active = true;
                h.command(id, seat, json!({"type":"game","action":action}))
                    .await;
            }
        }
        if !active {
            break;
        }
    }
    for (id, seats, game_id) in [(&mj, 4, "sichuan-blood-battle"), (&ddz, 3, "doudizhu")] {
        let r = h.room(id, 0).await;
        assert_eq!(r["status"], "finished");
        assert_eq!(
            r["seats"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s["score"].as_i64().unwrap())
                .sum::<i64>(),
            0
        );
        let history = r["historyId"].as_str().unwrap();
        let route = format!("/history/{history}/replay");
        let replay: Value = h
            .get(&route, 0)
            .await
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(replay["gameId"], game_id);
        assert_eq!(replay["players"].as_array().unwrap().len(), seats);
        assert_eq!(h.get(&route, 4).await.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            h.get(&format!("{route}?seat={seats}"), 0).await.status(),
            StatusCode::BAD_REQUEST
        );
        let frames = replay["frames"].as_array().unwrap();
        assert!(frames.len() > 10);
        assert!(frames
            .iter()
            .all(|f| f["hands"].as_array().unwrap().len() == seats
                && f["game"]["legal_actions"].as_array().unwrap().is_empty()));
        let csv = h
            .get(&format!("/history/{history}/export.csv"), 0)
            .await
            .error_for_status()
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(csv.contains("分数"));
        if id == &ddz {
            assert!(csv.contains("地主获胜") || csv.contains("农民获胜"));
            let ledger = r["game"]["ledger"].as_array().unwrap();
            assert_eq!(ledger.len(), 2);
            // The Mahjong-only confirmation gate must never block landlord rooms.
            let next = h.command(id, 0, json!({"type":"start"})).await;
            assert_eq!(next["round"], 2);
            assert_eq!(next["game"]["phase"], "bidding");
            for i in 0..3 {
                assert_eq!(next["seats"][i]["score"], r["seats"][i]["score"]);
            }
        }
    }
    let history: Value = h.get("/history", 0).await.json().await.unwrap();
    assert_eq!(history.as_array().unwrap().len(), 2);
    h.close().await;
}

#[tokio::test]
async fn landlord_bots_and_websocket_snapshot_share_room_dispatch() {
    let h = Hub::new().await;
    let id = h.create("doudizhu").await;
    h.command(&id, 0, json!({"type":"addBot"})).await;
    h.command(&id, 0, json!({"type":"addBot"})).await;
    h.command(&id, 0, json!({"type":"start"})).await;
    let (mut ws, _) =
        tokio_tungstenite::connect_async(h.base.replace("http://", "ws://") + "/ws/v1")
            .await
            .unwrap();
    ws.send(Message::Text(
        json!({"type":"auth","token":h.tokens[0],"roomId":id})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    let first = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if let Some(Ok(Message::Text(text))) = ws.next().await {
                break serde_json::from_str::<Value>(&text).unwrap();
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(first["room"]["gameId"], "doudizhu");
    assert!(first["room"]["game"]["players"][1]["hand"].is_null());
    // Allow whichever seat is first to act; observe server-driven bot turns.
    let r = h.room(&id, 0).await;
    if r["game"]["turn"] == 0 {
        h.command(
            &id,
            0,
            json!({"type":"game","action":{"type":"bid","score":0}}),
        )
        .await;
    }
    let initial = h.room(&id, 0).await["game"]["version"].as_u64().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if h.room(&id, 0).await["game"]["version"].as_u64().unwrap() > initial {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();
    ws.close(None).await.unwrap();
    h.close().await;
}

#[tokio::test]
async fn landlord_storage_failure_rolls_back_bid_and_dedup() {
    let h = Hub::new().await;
    let id = h.create("doudizhu").await;
    h.start_humans(&id, 3).await;
    let before = h.room(&id, 0).await;
    let turn = before["game"]["turn"].as_u64().unwrap() as usize;
    let db = rusqlite::Connection::open(h.dir.path().join("hub.sqlite3")).unwrap();
    db.execute_batch("CREATE TRIGGER fail_ddz BEFORE INSERT ON frames BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    let body = json!({"id":"atomic-bid","version":before["version"],"action":{"type":"game","action":{"type":"bid","score":3}}});
    let route = format!("/rooms/{id}/command");
    assert_eq!(
        h.post(&route, turn, body.clone()).await.status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let after = h.room(&id, 0).await;
    assert_eq!(after["game"], before["game"]);
    assert_eq!(after["version"], before["version"]);
    db.execute_batch("DROP TRIGGER fail_ddz;").unwrap();
    assert!(h.post(&route, turn, body).await.status().is_success());
    h.close().await;
}

#[tokio::test]
async fn guandan_four_seats_private_atomic_recovery_replay_and_tribute() {
    let mut h = Hub::new().await;
    let id = h.create("guandan").await;
    assert_eq!(h.room(&id, 0).await["seats"].as_array().unwrap().len(), 4);
    assert_eq!(
        h.post(&format!("/rooms/{id}/join"), 1, json!({"seat":4}))
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    h.start_humans(&id, 4).await;
    let before = h.room(&id, 0).await;
    let turn = before["game"]["turn"].as_u64().unwrap() as usize;
    let own = h.room(&id, turn).await;
    for i in 0..4 {
        assert_eq!(own["game"]["players"][i]["hand"].is_null(), i != turn);
    }
    assert_eq!(own["game"]["own_hand"].as_array().unwrap().len(), 27);
    assert!(h.room(&id, 4).await["game"].is_null());
    let g: guandan::Game = serde_json::from_value(h.private_game(&id)).unwrap();
    let action = serde_json::to_value(g.bot_action(turn).unwrap()).unwrap();
    let body =
        json!({"id":"gd-atomic","version":own["version"],"action":{"type":"game","action":action}});
    let route = format!("/rooms/{id}/command");
    let db = rusqlite::Connection::open(h.dir.path().join("hub.sqlite3")).unwrap();
    db.execute_batch("CREATE TRIGGER fail_gd BEFORE INSERT ON frames BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    assert_eq!(
        h.post(&route, turn, body.clone()).await.status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(h.room(&id, turn).await["game"], own["game"]);
    db.execute_batch("DROP TRIGGER fail_gd;").unwrap();
    let first: Value = h
        .post(&route, turn, body.clone())
        .await
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let again: Value = h
        .post(&route, turn, body)
        .await
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(first["version"], again["version"]);
    let current_turn = first["game"]["turn"].as_u64().unwrap() as usize;
    assert_eq!(h.post(&route,current_turn,json!({"id":"gd-wrong","version":first["version"],"action":{"type":"game","action":{"type":"bid","score":3}}})).await.status(),StatusCode::BAD_REQUEST);
    h.restart().await;
    assert_eq!(h.room(&id, turn).await["game"], first["game"]);
    assert_eq!(h.room(&id, 0).await["paused"], true);
    h.command(&id, 0, json!({"type":"resume"})).await;
    let (mut ws, _) =
        tokio_tungstenite::connect_async(h.base.replace("http://", "ws://") + "/ws/v1")
            .await
            .unwrap();
    ws.send(Message::Text(
        json!({"type":"auth","token":h.tokens[0],"roomId":id})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    let snapshot = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if let Some(Ok(Message::Text(t))) = ws.next().await {
                break serde_json::from_str::<Value>(&t).unwrap();
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(snapshot["room"]["gameId"], "guandan");
    assert!(snapshot["room"]["game"]["players"][2]["hand"].is_null());
    for _ in 0..800 {
        let g: guandan::Game = serde_json::from_value(h.private_game(&id)).unwrap();
        if !g.running() {
            break;
        }
        h.command(
            &id,
            g.turn,
            json!({"type":"game","action":g.bot_action(g.turn).unwrap()}),
        )
        .await;
    }
    let finished = h.room(&id, 0).await;
    assert_eq!(finished["status"], "finished");
    assert_eq!(finished["game"]["ledger"].as_array().unwrap().len(), 4);
    assert_eq!(
        finished["seats"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["score"].as_i64().unwrap())
            .sum::<i64>(),
        0
    );
    let history = finished["historyId"].as_str().unwrap();
    let replay: Value = h
        .get(&format!("/history/{history}/replay"), 0)
        .await
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(replay["gameId"], "guandan");
    assert!(replay["frames"].as_array().unwrap().len() > 10);
    assert!(replay["frames"]
        .as_array()
        .unwrap()
        .iter()
        .all(|f| f["hands"].as_array().unwrap().len() == 4
            && f["game"]["legal_actions"].as_array().unwrap().is_empty()));
    assert_eq!(
        h.get(&format!("/history/{history}/replay"), 4)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let csv = h
        .get(&format!("/history/{history}/export.csv"), 0)
        .await
        .error_for_status()
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(csv.contains("掼蛋队伍获胜"));
    let next = h.command(&id, 0, json!({"type":"start"})).await;
    assert_eq!(next["round"], 2);
    assert_eq!(next["game"]["level"], finished["game"]["next_level"]);
    for i in 0..4 {
        assert_eq!(next["seats"][i]["score"], finished["seats"][i]["score"]);
    }
    h.restart().await;
    assert_eq!(h.room(&id, 0).await["game"], next["game"]);
    ws.close(None).await.ok();
    h.close().await;
}
