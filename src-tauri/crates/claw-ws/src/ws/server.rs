// Claw Desktop - WS服务器 - WebSocket连接管理和消息分发
use crate::ws::protocol::WsEvent;
use std::sync::{
    OnceLock,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Instant;
use tokio::sync::broadcast;

const STREAM_CHANNEL_CAPACITY: usize = 4096;

static STREAM_TX: OnceLock<broadcast::Sender<String>> = OnceLock::new();
static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();
static ACTIVE_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);
static TOTAL_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);
static SERVER_START_TIME: OnceLock<Instant> = OnceLock::new();

pub const MAX_CONNECTIONS: usize = 100;
pub const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// 获取WebSocket服务器端口
pub fn get_ws_port() -> Option<u16> {
    WS_PORT.get().copied()
}
/// 获取Tauri AppHandle
pub fn get_app_handle() -> Option<tauri::AppHandle> {
    APP_HANDLE.get().cloned()
}
/// 设置Tauri AppHandle（启动时调用一次）
pub fn set_app_handle(handle: tauri::AppHandle) {
    APP_HANDLE.get_or_init(|| handle);
}

/// 获取当前活跃连接数
pub fn get_active_connections() -> usize {
    ACTIVE_CONNECTIONS.load(Ordering::Relaxed)
}
/// 获取历史总连接数
pub fn get_total_connections() -> usize {
    TOTAL_CONNECTIONS.load(Ordering::Relaxed)
}

/// 获取服务器指标 — 活跃连接数、总连接数、运行时间等
pub fn get_metrics() -> serde_json::Value {
    let uptime_secs = SERVER_START_TIME
        .get()
        .map(|t| t.elapsed().as_secs())
        .unwrap_or(0);
    serde_json::json!({
        "active_connections": ACTIVE_CONNECTIONS.load(Ordering::Relaxed),
        "total_connections": TOTAL_CONNECTIONS.load(Ordering::Relaxed),
        "max_connections": MAX_CONNECTIONS,
        "max_message_size_bytes": MAX_MESSAGE_SIZE,
        "uptime_seconds": uptime_secs,
        "stream_channel_capacity": STREAM_CHANNEL_CAPACITY,
    })
}

/// 获取事件广播发送端 — 用于向所有WS客户端广播消息
pub fn get_stream_sender() -> broadcast::Sender<String> {
    STREAM_TX.get().expect("EventBus not initialized").clone()
}

/// 广播事件 — 通过EventBus向所有WS客户端发送事件
pub fn emit_event(event: &WsEvent) {
    if let Some(tx) = STREAM_TX.get() {
        let msg = serde_json::to_string(event).unwrap_or_default();
        if tx.send(msg).is_err() {
            log::warn!(
                "[EventBus] No active subscribers for event method={}",
                event.method
            );
        }
    }
}

/// 发送流式消息事件 — 用于对话流式响应
pub fn emit_stream(conv_id: &str, event_type: &str, data: serde_json::Value) {
    emit_event(&WsEvent::new(
        conv_id,
        "send_message_streaming",
        event_type,
        data,
    ));
}

/// 发送子Agent事件 — 用于多Agent任务的事件通知
pub fn emit_subagent_event(task_id: &str, event_type: &str, data: serde_json::Value) {
    emit_stream(task_id, event_type, data);
}

static WS_PORT: OnceLock<u16> = OnceLock::new();

/// 启动WebSocket服务器 — 绑定127.0.0.1随机端口，返回端口号
///
/// 初始化EventBus广播通道，启动连接接受循环
/// 每个连接独立spawn处理，支持最大MAX_CONNECTIONS个并发连接
pub async fn start_ws_server() -> Result<u16, String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Failed to bind WebSocket server: {}", e))?;

    let port = listener
        .local_addr()
        .map_err(|e| format!("Failed to get local address: {}", e))?
        .port();

    WS_PORT.get_or_init(|| port);

    let (stream_tx, _) = broadcast::channel::<String>(STREAM_CHANNEL_CAPACITY);
    STREAM_TX.get_or_init(|| stream_tx);
    SERVER_START_TIME.get_or_init(|| Instant::now());

    let (shutdown_tx, _) = broadcast::channel::<()>(1);

    log::info!(
        "[WS] EventBus initialized with capacity={}, port={}",
        STREAM_CHANNEL_CAPACITY,
        port
    );
    log::info!("[WS] WebSocket server starting on ws://127.0.0.1:{}", port);

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let current = ACTIVE_CONNECTIONS.fetch_add(1, Ordering::Relaxed);
                    TOTAL_CONNECTIONS.fetch_add(1, Ordering::Relaxed);

                    if current >= MAX_CONNECTIONS {
                        ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
                        log::warn!(
                            "[WS] Rejected {} - too many connections ({}/{})",
                            addr,
                            current + 1,
                            MAX_CONNECTIONS
                        );
                        return;
                    }

                    log::debug!(
                        "[WS] New connection from {} (active: {}/{})",
                        addr,
                        current + 1,
                        MAX_CONNECTIONS
                    );
                    let shutdown_rx = shutdown_tx.subscribe();
                    let stream_rx = get_stream_sender().subscribe();
                    tokio::spawn(handle_connection(stream, addr, shutdown_rx, stream_rx));
                }
                Err(e) => log::error!("[WS] Accept error: {}", e),
            }
        }
    });

    Ok(port)
}

/// 处理单个WS连接 — 消息接收、路由分发、心跳保活、广播桥接
async fn handle_connection(
    stream: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
    mut shutdown_rx: broadcast::Receiver<()>,
    mut stream_rx: broadcast::Receiver<String>,
) {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

    let ws_stream = match tokio_tungstenite::accept_hdr_async(stream, |_req: &Request, resp| {
        Ok::<_, tokio_tungstenite::tungstenite::handshake::server::ErrorResponse>(Response::from(
            resp,
        ))
    })
    .await
    {
        Ok(ws) => ws,
        Err(e) => {
            log::error!("[WS] Handshake failed for {}: {}", addr, e);
            return;
        }
    };

    log::info!("[WS] Connected: {}", addr);

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<String>(512);

    let (buffer_tx, mut buffer_rx) = tokio::sync::mpsc::channel::<String>(8192);
    let buffer_tx_for_recv = buffer_tx.clone();
    let buffer_tx_heartbeat = buffer_tx.clone();

    let sender_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = buffer_rx.recv() => match msg {
                    Some(m) => if ws_sender.send(tokio_tungstenite::tungstenite::Message::Text(m.into())).await.is_err() { break; }
                    None => break,
                },
                msg = event_rx.recv() => match msg {
                    Some(m) => if buffer_tx.send(m).await.is_err() { break; }
                    None => break,
                },
            }
        }
    });

    let bridge_task = tokio::spawn(async move {
        loop {
            match stream_rx.recv().await {
                Ok(msg) => {
                    if buffer_tx_for_recv.send(msg).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    log::warn!(
                        "[WS:{}] Broadcast lagged {} events, using local buffer to prevent data loss",
                        addr,
                        n
                    );
                }
                Err(_) => break,
            }
        }
    });

    let recv_task = tokio::spawn(async move {
        use crate::ws::protocol::{WsRequest, WsResponse};
        use crate::ws::router;
        let tx = event_tx;
        while let Some(msg_result) = ws_receiver.next().await {
            match msg_result {
                Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                    if text.len() > MAX_MESSAGE_SIZE {
                        log::warn!(
                            "[WS:{}] Message too large ({} bytes > {}), dropping",
                            addr,
                            text.len(),
                            MAX_MESSAGE_SIZE
                        );
                        continue;
                    }

                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                        if v.get("type").and_then(|s| s.as_str()) == Some("ping") {
                            let _ = tx
                                .send(serde_json::json!({"type":"pong"}).to_string())
                                .await;
                            continue;
                        }
                    }
                    match serde_json::from_str::<WsRequest>(&text) {
                        Ok(req) => {
                            log::debug!("[WS] {} method={}", addr, req.method);
                            let resp = router::dispatch(req).await;
                            let _ = tx
                                .send(serde_json::to_string(&resp).unwrap_or_default())
                                .await;
                        }
                        Err(_) => {
                            let err = WsResponse::err("0", "unknown", "Invalid request");
                            let _ = tx
                                .send(serde_json::to_string(&err).unwrap_or_default())
                                .await;
                        }
                    }
                }
                Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => {
                    log::info!("[WS] Disconnected: {}", addr);
                    break;
                }
                Err(e) => {
                    log::error!("[WS] Error from {}: {}", addr, e);
                    break;
                }
                _ => {}
            }
        }
        ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
    });

    tokio::select! {
        _ = recv_task => {}
        _ = async {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                if buffer_tx_heartbeat.send(r#"{"type":"ping"}"#.to_string()).await.is_err() {
                    log::debug!("[WS] Heartbeat ping failed for {}", addr);
                    break;
                }
            }
        } => { log::info!("[WS] Heartbeat ended: {}", addr); }
        _ = shutdown_rx.recv() => {}
    }

    sender_task.abort();
    bridge_task.abort();
    log::debug!("[WS] Connection handler ended: {}", addr);
}
