use crate::game::RoomGame;
use crate::{model::*, registry, storage, AppState};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, Query, State, WebSocketUpgrade,
    },
    http::{header, HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
use mahjong::RoomConfig;
use rusqlite::{params, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;

type ApiResult<T> = Result<T, ApiError>;
pub(crate) struct ApiError(StatusCode, String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error":self.1}))).into_response()
    }
}
impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        eprintln!("Storage error: {e:#}");
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "保存失败，操作未确认，请重试并检查主机存储空间".into(),
        )
    }
}
impl From<rusqlite::Error> for ApiError {
    fn from(e: rusqlite::Error) -> Self {
        anyhow::Error::from(e).into()
    }
}
impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        anyhow::Error::from(e).into()
    }
}
fn bad(s: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, s.into())
}
fn forbidden() -> ApiError {
    ApiError(StatusCode::FORBIDDEN, "没有此操作或记录的访问权限".into())
}
fn missing() -> ApiError {
    ApiError(StatusCode::NOT_FOUND, "房间或记录不存在".into())
}
fn bearer(headers: &HeaderMap) -> &str {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|s| s.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .unwrap_or("")
}
fn auth(store: &Store, headers: &HeaderMap) -> ApiResult<PlayerSession> {
    let token = bearer(headers);
    if token == store.admin_token {
        return Ok(PlayerSession {
            player_id: "admin".into(),
            name: "主机管理员".into(),
            avatar: "茶".into(),
            token: token.into(),
        });
    }
    store
        .sessions
        .get(token)
        .cloned()
        .ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "请先输入昵称加入".into()))
}
fn is_owner(room: &Room, who: &PlayerSession) -> bool {
    room.owner_id == who.player_id || who.player_id == "admin"
}
fn check_name(name: &str) -> ApiResult<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 24 {
        Err(bad("名称需为 1–24 个字"))
    } else {
        Ok(name.to_owned())
    }
}
fn validate_config(c: &RoomConfig) -> ApiResult<()> {
    if !(1..=1_000_000).contains(&c.base_score)
        || !matches!(c.cap, None | Some(8 | 16 | 32 | 64))
        || !matches!(c.flower_penalty, 8 | 16 | 32)
        || !matches!(c.turn_seconds, 0 | 10..=180)
        || !matches!(c.response_seconds, 0 | 5..=60)
    {
        return Err(bad(
            "规则配置不合法：底分 1–1000000，封顶 8/16/32/64，花猪 8/16/32，计时允许关闭",
        ));
    }
    Ok(())
}

fn summary(r: &Room) -> Value {
    json!({"id":r.id,"name":r.name,"gameId":r.game_id,"ownerId":r.owner_id,"status":r.status(),"occupied":r.seats.iter().flatten().count(),"capacity":r.seats.len(),"round":r.round,"paused":r.paused,"config":r.config})
}
fn view(store: &Store, r: &Room, who: &PlayerSession) -> Value {
    let seat = r.seat_of(&who.player_id);
    let seats:Vec<Value>=r.seats.iter().map(|s|match s {None=>Value::Null,Some(s)=>json!({"playerId":s.player_id,"name":s.name,"avatar":s.avatar,"bot":s.bot,"ready":s.ready,"score":s.score,"connected":s.bot||store.connections.get(&(r.id.clone(),s.player_id.clone())).copied().unwrap_or(0)>0,"offlineSince":s.offline_since})}).collect();
    let same_roster = r
        .seats
        .iter()
        .zip(&r.round_players)
        .all(|(seat, id)| seat.as_ref().map(|s| &s.player_id) == id.as_ref());
    let game = seat
        .filter(|_| r.running() || same_roster)
        .and_then(|s| r.game.as_ref().map(|g| g.view(s)));
    let settlement = game.as_ref().and_then(|_| r.settlement_confirmed.as_ref().map(|acknowledged| {
        json!({"round":r.round,"acknowledged":acknowledged,"allAcknowledged":acknowledged.iter().all(|v| *v)})
    }));
    let mut standings = r.totals.values().collect::<Vec<_>>();
    standings.sort_by_key(|s| std::cmp::Reverse(s.score));
    json!({"id":r.id,"name":r.name,"gameId":r.game_id,"ownerId":r.owner_id,"version":r.version,"round":r.round,"paused":r.paused,"config":r.config,"seats":seats,"standings":standings,"selfSeat":seat,"game":game,"settlement":settlement,"status":r.status(),"deadline":r.deadline,"invite":if seat.is_some()||is_owner(r,who){Some(&r.invite)}else{None},"historyId":r.history_id})
}
pub(crate) async fn health() -> Json<Value> {
    Json(json!({"ok":true,"version":"0.1.0"}))
}
pub(crate) async fn host(State(s): State<AppState>) -> Json<Value> {
    let store = s.store.lock().unwrap();
    Json(
        json!({"port":s.port,"addresses":crate::addresses(s.port),"version":"0.1.0","rooms":store.rooms.values().filter(|r|!r.archived).count()}),
    )
}
pub(crate) async fn games() -> Json<Value> {
    Json(json!(registry::games()))
}
#[derive(Deserialize)]
pub(crate) struct NewSession {
    name: String,
    #[serde(default)]
    avatar: String,
}
pub(crate) async fn session(
    State(s): State<AppState>,
    Json(req): Json<NewSession>,
) -> ApiResult<Json<Value>> {
    let name = check_name(&req.name)?;
    let avatar = req.avatar.chars().take(2).collect::<String>();
    let avatar = if avatar.is_empty() {
        name.chars().take(1).collect()
    } else {
        avatar
    };
    let user = PlayerSession {
        player_id: id(),
        token: secret(),
        name,
        avatar,
    };
    let mut store = s.store.lock().unwrap();
    store.db.execute(
        "INSERT INTO sessions VALUES(?1,?2,?3,?4)",
        params![user.token, user.player_id, user.name, user.avatar],
    )?;
    let response =
        json!({"token":user.token,"playerId":user.player_id,"name":user.name,"avatar":user.avatar});
    store.sessions.insert(user.token.clone(), user);
    Ok(Json(response))
}
pub(crate) async fn rooms(State(s): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let store = s.store.lock().unwrap();
    auth(&store, &headers)?;
    let mut list = store
        .rooms
        .values()
        .filter(|r| !r.archived)
        .collect::<Vec<_>>();
    list.sort_by_key(|r| -r.created_at);
    Ok(Json(json!(list
        .into_iter()
        .map(summary)
        .collect::<Vec<_>>())))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NewRoom {
    name: String,
    #[serde(default)]
    config: RoomConfig,
    #[serde(default)]
    game_id: Option<String>,
}
pub(crate) async fn create_room(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<NewRoom>,
) -> ApiResult<Json<Value>> {
    let name = check_name(&req.name)?;
    validate_config(&req.config)?;
    let game_id = req.game_id.as_deref().unwrap_or(registry::MAHJONG_ID);
    let definition = registry::games()
        .into_iter()
        .find(|g| g.game_id == game_id)
        .ok_or_else(|| bad("此版本尚未安装该游戏"))?;
    let mut store = s.store.lock().unwrap();
    let who = auth(&store, &headers)?;
    if store.rooms.values().filter(|r| !r.archived).count() >= 32 {
        return Err(bad("主机最多同时保留 32 个开放房间，请先结束旧房间"));
    }
    let mut r = Room {
        id: id(),
        name,
        game_id: game_id.into(),
        owner_id: who.player_id.clone(),
        invite: secret(),
        version: 1,
        round: 0,
        config: req.config,
        seats: vec![None; definition.seats as usize],
        game: None,
        paused: false,
        archived: false,
        created_at: now(),
        round_started_at: None,
        deadline: None,
        next_bot_at: now(),
        history_id: None,
        accounted_ledger: 0,
        accounted_wins: 0,
        totals: Default::default(),
        round_players: Default::default(),
        settlement_confirmed: None,
        participants: HashSet::from([who.player_id.clone()]),
    };
    r.seats[0] = Some(Seat {
        player_id: who.player_id.clone(),
        name: who.name.clone(),
        avatar: who.avatar.clone(),
        bot: false,
        ready: true,
        score: 0,
        offline_since: Some(now()),
    });
    let room_id = r.id.clone();
    storage::save(&mut store, r, None, "创建房间", false)?;
    Ok(Json(view(&store, &store.rooms[&room_id], &who)))
}
pub(crate) async fn room(
    State(s): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let store = s.store.lock().unwrap();
    let who = auth(&store, &headers)?;
    let r = store.rooms.get(&id).ok_or_else(missing)?;
    Ok(Json(view(&store, r, &who)))
}
#[derive(Deserialize)]
pub(crate) struct Join {
    seat: Option<usize>,
    invite: Option<String>,
}
pub(crate) async fn join(
    State(s): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<Join>,
) -> ApiResult<Json<Value>> {
    let mut store = s.store.lock().unwrap();
    let who = auth(&store, &headers)?;
    let mut r = store.rooms.get(&id).cloned().ok_or_else(missing)?;
    if r.seat_of(&who.player_id).is_some() {
        return Ok(Json(view(&store, &r, &who)));
    }
    if r.archived || r.running() || r.awaiting_settlement() {
        return Err(bad("本局进行中，请等待下一局或请房主恢复你的原座位"));
    }
    // Rooms are public to this LAN. An explicitly supplied stale invite is rejected.
    if req.invite.as_ref().is_some_and(|i| i != &r.invite) {
        return Err(forbidden());
    }
    let seat = req
        .seat
        .or_else(|| r.seats.iter().position(Option::is_none))
        .ok_or_else(|| bad("房间已满"))?;
    if seat >= r.seats.len() || r.seats[seat].is_some() {
        return Err(bad("座位不可用"));
    }
    r.seats[seat] = Some(Seat {
        player_id: who.player_id.clone(),
        name: who.name.clone(),
        avatar: who.avatar.clone(),
        bot: false,
        ready: false,
        score: r.totals.get(&who.player_id).map(|s| s.score).unwrap_or(0),
        offline_since: Some(now()),
    });
    if r.game.is_none() {
        r.participants.insert(who.player_id.clone());
    }
    r.version += 1;
    storage::save(&mut store, r, None, "加入房间", false)?;
    let result = view(&store, &store.rooms[&id], &who);
    let _ = s.updates.send(id);
    Ok(Json(result))
}

#[derive(Deserialize)]
pub(crate) struct Command {
    id: String,
    version: u64,
    action: RoomAction,
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum RoomAction {
    ConfirmSettlement {
        round: u32,
    },
    Ready {
        #[serde(default = "yes")]
        ready: bool,
    },
    Start,
    AddBot {
        seat: Option<usize>,
    },
    RemoveBot {
        seat: usize,
    },
    Configure {
        config: RoomConfig,
    },
    Pause,
    Resume,
    Abort,
    Leave,
    Rebind {
        seat: usize,
        #[serde(rename = "playerId")]
        player_id: String,
    },
    Game {
        action: Value,
    },
}
fn yes() -> bool {
    true
}
fn set_deadline(r: &mut Room) {
    r.next_bot_at = now() + 700;
    r.deadline = if r.paused || !r.running() {
        None
    } else {
        let g = r.game.as_ref().unwrap();
        let seconds = g.seconds();
        (seconds > 0).then(|| now() + i64::from(seconds) * 1000)
    };
}
fn update_scores(r: &mut Room) -> ApiResult<()> {
    for seat in r.seats.iter().flatten() {
        r.totals
            .entry(seat.player_id.clone())
            .or_insert_with(|| Standing {
                player_id: seat.player_id.clone(),
                name: seat.name.clone(),
                score: seat.score,
                ..Default::default()
            });
    }
    if let Some(g) = &r.game {
        for e in g.ledger().iter().skip(r.accounted_ledger) {
            // Ledger fields are stable engine API: payer, payee, amount.
            if let Some(seat) = &mut r.seats[e.from] {
                seat.score = seat
                    .score
                    .checked_sub(e.amount)
                    .ok_or_else(|| bad("累计分数超过支持范围"))?;
                r.totals.get_mut(&seat.player_id).unwrap().score = seat.score;
            }
            if let Some(seat) = &mut r.seats[e.to] {
                seat.score = seat
                    .score
                    .checked_add(e.amount)
                    .ok_or_else(|| bad("累计分数超过支持范围"))?;
                r.totals.get_mut(&seat.player_id).unwrap().score = seat.score;
            }
        }
        r.accounted_ledger = g.ledger().len();
        for &winner in g.winners().iter().skip(r.accounted_wins) {
            if let Some(seat) = &r.seats[winner] {
                let standing = r.totals.get_mut(&seat.player_id).unwrap();
                standing.win_count += 1;
                if g.self_draw(winner) {
                    standing.self_draw_count += 1;
                }
            }
        }
        let previous = g
            .discard_wins(0, r.accounted_wins)
            .into_iter()
            .collect::<HashSet<_>>();
        let new_discards = g
            .discard_wins(r.accounted_wins, usize::MAX)
            .into_iter()
            .filter(|key| !previous.contains(key))
            .collect::<HashSet<_>>();
        for (from, _) in new_discards {
            if let Some(seat) = &r.seats[from] {
                r.totals.get_mut(&seat.player_id).unwrap().discard_win_count += 1;
            }
        }
        r.accounted_wins = g.winners().len();
    }
    r.initialize_settlement();
    Ok(())
}
pub(crate) async fn command(
    State(s): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<Command>,
) -> ApiResult<Json<Value>> {
    if req.id.is_empty() || req.id.len() > 128 {
        return Err(bad("命令 ID 不合法"));
    }
    let mut store = s.store.lock().unwrap();
    let who = auth(&store, &headers)?;
    let mut r = store.rooms.get(&id).cloned().ok_or_else(missing)?;
    let used = store
        .db
        .query_row(
            "SELECT room_id FROM commands WHERE player_id=?1 AND id=?2",
            params![who.player_id, req.id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    if let Some(previous) = used {
        if previous != id {
            return Err(bad("命令 ID 已用于其他房间"));
        }
        return Ok(Json(view(&store, &r, &who)));
    }
    // Independent acknowledgements of the same completed hand commute.
    let concurrent_confirmation = matches!(&req.action, RoomAction::ConfirmSettlement { round }
        if *round == r.round && r.settlement_confirmed.is_some() && req.version < r.version);
    if r.version != req.version && !concurrent_confirmation {
        return Err(ApiError(
            StatusCode::CONFLICT,
            "牌局已更新，请按最新状态操作".into(),
        ));
    }
    if r.archived && !matches!(&req.action, RoomAction::ConfirmSettlement { .. }) {
        return Err(bad("房间已结束"));
    }
    let seat = r.seat_of(&who.player_id);
    let owner = is_owner(&r, &who);
    let mut record = false;
    let label;
    match req.action {
        RoomAction::ConfirmSettlement { round } => {
            r.confirm_settlement(&who.player_id, round).map_err(bad)?;
            label = "确认本局分数";
        }
        RoomAction::Ready { ready } => {
            let i = seat.ok_or_else(forbidden)?;
            if r.running() {
                return Err(bad("本局已开始"));
            }
            if r.awaiting_settlement() {
                return Err(bad("请先确认本局分数，等待四人全部确认"));
            }
            r.seats[i].as_mut().unwrap().ready = ready;
            label = "准备";
        }
        RoomAction::AddBot { seat } => {
            if !owner {
                return Err(forbidden());
            }
            if r.running() || r.awaiting_settlement() {
                return Err(bad("开局后不能改变座位"));
            }
            let i = seat
                .or_else(|| r.seats.iter().position(Option::is_none))
                .ok_or_else(|| bad("座位已满"))?;
            if i >= r.seats.len() || r.seats[i].is_some() {
                return Err(bad("座位不可用"));
            }
            r.seats[i] = Some(Seat {
                player_id: format!("bot-{}", crate::model::id()),
                name: format!("茶友{}", i + 1),
                avatar: "竹".into(),
                bot: true,
                ready: true,
                score: 0,
                offline_since: None,
            });
            label = "电脑入座";
        }
        RoomAction::RemoveBot { seat } => {
            if !owner {
                return Err(forbidden());
            }
            if r.running()
                || r.awaiting_settlement()
                || seat >= r.seats.len()
                || !r.seats[seat].as_ref().is_some_and(|x| x.bot)
            {
                return Err(bad("只能在局间移除电脑"));
            }
            r.seats[seat] = None;
            label = "移除电脑";
        }
        RoomAction::Configure { config } => {
            if !owner {
                return Err(forbidden());
            }
            validate_config(&config)?;
            r.config = config;
            label = "设置下局规则";
        }
        RoomAction::Start => {
            if !owner {
                return Err(forbidden());
            }
            if r.running() {
                return Err(bad("牌局已开始"));
            }
            if r.awaiting_settlement() {
                return Err(bad("请等待四位玩家全部确认本局分数，再开始下一局"));
            }
            if !r.seats.iter().all(|s| s.as_ref().is_some_and(|x| x.ready)) {
                return Err(bad(format!("需要 {} 个座位都准备好", r.seats.len())));
            }
            let game = RoomGame::new(
                &r.game_id,
                r.config.clone(),
                rand::random(),
                r.game.as_ref().filter(|_| {
                    r.game_id != guandan::GAME_ID
                        || r.seats
                            .iter()
                            .map(|s| s.as_ref().map(|s| &s.player_id))
                            .eq(r.round_players.iter().map(|s| s.as_ref()))
                }),
            )
            .map_err(bad)?;
            r.game = Some(game);
            r.round += 1;
            r.round_started_at = Some(now());
            r.history_id = Some(crate::model::id());
            r.accounted_ledger = 0;
            r.accounted_wins = 0;
            r.settlement_confirmed = None;
            r.participants = r
                .seats
                .iter()
                .flatten()
                .filter(|s| !s.bot)
                .map(|s| s.player_id.clone())
                .collect();
            r.round_players = r
                .seats
                .iter()
                .map(|s| s.as_ref().map(|s| s.player_id.clone()))
                .collect();
            r.paused = false;
            set_deadline(&mut r);
            record = true;
            label = "开局";
        }
        RoomAction::Pause => {
            if !owner {
                return Err(forbidden());
            }
            r.paused = true;
            r.deadline = None;
            label = "暂停";
        }
        RoomAction::Resume => {
            if !owner {
                return Err(forbidden());
            }
            r.paused = false;
            set_deadline(&mut r);
            label = "继续";
        }
        RoomAction::Abort => {
            if !owner {
                return Err(forbidden());
            }
            if let Some(g) = &mut r.game {
                g.abort();
            }
            r.archived = true;
            r.paused = false;
            r.deadline = None;
            record = r.game.is_some();
            label = "提前结束";
        }
        RoomAction::Leave => {
            let i = seat.ok_or_else(forbidden)?;
            if r.running() {
                return Err(bad("请在本局结束后离座，断线时电脑会临时接管"));
            }
            if r.awaiting_settlement() {
                return Err(bad("请等待四位玩家全部确认本局分数后再离座"));
            }
            r.seats[i] = None;
            if r.owner_id == who.player_id {
                if let Some(next) = r.seats.iter().flatten().find(|s| !s.bot) {
                    r.owner_id = next.player_id.clone();
                } else {
                    r.archived = true;
                }
            }
            label = "离座";
        }
        RoomAction::Rebind { seat, player_id } => {
            if !owner {
                return Err(forbidden());
            }
            if seat >= r.seats.len() || r.seats[seat].is_none() || r.seat_of(&player_id).is_some() {
                return Err(bad("座位或新身份不合法"));
            }
            if r.totals.contains_key(&player_id) {
                return Err(bad(
                    "此身份在本桌已有分数记录，请用原身份重新入座，或使用全新的恢复身份",
                ));
            }
            let user = store
                .sessions
                .values()
                .find(|p| p.player_id == player_id)
                .ok_or_else(|| bad("新身份尚未连接到此主机"))?;
            let target = r.seats[seat].as_mut().unwrap();
            if target.bot {
                return Err(bad("电脑座位不能作为身份恢复目标"));
            }
            if target.player_id == r.owner_id {
                r.owner_id = player_id.clone();
            }
            if let Some(mut standing) = r.totals.remove(&target.player_id) {
                standing.player_id = player_id.clone();
                standing.name = user.name.clone();
                r.totals.insert(player_id.clone(), standing);
            }
            target.player_id = player_id.clone();
            target.name = user.name.clone();
            target.avatar = user.avatar.clone();
            target.offline_since = Some(now());
            r.round_players[seat] = Some(player_id.clone());
            if let Some(confirmed) = &mut r.settlement_confirmed {
                confirmed[seat] = false;
                target.ready = false;
            }
            r.participants.insert(player_id);
            label = "恢复座位";
        }
        RoomAction::Game { action } => {
            let i = seat.ok_or_else(forbidden)?;
            if r.paused {
                return Err(bad("房间已暂停，请房主继续"));
            }
            let g = r.game.as_mut().ok_or_else(|| bad("尚未开局"))?;
            if g.apply(i, action).map_err(bad)? {
                set_deadline(&mut r);
            }
            record = true;
            label = "玩家操作";
        }
    }
    update_scores(&mut r)?;
    if !r.running() {
        r.deadline = None;
    }
    r.version += 1;
    storage::save(
        &mut store,
        r,
        Some((&who.player_id, &req.id)),
        label,
        record,
    )?;
    let result = view(&store, &store.rooms[&id], &who);
    let _ = s.updates.send(id);
    Ok(Json(result))
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistoryFilter {
    room_id: Option<String>,
    player: Option<String>,
    from: Option<i64>,
    to: Option<i64>,
    seat: Option<usize>,
    confirm: Option<bool>,
}
fn history_allowed(r: &Room, who: &PlayerSession) -> bool {
    who.player_id == "admin"
        || r.owner_id == who.player_id
        || r.participants.contains(&who.player_id)
}
fn read_history(store: &Store, id: &str, who: &PlayerSession) -> ApiResult<Room> {
    let state: Option<String> = store
        .db
        .query_row(
            "SELECT state FROM history WHERE id=?1 AND finished_at IS NOT NULL",
            [id],
            |r| r.get(0),
        )
        .optional()?;
    let r: Room = serde_json::from_str(&state.ok_or_else(missing)?)?;
    if !history_allowed(&r, who) {
        return Err(forbidden());
    }
    Ok(r)
}
fn history_summary(r: &Room) -> Value {
    let g = r.game.as_ref().unwrap();
    let players: Vec<Value> = r
        .seats
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut p = g.player_summary(i);
            p["playerId"] = json!(s.as_ref().map(|s| &s.player_id));
            p["name"] = json!(s.as_ref().map(|s| s.name.as_str()).unwrap_or("空座"));
            p
        })
        .collect();
    json!({"id":r.history_id,"roomId":r.id,"roomName":r.name,"gameId":r.game_id,"round":r.round,"startedAt":r.round_started_at,"status":g.phase(),"players":players,"config":g.config()})
}
pub(crate) async fn history(
    State(s): State<AppState>,
    headers: HeaderMap,
    Query(filter): Query<HistoryFilter>,
) -> ApiResult<Json<Value>> {
    let store = s.store.lock().unwrap();
    let who = auth(&store, &headers)?;
    let mut stmt=store.db.prepare("SELECT state,finished_at FROM history WHERE finished_at IS NOT NULL ORDER BY started_at DESC")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    let mut result = vec![];
    for row in rows {
        let (state, finished) = row?;
        let r: Room = serde_json::from_str(&state)?;
        if !history_allowed(&r, &who)
            || filter.room_id.as_ref().is_some_and(|id| id != &r.id)
            || filter
                .from
                .is_some_and(|t| r.round_started_at.unwrap_or(0) < t)
            || filter
                .to
                .is_some_and(|t| r.round_started_at.unwrap_or(0) > t)
            || filter.player.as_ref().is_some_and(|name| {
                !r.seats
                    .iter()
                    .flatten()
                    .any(|p| p.name.contains(name) || &p.player_id == name)
            })
        {
            continue;
        }
        let mut item = history_summary(&r);
        item["finishedAt"] = json!(finished);
        result.push(item);
    }
    Ok(Json(json!(result)))
}
pub(crate) async fn replay(
    State(s): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Query(filter): Query<HistoryFilter>,
) -> ApiResult<Json<Value>> {
    let store = s.store.lock().unwrap();
    let who = auth(&store, &headers)?;
    let r = read_history(&store, &id, &who)?;
    let perspective = filter.seat.unwrap_or(0);
    if perspective >= r.seats.len() {
        return Err(bad("视角无效"));
    }
    let mut stmt = store.db.prepare(
        "SELECT frame_index,at,label,game FROM frames WHERE history_id=?1 ORDER BY frame_index",
    )?;
    let rows = stmt.query_map([&id], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    let mut frames = vec![];
    for row in rows {
        let (index, at, label, text) = row?;
        let game: RoomGame = serde_json::from_str(&text)?;
        let (view, hands) = game.replay(perspective);
        frames.push(json!({"index":index,"at":at,"label":label,"game":view,"hands":hands}));
    }
    Ok(Json(
        json!({"gameId":r.game_id,"rulesVersion":r.game.as_ref().unwrap().rules_version(),"roomName":r.name,"round":r.round,"players":r.seats.iter().map(|s|s.as_ref().map(|s|s.name.clone()).unwrap_or_default()).collect::<Vec<_>>(),"config":r.game.as_ref().unwrap().config(),"frames":frames}),
    ))
}
fn csv_cell(s: &str) -> String {
    // Prevent spreadsheet formula execution when a player chooses a formula as their nickname.
    let safe = if s.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{s}")
    } else {
        s.to_owned()
    };
    format!("\"{}\"", safe.replace('"', "\"\""))
}
pub(crate) async fn export_csv(
    State(s): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let store = s.store.lock().unwrap();
    let who = auth(&store, &headers)?;
    let r = read_history(&store, &id, &who)?;
    let mut csv = String::from("\u{feff}局数,付款人,收款人,原因,倍率,分数,事件编号\r\n");
    let name = |i: usize| {
        r.seats[i]
            .as_ref()
            .map(|s| s.name.as_str())
            .unwrap_or("空座")
    };
    for e in &r.game.as_ref().unwrap().ledger() {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{}\r\n",
            r.round,
            csv_cell(name(e.from)),
            csv_cell(name(e.to)),
            csv_cell(match e.reason.as_str() {
                "self_draw" => "自摸",
                "discard_win" => "点炮胡",
                "rob_kong" => "抢杠胡",
                "kong_discard" => "杠上炮",
                "concealed_kong" => "暗杠",
                "exposed_kong" => "直杠",
                "supplemental_kong" => "补杠",
                "flower_penalty" => "查花猪",
                "not_ready_penalty" => "查大叫",
                "kong_refund" => "退税",
                "landlord_win" => "地主获胜",
                "farmers_win" => "农民获胜",
                other => other,
            }),
            e.multiplier,
            e.amount,
            e.event_id
        ));
    }
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=game-ledger.csv",
            ),
        ],
        csv,
    )
        .into_response())
}
pub(crate) async fn delete_history(
    State(s): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Query(filter): Query<HistoryFilter>,
) -> ApiResult<Json<Value>> {
    if filter.confirm != Some(true) {
        return Err(bad("删除历史需要明确确认"));
    }
    let mut store = s.store.lock().unwrap();
    let who = auth(&store, &headers)?;
    let r = read_history(&store, &id, &who)?;
    if !is_owner(&r, &who) {
        return Err(forbidden());
    }
    let mut current = store.rooms.get(&r.id).cloned();
    if let Some(room) = &mut current {
        if room.history_id.as_deref() == Some(&id) {
            room.history_id = None;
        }
    }
    // Detach and delete atomically; a storage failure cannot resurrect the history later.
    let tx = store.db.transaction()?;
    if let Some(room) = &current {
        tx.execute(
            "UPDATE rooms SET state=?1 WHERE id=?2",
            params![serde_json::to_string(room)?, room.id],
        )?;
    }
    tx.execute("DELETE FROM frames WHERE history_id=?1", [&id])?;
    tx.execute("DELETE FROM ledger WHERE history_id=?1", [&id])?;
    tx.execute("DELETE FROM history WHERE id=?1", [&id])?;
    tx.commit()?;
    if let Some(room) = current {
        store.rooms.insert(room.id.clone(), room);
    }
    Ok(Json(json!({"ok":true})))
}

pub(crate) async fn ws(
    State(s): State<AppState>,
    upgrade: WebSocketUpgrade,
    headers: HeaderMap,
) -> ApiResult<Response> {
    if let Some(origin) = headers.get(header::ORIGIN).and_then(|h| h.to_str().ok()) {
        let native = [
            "tauri://localhost",
            "http://tauri.localhost",
            "https://tauri.localhost",
            "http://localhost:1420",
            "http://localhost:5173",
            "http://localhost:5174",
        ];
        let host = headers
            .get(header::HOST)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        if origin != format!("http://{host}") && !native.contains(&origin) {
            return Err(forbidden());
        }
    }
    Ok(upgrade
        .max_message_size(32 * 1024)
        .on_upgrade(move |socket| socket_loop(socket, s))
        .into_response())
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WsAuth {
    r#type: String,
    token: String,
    room_id: String,
}
async fn socket_loop(mut socket: WebSocket, s: AppState) {
    let mut stopped = s.stopped.clone();
    if *stopped.borrow() {
        return;
    }
    let message = tokio::time::timeout(std::time::Duration::from_secs(5), socket.recv()).await;
    let Ok(Some(Ok(Message::Text(text)))) = message else {
        return;
    };
    let Ok(req) = serde_json::from_str::<WsAuth>(&text) else {
        return;
    };
    if req.r#type != "auth" {
        return;
    }
    let who = {
        let store = s.store.lock().unwrap();
        store.sessions.get(&req.token).cloned().or_else(|| {
            (req.token == store.admin_token).then(|| PlayerSession {
                player_id: "admin".into(),
                name: "主机管理员".into(),
                avatar: "茶".into(),
                token: req.token.clone(),
            })
        })
    };
    let Some(who) = who else {
        let _ = socket
            .send(Message::Text(
                json!({"type":"error","message":"身份已失效，请重新加入"})
                    .to_string()
                    .into(),
            ))
            .await;
        return;
    };
    let mut updates = s.updates.subscribe();
    {
        let mut store = s.store.lock().unwrap();
        if !store.rooms.contains_key(&req.room_id) {
            return;
        }
        *store
            .connections
            .entry((req.room_id.clone(), who.player_id.clone()))
            .or_default() += 1;
        if let Some(r) = store.rooms.get_mut(&req.room_id) {
            for seat in r.seats.iter_mut().flatten() {
                if seat.player_id == who.player_id {
                    seat.offline_since = None;
                }
            }
        }
    }
    let _ = s.updates.send(req.room_id.clone());
    let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(10));
    let mut last_seen = now();
    let mut should_send = true;
    loop {
        if should_send {
            let payload = {
                let store = s.store.lock().unwrap();
                store
                    .rooms
                    .get(&req.room_id)
                    .map(|r| json!({"type":"room","room":view(&store,r,&who)}).to_string())
            };
            let Some(payload) = payload else {
                break;
            };
            if socket.send(Message::Text(payload.into())).await.is_err() {
                break;
            }
            should_send = false;
        }
        tokio::select! {
            _=stopped.changed()=>{let _=socket.send(Message::Close(None)).await;break;},
            message=socket.recv()=>match message {
                Some(Ok(Message::Close(_)))|None|Some(Err(_))=>break,
                Some(Ok(_))=>{last_seen=now();},
            },
            update=updates.recv()=>match update {
                Ok(room) if room==req.room_id=>should_send=true,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_))=>should_send=true,
                Err(_)=>break,
                _=>{},
            },
            _=heartbeat.tick()=>{if now()-last_seen>35_000{break;}if socket.send(Message::Ping(vec![].into())).await.is_err(){break;}},
        }
    }
    {
        let mut store = s.store.lock().unwrap();
        let count = store
            .connections
            .entry((req.room_id.clone(), who.player_id.clone()))
            .or_default();
        *count = count.saturating_sub(1);
        if *count == 0 {
            if let Some(r) = store.rooms.get_mut(&req.room_id) {
                for seat in r.seats.iter_mut().flatten() {
                    if seat.player_id == who.player_id {
                        seat.offline_since = Some(now());
                    }
                }
            }
        }
    }
    let _ = s.updates.send(req.room_id);
}

pub(crate) async fn tick_loop(s: AppState) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));
    loop {
        interval.tick().await;
        let mut store = s.store.lock().unwrap();
        let rooms = store.rooms.keys().cloned().collect::<Vec<_>>();
        for id in rooms {
            let mut r = store.rooms[&id].clone();
            if !r.running() || r.paused || r.archived {
                continue;
            }
            let t = now();
            let g = r.game.as_ref().unwrap();
            let action = g.acting_seats().into_iter().find_map(|i| {
                let seat = r.seats[i].as_ref()?;
                let connected = store
                    .connections
                    .get(&(r.id.clone(), seat.player_id.clone()))
                    .copied()
                    .unwrap_or(0)
                    > 0;
                let offline_since = seat.offline_since.unwrap_or(t);
                if !seat.bot && !connected && t - offline_since < 60_000 {
                    return None;
                }
                let is_bot = seat.bot || !connected;
                if is_bot && t >= r.next_bot_at {
                    return g.bot_action(i, false).map(|a| (i, a));
                }
                if !is_bot && r.deadline.is_some_and(|deadline| t >= deadline) {
                    let a = g.bot_action(i, true);
                    return a.map(|a| (i, a));
                }
                None
            });
            let Some((seat, action)) = action else {
                continue;
            };
            let g = r.game.as_mut().unwrap();
            match g.apply(seat, action) {
                Ok(true) => set_deadline(&mut r),
                Ok(false) => r.next_bot_at = t + 700,
                Err(error) => {
                    eprintln!("Automatic action rejected: {error}");
                    continue;
                }
            }
            if let Err(error) = update_scores(&mut r) {
                eprintln!("Score accounting failed: {}", error.1);
                continue;
            }
            if !r.running() {
                r.deadline = None;
            }
            r.version += 1;
            match storage::save(&mut store, r, None, "自动操作", true) {
                Ok(()) => {
                    let _ = s.updates.send(id);
                }
                Err(error) => {
                    eprintln!("Auto-save failed; action rolled back: {error}");
                }
            }
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/assets.rs"));
pub(crate) async fn assets(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if path.starts_with("api/") || path.starts_with("ws/") {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    }
    let name = if path.is_empty() { "index.html" } else { path };
    let asset = ASSETS.iter().find(|(key, _)| *key == name).or_else(|| {
        (!name.contains('.'))
            .then(|| ASSETS.iter().find(|(key, _)| *key == "index.html"))
            .flatten()
    });
    if let Some((name, bytes)) = asset {
        let mime = mime_guess::from_path(name)
            .first_or_octet_stream()
            .to_string();
        return (
            [
                (header::CONTENT_TYPE, mime),
                (
                    header::CACHE_CONTROL,
                    if name.starts_with("assets/") {
                        "public, max-age=31536000, immutable".into()
                    } else {
                        // Public audio/icons keep stable names across releases.
                        // Only Vite's fingerprinted assets can be cached forever.
                        "no-cache".into()
                    },
                ),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff".into()),
            ],
            *bytes,
        )
            .into_response();
    }
    (
        StatusCode::NOT_FOUND,
        "网页资源尚未构建。请运行 npm --prefix web run build 后重新编译服务器。",
    )
        .into_response()
}
