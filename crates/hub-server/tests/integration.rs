use futures_util::{SinkExt, StreamExt};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use tempfile::TempDir;
use tokio_tungstenite::tungstenite::Message;

struct Harness {
    dir: TempDir,
    host: Option<hub_server::HostHandle>,
    client: Client,
    base: String,
    tokens: Vec<String>,
}
impl Harness {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let host = hub_server::spawn_host(dir.path().into(), 0).await.unwrap();
        let base = format!("http://127.0.0.1:{}", host.info().port);
        let client = Client::new();
        let mut tokens = vec![];
        for name in ["青竹", "听雨", "晚风", "山月", "路人"] {
            let v: Value = client
                .post(format!("{base}/api/v1/session"))
                .json(&json!({"name":name}))
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .json()
                .await
                .unwrap();
            tokens.push(v["token"].as_str().unwrap().into());
        }
        Self {
            dir,
            host: Some(host),
            client,
            base,
            tokens,
        }
    }
    async fn get(&self, path: &str, p: usize) -> reqwest::Response {
        self.client
            .get(format!("{}{path}", self.base))
            .bearer_auth(&self.tokens[p])
            .send()
            .await
            .unwrap()
    }
    async fn post(&self, path: &str, p: usize, value: Value) -> reqwest::Response {
        self.client
            .post(format!("{}{path}", self.base))
            .bearer_auth(&self.tokens[p])
            .json(&value)
            .send()
            .await
            .unwrap()
    }
    async fn room(&self, id: &str, p: usize) -> Value {
        self.get(&format!("/api/v1/rooms/{id}"), p)
            .await
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap()
    }
    async fn command(&self, id: &str, p: usize, action: Value) -> Value {
        let room = self.room(id, p).await;
        let res=self.post(&format!("/api/v1/rooms/{id}/command"),p,json!({"id":uuid::Uuid::new_v4().to_string(),"version":room["version"],"action":action})).await;
        let status = res.status();
        let body: Value = res.json().await.unwrap();
        assert!(status.is_success(), "{status}: {body}");
        body
    }
    async fn table(&self) -> String {
        let r: Value = self
            .post(
                "/api/v1/rooms",
                0,
                json!({"name":"周末茶局","config":{"turn_seconds":0,"response_seconds":0}}),
            )
            .await
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        let id = r["id"].as_str().unwrap().to_owned();
        for p in 1..4 {
            self.post(&format!("/api/v1/rooms/{id}/join"), p, json!({"seat":p}))
                .await
                .error_for_status()
                .unwrap();
            self.command(&id, p, json!({"type":"ready","ready":true}))
                .await;
        }
        self.command(&id, 0, json!({"type":"start"})).await;
        for p in 0..4 {
            let (mut socket, _) =
                tokio_tungstenite::connect_async(self.base.replace("http://", "ws://") + "/ws/v1")
                    .await
                    .unwrap();
            socket
                .send(Message::Text(
                    json!({"type":"auth","token":self.tokens[p],"roomId":id})
                        .to_string()
                        .into(),
                ))
                .await
                .unwrap();
            assert!(socket.next().await.unwrap().is_ok());
            tokio::spawn(async move {
                while let Some(Ok(message)) = socket.next().await {
                    match message {
                        Message::Ping(p) => {
                            if socket.send(Message::Pong(p)).await.is_err() {
                                break;
                            }
                        }
                        Message::Close(_) => break,
                        _ => {}
                    }
                }
            });
        }
        id
    }
    async fn close(mut self) {
        self.host.take().unwrap().shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn auth_idempotency_hidden_hands_and_restart() {
    let mut h = Harness::new().await;
    let id = h.table().await;
    let public: Value = h
        .client
        .get(format!("{}/api/v1/host", h.base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(public.get("adminToken").is_none());
    assert_eq!(
        h.client
            .get(format!("{}/api/v1/rooms", h.base))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let first = h.room(&id, 0).await;
    assert_eq!(first["game"]["phase"], "ding_que");
    assert!(first["game"].get("wall").is_none());
    for i in 1..4 {
        assert!(first["game"]["players"][i]["hand"].is_null());
    }
    let command = json!({"id":"same-request","version":first["version"],"action":{"type":"game","action":{"type":"ding_que","suit":2}}});
    let route = format!("/api/v1/rooms/{id}/command");
    let one: Value = h
        .post(&route, 0, command.clone())
        .await
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let two: Value = h
        .post(&route, 0, command.clone())
        .await
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(one["version"], two["version"]);
    let mut stale = command;
    stale["id"] = json!("new-id-stale-version");
    assert_eq!(
        h.post(&route, 0, stale).await.status(),
        StatusCode::CONFLICT
    );
    assert!(h.room(&id, 4).await["game"].is_null());
    let hist_id = first["historyId"].as_str().unwrap();
    assert_eq!(
        h.get(&format!("/api/v1/history/{hist_id}/replay"), 0)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    h.host.take().unwrap().shutdown().await.unwrap();
    let host = hub_server::spawn_host(h.dir.path().into(), 0)
        .await
        .unwrap();
    h.base = format!("http://127.0.0.1:{}", host.info().port);
    h.host = Some(host);
    let restored = h.room(&id, 0).await;
    assert_eq!(restored["game"], one["game"]);
    assert_eq!(restored["paused"], true);
    h.command(&id, 0, json!({"type":"resume"})).await;
    // Dedup keys survive process-level server restarts.
    let again:Value=h.post(&route,0,json!({"id":"same-request","version":0,"action":{"type":"game","action":{"type":"ding_que","suit":2}}})).await.error_for_status().unwrap().json().await.unwrap();
    assert_eq!(again["game"], one["game"]);
    h.close().await;
}

#[tokio::test]
async fn four_tables_complete_conserve_scores_and_export_authorized_replays() {
    let h = Harness::new().await;
    let mut ids = vec![];
    for _ in 0..4 {
        ids.push(h.table().await);
    }
    for _ in 0..600 {
        let mut all_finished = true;
        for id in &ids {
            let r = h.room(id, 0).await;
            if r["status"] == "finished" {
                continue;
            }
            all_finished = false;
            let mut moved = false;
            for p in 0..4 {
                let r = h.room(id, p).await;
                let actions = r["game"]["legal_actions"].as_array().unwrap();
                if actions.is_empty() {
                    continue;
                }
                let chosen = actions
                    .iter()
                    .find(|a| a["type"] == "hu")
                    .or_else(|| actions.iter().find(|a| a["type"] == "kong"))
                    .or_else(|| actions.iter().find(|a| a["type"] == "peng"))
                    .unwrap_or(&actions[0])
                    .clone();
                h.command(id, p, json!({"type":"game","action":chosen}))
                    .await;
                moved = true;
                break;
            }
            assert!(
                moved,
                "active game must always offer at least one legal action"
            );
        }
        if all_finished {
            break;
        }
    }
    for id in &ids {
        let r = h.room(id, 0).await;
        assert_eq!(r["status"], "finished");
        let total: i64 = r["seats"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["score"].as_i64().unwrap())
            .sum();
        assert_eq!(total, 0);
        let history = r["historyId"].as_str().unwrap();
        let route = format!("/api/v1/history/{history}/replay");
        let replay: Value = h
            .get(&route, 0)
            .await
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        let frames = replay["frames"].as_array().unwrap();
        assert!(frames.len() > 20);
        assert_eq!(
            frames.last().unwrap()["game"]["ledger"],
            r["game"]["ledger"]
        );
        assert_eq!(h.get(&route, 4).await.status(), StatusCode::FORBIDDEN);
        let csv = h
            .get(&format!("/api/v1/history/{history}/export.csv"), 0)
            .await
            .error_for_status()
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(csv.contains("付款人,收款人"));
        let before: Value = h.get("/api/v1/history", 0).await.json().await.unwrap();
        for p in 0..4 {
            h.command(
                id,
                p,
                json!({"type":"confirmSettlement","round":r["round"]}),
            )
            .await;
        }
        h.command(id, 1, json!({"type":"leave"})).await;
        h.post(&format!("/api/v1/rooms/{id}/join"), 4, json!({"seat":1}))
            .await
            .error_for_status()
            .unwrap();
        assert!(
            h.room(id, 4).await["game"].is_null(),
            "new occupant cannot see old completed hands"
        );
        let standings = h.room(id, 0).await["standings"].as_array().unwrap().clone();
        assert!(
            standings.iter().any(|s| s["name"] == "听雨"),
            "departed players remain on the session leaderboard"
        );
        let after: Value = h.get("/api/v1/history", 0).await.json().await.unwrap();
        assert_eq!(
            before, after,
            "finished history must not change when seats change"
        );
        assert_eq!(
            h.get(&route, 4).await.status(),
            StatusCode::FORBIDDEN,
            "new occupant cannot read former player's history"
        );
    }
    h.close().await;
}

#[tokio::test]
async fn aborted_hand_preserves_settled_scores_without_flow_penalties() {
    let h = Harness::new().await;
    let id = h.table().await;
    let r = h.command(&id, 0, json!({"type":"abort"})).await;
    assert_eq!(r["status"], "archived");
    assert_eq!(r["game"]["phase"], "aborted");
    assert_eq!(r["game"]["ledger"], json!([]));
    h.close().await;
}

#[tokio::test]
async fn websocket_presence_is_room_scoped_and_shutdown_closes_clients() {
    let mut h = Harness::new().await;
    let mut ids = vec![];
    for _ in 0..2 {
        let r: Value = h
            .post("/api/v1/rooms", 0, json!({"name":"连接测试"}))
            .await
            .json()
            .await
            .unwrap();
        ids.push(r["id"].as_str().unwrap().to_owned());
    }
    let (mut socket, _) =
        tokio_tungstenite::connect_async(h.base.replace("http://", "ws://") + "/ws/v1")
            .await
            .unwrap();
    socket
        .send(Message::Text(
            json!({"type":"auth","token":h.tokens[0],"roomId":ids[0]})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let _ = socket.next().await.unwrap().unwrap();
    assert_eq!(h.room(&ids[0], 0).await["seats"][0]["connected"], true);
    assert_eq!(h.room(&ids[1], 0).await["seats"][0]["connected"], false);
    assert!(
        hub_server::spawn_host(h.dir.path().into(), 0)
            .await
            .is_err(),
        "two servers must not write the same save directory"
    );
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        h.host.take().unwrap().shutdown(),
    )
    .await
    .unwrap()
    .unwrap();
    let closed = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while let Some(message) = socket.next().await {
            if message.is_err() || matches!(message, Ok(Message::Close(_))) {
                break;
            }
        }
    })
    .await;
    assert!(
        closed.is_ok(),
        "old sockets must close before an original-host restart"
    );
}

#[tokio::test]
async fn failed_persistence_rolls_back_state_scores_and_dedup_key() {
    let h = Harness::new().await;
    let id = h.table().await;
    let before = h.room(&id, 0).await;
    let db = rusqlite::Connection::open(h.dir.path().join("hub.sqlite3")).unwrap();
    db.execute_batch("CREATE TRIGGER fail_frame BEFORE INSERT ON frames BEGIN SELECT RAISE(ABORT,'injected disk failure'); END;").unwrap();
    let payload = json!({"id":"rollback-command","version":before["version"],"action":{"type":"game","action":{"type":"ding_que","suit":2}}});
    let route = format!("/api/v1/rooms/{id}/command");
    assert_eq!(
        h.post(&route, 0, payload.clone()).await.status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let after = h.room(&id, 0).await;
    assert_eq!(before["version"], after["version"]);
    assert_eq!(before["game"], after["game"]);
    db.execute_batch("DROP TRIGGER fail_frame;").unwrap();
    let committed: Value = h
        .post(&route, 0, payload)
        .await
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(committed["game"]["players"][0]["missing_suit"], 2);
    h.close().await;
}

#[tokio::test]
async fn occupied_loopback_port_is_rejected_instead_of_shadowing() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    assert!(
        hub_server::spawn_host(dir.path().into(), listener.local_addr().unwrap().port())
            .await
            .is_err()
    );
}

async fn finish_mahjong(h: &Harness, id: &str) -> Value {
    for _ in 0..600 {
        let r = h.room(id, 0).await;
        if r["status"] == "finished" {
            return r;
        }
        let mut moved = false;
        for p in 0..4 {
            let r = h.room(id, p).await;
            let actions = r["game"]["legal_actions"].as_array().unwrap();
            if let Some(action) = actions
                .iter()
                .find(|a| a["type"] == "hu")
                .or_else(|| actions.iter().find(|a| a["type"] == "kong"))
                .or_else(|| actions.iter().find(|a| a["type"] == "peng"))
                .or_else(|| actions.first())
            {
                h.command(id, p, json!({"type":"game","action":action}))
                    .await;
                moved = true;
                break;
            }
        }
        assert!(moved);
    }
    panic!("game failed to finish");
}

#[tokio::test]
async fn settlement_requires_all_players_is_atomic_and_survives_restart() {
    let mut h = Harness::new().await;
    let id = h.table().await;
    let route = format!("/api/v1/rooms/{id}/command");
    let playing = h.room(&id, 0).await;
    let early = h.post(&route, 0, json!({"id":"too-early","version":playing["version"],"action":{"type":"confirmSettlement","round":1}})).await;
    assert_eq!(early.status(), StatusCode::BAD_REQUEST);
    let ended = finish_mahjong(&h, &id).await;
    assert_eq!(
        ended["settlement"]["acknowledged"],
        json!([false, false, false, false])
    );
    assert!(ended["seats"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["ready"] == false));
    assert!(h.room(&id, 4).await["settlement"].is_null());
    let history = ended["historyId"].as_str().unwrap();
    let replay_route = format!("/api/v1/history/{history}/replay");
    let replay_before: Value = h.get(&replay_route, 0).await.json().await.unwrap();
    for action in [
        json!({"type":"start"}),
        json!({"type":"ready","ready":true}),
        json!({"type":"leave"}),
        json!({"type":"confirmSettlement","round":2}),
    ] {
        let response = h.post(&route, 0, json!({"id":uuid::Uuid::new_v4().to_string(),"version":ended["version"],"action":action})).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    let outsider = h.post(&route, 4, json!({"id":"outsider-confirm","version":ended["version"],"action":{"type":"confirmSettlement","round":1}})).await;
    assert!(!outsider.status().is_success());
    let db = rusqlite::Connection::open(h.dir.path().join("hub.sqlite3")).unwrap();
    db.execute_batch("CREATE TRIGGER fail_confirmation BEFORE UPDATE ON rooms BEGIN SELECT RAISE(ABORT,'injected save failure'); END;").unwrap();
    let payload = json!({"id":"persistent-confirm","version":ended["version"],"action":{"type":"confirmSettlement","round":1}});
    assert_eq!(
        h.post(&route, 0, payload.clone()).await.status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(h.room(&id, 0).await["settlement"], ended["settlement"]);
    db.execute_batch("DROP TRIGGER fail_confirmation;").unwrap();
    let first: Value = h
        .post(&route, 0, payload.clone())
        .await
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let duplicate: Value = h
        .post(&route, 0, payload)
        .await
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(first["version"], duplicate["version"]);
    // Both independent clients intentionally submit the same stale room version.
    let (one, two) = tokio::join!(
        h.post(&route,1,json!({"id":"confirm-1","version":ended["version"],"action":{"type":"confirmSettlement","round":1}})),
        h.post(&route,2,json!({"id":"confirm-2","version":ended["version"],"action":{"type":"confirmSettlement","round":1}}))
    );
    assert!(one.status().is_success() && two.status().is_success());
    let partial = h.room(&id, 0).await;
    assert_eq!(
        partial["settlement"]["acknowledged"],
        json!([true, true, true, false])
    );
    let denied = h.post(&route,0,json!({"id":"start-before-fourth","version":partial["version"],"action":{"type":"start"}})).await;
    assert_eq!(denied.status(), StatusCode::BAD_REQUEST);
    h.host.take().unwrap().shutdown().await.unwrap();
    let host = hub_server::spawn_host(h.dir.path().into(), 0)
        .await
        .unwrap();
    h.base = format!("http://127.0.0.1:{}", host.info().port);
    h.host = Some(host);
    let restored = h.room(&id, 0).await;
    assert_eq!(restored["settlement"], partial["settlement"]);
    assert_eq!(restored["game"], ended["game"]);
    // Offline human seats are still humans, not auto-confirming bots.
    assert_eq!(restored["settlement"]["acknowledged"][3], false);
    // Recovery to a fresh browser credential resets only the recovered seat.
    let new_player: Value = h
        .post("/api/v1/session", 4, json!({"name":"恢复山月"}))
        .await
        .json()
        .await
        .unwrap();
    h.tokens
        .push(new_player["token"].as_str().unwrap().to_owned());
    h.command(
        &id,
        0,
        json!({"type":"rebind","seat":3,"playerId":new_player["playerId"]}),
    )
    .await;
    let rebounded = h.room(&id, 5).await;
    assert_eq!(
        rebounded["settlement"]["acknowledged"],
        json!([true, true, true, false])
    );
    assert!(!h.post(&route,3,json!({"id":"old-identity","version":rebounded["version"],"action":{"type":"confirmSettlement","round":1}})).await.status().is_success());
    let final_confirmation = h
        .command(&id, 5, json!({"type":"confirmSettlement","round":1}))
        .await;
    assert_eq!(final_confirmation["settlement"]["allAcknowledged"], true);
    let replay_after: Value = h.get(&replay_route, 0).await.json().await.unwrap();
    assert_eq!(
        replay_before, replay_after,
        "confirmations must not alter finished replay or ledger"
    );
    let next = h.command(&id, 0, json!({"type":"start"})).await;
    assert_eq!(next["round"], 2);
    assert!(next["settlement"].is_null());
    assert!(!h.post(&route,0,json!({"id":"delayed-old-round","version":ended["version"],"action":{"type":"confirmSettlement","round":1}})).await.status().is_success());
    assert_eq!(h.room(&id, 0).await["version"], next["version"]);
    h.close().await;
}

#[tokio::test]
async fn settlement_bots_auto_confirm_but_aborted_room_cannot_restart() {
    let h = Harness::new().await;
    let r: Value = h
        .post("/api/v1/rooms", 0, json!({"name":"电脑确认"}))
        .await
        .json()
        .await
        .unwrap();
    let id = r["id"].as_str().unwrap();
    for seat in 1..4 {
        h.command(id, 0, json!({"type":"addBot","seat":seat})).await;
    }
    h.command(id, 0, json!({"type":"start"})).await;
    let result = h.command(id, 0, json!({"type":"abort"})).await;
    assert_eq!(
        result["settlement"]["acknowledged"],
        json!([false, true, true, true])
    );
    let confirmed = h
        .command(id, 0, json!({"type":"confirmSettlement","round":1}))
        .await;
    assert_eq!(confirmed["settlement"]["allAcknowledged"], true);
    let response = h
        .post(
            &format!("/api/v1/rooms/{id}/command"),
            0,
            json!({"id":"start-archived","version":confirmed["version"],"action":{"type":"start"}}),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    h.close().await;
}
