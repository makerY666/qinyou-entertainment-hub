mod game;
mod model;
pub mod registry;
mod routes;
mod settlement;
mod storage;

use axum::{
    routing::{delete, get, post},
    Router,
};
use model::*;
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::{broadcast, oneshot, watch};
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    store: Arc<Mutex<Store>>,
    updates: broadcast::Sender<String>,
    port: u16,
    stopped: watch::Receiver<bool>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostInfo {
    pub port: u16,
    pub addresses: Vec<String>,
    pub admin_token: String,
}
pub struct HostHandle {
    port: u16,
    admin_token: String,
    stop: Option<oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<std::io::Result<()>>,
    tick: tokio::task::JoinHandle<()>,
    stopped: watch::Sender<bool>,
    _data_lock: std::fs::File,
}
impl HostHandle {
    pub fn info(&self) -> HostInfo {
        HostInfo {
            port: self.port,
            addresses: addresses(self.port),
            admin_token: self.admin_token.clone(),
        }
    }
    pub async fn shutdown(mut self) -> anyhow::Result<()> {
        let _ = self.stopped.send(true);
        self.tick.abort();
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        // Active websockets must not keep a native foreground service alive forever.
        match tokio::time::timeout(std::time::Duration::from_secs(3), &mut self.task).await {
            Ok(result) => {
                result??;
            }
            Err(_) => {
                self.task.abort();
            }
        }
        Ok(())
    }
}
impl Drop for HostHandle {
    fn drop(&mut self) {
        let _ = self.stopped.send(true);
        self.tick.abort();
        self.task.abort();
    }
}
pub fn addresses(port: u16) -> Vec<String> {
    let mut result: Vec<String> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| !i.is_loopback())
        .filter_map(|i| match i.ip() {
            std::net::IpAddr::V4(ip) if !ip.is_link_local() => Some(format!("http://{ip}:{port}")),
            _ => None,
        })
        .collect();
    result.sort();
    result.dedup();
    if result.is_empty() {
        result.push(format!("http://127.0.0.1:{port}"));
    }
    result
}
pub async fn spawn_host(data_dir: PathBuf, port: u16) -> anyhow::Result<HostHandle> {
    if port != 0
        && tokio::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port))
            .await
            .is_ok()
    {
        anyhow::bail!("端口 {port} 已被本机服务占用，请换一个端口。");
    }
    std::fs::create_dir_all(&data_dir)?;
    let data_lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(data_dir.join("host.lock"))?;
    data_lock
        .try_lock()
        .map_err(|e| anyhow::anyhow!("此存档目录已由另一个主机实例使用：{e}"))?;
    let store = storage::open(&data_dir)?;
    let admin_token = store.admin_token.clone();
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::UNSPECIFIED, port))
        .await
        .map_err(|e| anyhow::anyhow!("无法监听端口 {port}：{e}。请关闭重复主机或换一个端口。"))?;
    let port = listener.local_addr()?.port();
    let (updates, _) = broadcast::channel(256);
    let (stopped, stopped_rx) = watch::channel(false);
    let state = AppState {
        store: Arc::new(Mutex::new(store)),
        updates,
        port,
        stopped: stopped_rx,
    };
    // Only bundled Tauri origins and local Vite preview need CORS. LAN browsers use same-origin.
    let cors = CorsLayer::new()
        .allow_origin(
            [
                "tauri://localhost",
                "http://tauri.localhost",
                "https://tauri.localhost",
                "http://localhost:1420",
                "http://localhost:5173",
                "http://localhost:5174",
            ]
            .map(|s| s.parse::<axum::http::HeaderValue>().unwrap()),
        )
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::DELETE,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ]);
    let app = Router::new()
        .route("/api/v1/health", get(routes::health))
        .route("/api/v1/host", get(routes::host))
        .route("/api/v1/games", get(routes::games))
        .route("/api/v1/session", post(routes::session))
        .route(
            "/api/v1/rooms",
            get(routes::rooms).post(routes::create_room),
        )
        .route("/api/v1/rooms/{room}", get(routes::room))
        .route("/api/v1/rooms/{room}/join", post(routes::join))
        .route("/api/v1/rooms/{room}/command", post(routes::command))
        .route("/api/v1/history", get(routes::history))
        .route("/api/v1/history/{id}", delete(routes::delete_history))
        .route("/api/v1/history/{id}/replay", get(routes::replay))
        .route("/api/v1/history/{id}/export.csv", get(routes::export_csv))
        .route("/ws/v1", get(routes::ws))
        .fallback(routes::assets)
        .layer(axum::extract::DefaultBodyLimit::max(32 * 1024))
        .layer(cors)
        .with_state(state.clone());
    let tick = tokio::spawn(routes::tick_loop(state));
    let (stop, rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = rx.await;
            })
            .await
    });
    Ok(HostHandle {
        port,
        admin_token,
        stop: Some(stop),
        task,
        tick,
        stopped,
        _data_lock: data_lock,
    })
}
