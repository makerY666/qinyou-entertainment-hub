use crate::model::*;
use rusqlite::{params, Connection};
use std::{collections::HashMap, path::Path};

pub fn open(dir: &Path) -> anyhow::Result<Store> {
    std::fs::create_dir_all(dir)?;
    let db = Connection::open(dir.join("hub.sqlite3"))?;
    db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;
        CREATE TABLE IF NOT EXISTS metadata(key TEXT PRIMARY KEY, value TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS sessions(token TEXT PRIMARY KEY, player_id TEXT NOT NULL, name TEXT NOT NULL, avatar TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS rooms(id TEXT PRIMARY KEY, state TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS commands(player_id TEXT NOT NULL, id TEXT NOT NULL, room_id TEXT NOT NULL, PRIMARY KEY(player_id,id));
        CREATE TABLE IF NOT EXISTS history(id TEXT PRIMARY KEY, room_id TEXT NOT NULL, started_at INTEGER NOT NULL, finished_at INTEGER, round INTEGER NOT NULL, state TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS frames(history_id TEXT NOT NULL, frame_index INTEGER NOT NULL, at INTEGER NOT NULL, label TEXT NOT NULL, game TEXT NOT NULL, PRIMARY KEY(history_id,frame_index));
        CREATE TABLE IF NOT EXISTS ledger(history_id TEXT NOT NULL, entry_index INTEGER NOT NULL, entry TEXT NOT NULL, PRIMARY KEY(history_id,entry_index));
        CREATE INDEX IF NOT EXISTS history_time ON history(started_at);")?;
    let admin_token = db
        .query_row(
            "SELECT value FROM metadata WHERE key='admin_token'",
            [],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_else(|_| secret());
    db.execute(
        "INSERT OR IGNORE INTO metadata VALUES('admin_token',?1)",
        [&admin_token],
    )?;
    db.execute(
        "INSERT OR IGNORE INTO metadata VALUES('schema_version','1')",
        [],
    )?;
    let rooms = {
        let mut stmt = db.prepare("SELECT state FROM rooms")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let mut rooms = HashMap::new();
        for row in rows {
            let mut room: Room = serde_json::from_str(&row?)?;
            room.initialize_settlement();
            if room.running() {
                room.paused = true;
                room.deadline = None;
            }
            for seat in room.seats.iter_mut().flatten() {
                if !seat.bot {
                    seat.offline_since = Some(now());
                }
            }
            rooms.insert(room.id.clone(), room);
        }
        rooms
    };
    let sessions = {
        let mut stmt = db.prepare("SELECT token,player_id,name,avatar FROM sessions")?;
        let rows = stmt.query_map([], |r| {
            Ok(PlayerSession {
                token: r.get(0)?,
                player_id: r.get(1)?,
                name: r.get(2)?,
                avatar: r.get(3)?,
            })
        })?;
        let mut sessions = HashMap::new();
        for s in rows {
            let s = s?;
            sessions.insert(s.token.clone(), s);
        }
        sessions
    };
    Ok(Store {
        db,
        rooms,
        sessions,
        connections: HashMap::new(),
        admin_token,
    })
}

// Persist the state, deduplication key, replay frame, and ledger in one commit.
// Callers publish the new state only after this transaction succeeds.
pub fn save(
    store: &mut Store,
    room: Room,
    command: Option<(&str, &str)>,
    label: &str,
    record_frame: bool,
) -> anyhow::Result<()> {
    let state = serde_json::to_string(&room)?;
    let tx = store.db.transaction()?;
    tx.execute("INSERT INTO rooms(id,state) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET state=excluded.state", params![room.id,state])?;
    if let Some((player, command)) = command {
        tx.execute(
            "INSERT INTO commands(player_id,id,room_id) VALUES(?1,?2,?3)",
            params![player, command, room.id],
        )?;
    }
    if let (true, Some(history), Some(game)) = (record_frame, &room.history_id, &room.game) {
        let finished = if room.running() { None } else { Some(now()) };
        tx.execute("INSERT INTO history(id,room_id,started_at,finished_at,round,state) VALUES(?1,?2,?3,?4,?5,?6)
            ON CONFLICT(id) DO UPDATE SET finished_at=excluded.finished_at,state=excluded.state WHERE history.finished_at IS NULL", params![history,room.id,room.round_started_at,finished,room.round,state])?;
        let already_finished: bool = tx.query_row(
            "SELECT finished_at IS NOT NULL FROM history WHERE id=?1",
            [history],
            |r| r.get(0),
        )?;
        // The current final frame is still needed, but a later abort/configuration
        // must never append to an already-completed replay.
        let prior_frame_version:Option<u64>=tx.query_row("SELECT json_extract(game,'$.version') FROM frames WHERE history_id=?1 ORDER BY frame_index DESC LIMIT 1",[history],|r|r.get(0)).ok();
        if record_frame && (!already_finished || prior_frame_version != Some(game.version())) {
            let index: i64 = tx.query_row(
                "SELECT COALESCE(MAX(frame_index)+1,0) FROM frames WHERE history_id=?1",
                [history],
                |r| r.get(0),
            )?;
            tx.execute(
                "INSERT INTO frames VALUES(?1,?2,?3,?4,?5)",
                params![history, index, now(), label, serde_json::to_string(game)?],
            )?;
        }
        for (i, entry) in game.ledger().iter().enumerate() {
            tx.execute(
                "INSERT OR IGNORE INTO ledger VALUES(?1,?2,?3)",
                params![history, i, serde_json::to_string(entry)?],
            )?;
        }
    }
    tx.commit()?;
    store.rooms.insert(room.id.clone(), room);
    Ok(())
}
