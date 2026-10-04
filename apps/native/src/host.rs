//! A dedicated runtime owns the server. No WebView lifecycle drives hosting.
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{mpsc, Mutex, OnceLock},
    time::Duration,
};

enum Message {
    Info(mpsc::Sender<Value>),
    Stop(mpsc::Sender<Result<(), String>>),
}
struct Control {
    sender: mpsc::Sender<Message>,
}
static CONTROL: OnceLock<Mutex<Option<Control>>> = OnceLock::new();
fn slot() -> &'static Mutex<Option<Control>> {
    CONTROL.get_or_init(|| Mutex::new(None))
}

pub fn start(data_dir: PathBuf, port: u16) -> Result<Value, String> {
    let mut guard = slot().lock().map_err(|_| "主机状态锁损坏")?;
    if let Some(control) = guard.as_ref() {
        if let Ok(info) = query(control) {
            return Ok(info);
        }
    }
    *guard = None;
    let (sender, receiver) = mpsc::channel();
    let (ready_tx, ready_rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("qinyou-host".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = ready_tx.send(Err(e.to_string()));
                    return;
                }
            };
            let host = match runtime.block_on(hub_server::spawn_host(data_dir, port)) {
                Ok(host) => host,
                Err(e) => {
                    let _ = ready_tx.send(Err(e.to_string()));
                    return;
                }
            };
            let info = serde_json::to_value(host.info()).map_err(|e| e.to_string());
            if ready_tx.send(info).is_err() {
                let _ = runtime.block_on(host.shutdown());
                return;
            }
            while let Ok(message) = receiver.recv() {
                match message {
                    Message::Info(reply) => {
                        let _ =
                            reply.send(serde_json::to_value(host.info()).unwrap_or(Value::Null));
                    }
                    Message::Stop(reply) => {
                        let _ = reply
                            .send(runtime.block_on(host.shutdown()).map_err(|e| e.to_string()));
                        return;
                    }
                }
            }
            let _ = runtime.block_on(host.shutdown());
        })
        .map_err(|e| e.to_string())?;
    let info = ready_rx
        .recv_timeout(Duration::from_secs(20))
        .map_err(|_| "主机启动超时")??;
    *guard = Some(Control { sender });
    Ok(info)
}
fn query(control: &Control) -> Result<Value, String> {
    let (tx, rx) = mpsc::channel();
    control
        .sender
        .send(Message::Info(tx))
        .map_err(|_| "主机服务已退出")?;
    rx.recv_timeout(Duration::from_secs(3))
        .map_err(|_| "无法读取主机状态".into())
}
pub fn info() -> Result<Option<Value>, String> {
    let guard = slot().lock().map_err(|_| "主机状态锁损坏")?;
    guard.as_ref().map(query).transpose()
}
pub fn stop() -> Result<(), String> {
    let mut guard = slot().lock().map_err(|_| "主机状态锁损坏")?;
    if let Some(control) = guard.take() {
        let (tx, rx) = mpsc::channel();
        control
            .sender
            .send(Message::Stop(tx))
            .map_err(|_| "主机服务已退出")?;
        rx.recv_timeout(Duration::from_secs(20))
            .map_err(|_| "主机停止超时")??;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_host_survives_caller_and_stops_cleanly() {
        let dir = tempfile::tempdir().unwrap();
        let first = super::start(dir.path().to_path_buf(), 0).unwrap();
        assert!(first["port"].as_u64().unwrap() > 0);
        // A later command sees the same independent runtime, not a WebView-owned task.
        assert_eq!(super::info().unwrap().unwrap()["port"], first["port"]);
        assert_eq!(
            super::start(dir.path().to_path_buf(), 0).unwrap()["port"],
            first["port"]
        );
        super::stop().unwrap();
        assert!(super::info().unwrap().is_none());
        let restored = super::start(dir.path().to_path_buf(), 0).unwrap();
        assert!(restored["port"].as_u64().unwrap() > 0);
        super::stop().unwrap();
        super::stop().unwrap();
    }
}
