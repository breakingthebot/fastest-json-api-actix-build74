//! src/handlers/websocket.rs
//! WebSocket connection upgrade and live telemetry streaming handler.
//! Connects to: src/services/websocket_broadcaster.rs, src/handlers/mod.rs
//! Created: 2026-08-28

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use actix_ws::Message;
use futures_util::StreamExt;
use std::sync::Arc;

use crate::models::{WsClientCommand, WsCommandResponse};
use crate::services::{
    MetricsService, RingBufferService, ShardedCacheService, WebSocketBroadcaster,
};

/// Handler for `GET /ws/metrics` and `GET /api/v1/stream/metrics`.
/// Upgrades HTTP request to WebSocket and streams real-time telemetry frames.
pub async fn ws_metrics_stream(
    req: HttpRequest,
    stream: web::Payload,
    broadcaster: web::Data<Arc<WebSocketBroadcaster>>,
    metrics: web::Data<Arc<MetricsService>>,
    ring_buffer: web::Data<Arc<RingBufferService>>,
    cache: web::Data<Arc<ShardedCacheService>>,
) -> Result<HttpResponse, actix_web::Error> {
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;
    let mut rx = broadcaster.subscribe();

    let metrics_clone = Arc::clone(&metrics);
    let ring_buffer_clone = Arc::clone(&ring_buffer);
    let cache_clone = Arc::clone(&cache);
    let broadcaster_clone = Arc::clone(&broadcaster);

    actix_rt::spawn(async move {
        // Send initial instant frame immediately upon connection
        let initial_frame = broadcaster_clone.build_current_frame(
            &metrics_clone,
            &ring_buffer_clone,
            &cache_clone,
        );
        if let Ok(json_str) = serde_json::to_string(&initial_frame) {
            let _ = session.text(json_str).await;
        }

        loop {
            tokio::select! {
                // 1. Broadcast telemetry frames to client
                Ok(frame) = rx.recv() => {
                    if let Ok(json_str) = serde_json::to_string(&frame) {
                        if session.text(json_str).await.is_err() {
                            break;
                        }
                    }
                }

                // 2. Inbound messages from WebSocket client
                Some(Ok(msg)) = msg_stream.next() => {
                    match msg {
                        Message::Text(text) => {
                            let text_str = text.trim();
                            if let Ok(cmd) = serde_json::from_str::<WsClientCommand>(text_str) {
                                match cmd.command.as_str() {
                                    "ping" => {
                                        let resp = WsCommandResponse {
                                            status: "ok".to_string(),
                                            message: "pong".to_string(),
                                            data: None,
                                            timestamp: chrono::Utc::now().to_rfc3339(),
                                        };
                                        if let Ok(s) = serde_json::to_string(&resp) {
                                            let _ = session.text(s).await;
                                        }
                                    }
                                    "get_snapshot" => {
                                        let snap = broadcaster_clone.build_current_frame(
                                            &metrics_clone,
                                            &ring_buffer_clone,
                                            &cache_clone,
                                        );
                                        if let Ok(s) = serde_json::to_string(&snap) {
                                            let _ = session.text(s).await;
                                        }
                                    }
                                    "reset_metrics" => {
                                        metrics_clone.reset();
                                        let resp = WsCommandResponse {
                                            status: "ok".to_string(),
                                            message: "Metrics have been atomically reset".to_string(),
                                            data: None,
                                            timestamp: chrono::Utc::now().to_rfc3339(),
                                        };
                                        if let Ok(s) = serde_json::to_string(&resp) {
                                            let _ = session.text(s).await;
                                        }
                                    }
                                    "drain_buffer" => {
                                        let drained = ring_buffer_clone.drain();
                                        let resp = WsCommandResponse {
                                            status: "ok".to_string(),
                                            message: format!("Drained {} events from ring buffer", drained.len()),
                                            data: Some(serde_json::json!({ "drained_count": drained.len() })),
                                            timestamp: chrono::Utc::now().to_rfc3339(),
                                        };
                                        if let Ok(s) = serde_json::to_string(&resp) {
                                            let _ = session.text(s).await;
                                        }
                                    }
                                    _ => {
                                        let resp = WsCommandResponse {
                                            status: "error".to_string(),
                                            message: format!("Unknown command: {}", cmd.command),
                                            data: None,
                                            timestamp: chrono::Utc::now().to_rfc3339(),
                                        };
                                        if let Ok(s) = serde_json::to_string(&resp) {
                                            let _ = session.text(s).await;
                                        }
                                    }
                                }
                            } else if text_str == "ping" {
                                let _ = session.text("{\"status\":\"ok\",\"message\":\"pong\"}").await;
                            }
                        }
                        Message::Ping(bytes) => {
                            if session.pong(&bytes).await.is_err() {
                                break;
                            }
                        }
                        Message::Close(reason) => {
                            let _ = session.close(reason).await;
                            break;
                        }
                        _ => {}
                    }
                }

                else => break,
            }
        }
    });

    Ok(response)
}

/// Handler for `GET /dashboard` and `GET /stream/dashboard`.
/// Serves a self-contained zero-dependency live telemetry web dashboard.
pub async fn get_live_dashboard() -> impl Responder {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>Actix Ultra-Fast API — Real-Time Telemetry Stream</title>
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <style>
    :root {
      --bg: #0d1117;
      --card-bg: #161b22;
      --border: #30363d;
      --text: #c9d1d9;
      --text-muted: #8b949e;
      --accent: #58a6ff;
      --green: #3fb950;
      --yellow: #d29922;
      --red: #f85149;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, monospace; background: var(--bg); color: var(--text); padding: 24px; }
    header { display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border); padding-bottom: 16px; margin-bottom: 24px; }
    h1 { font-size: 1.4rem; color: #fff; display: flex; align-items: center; gap: 8px; }
    .badge { font-size: 0.75rem; padding: 3px 8px; border-radius: 12px; font-weight: 600; text-transform: uppercase; }
    .badge-connected { background: rgba(63,185,80,0.2); color: var(--green); border: 1px solid var(--green); }
    .badge-disconnected { background: rgba(248,81,73,0.2); color: var(--red); border: 1px solid var(--red); }
    .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 16px; margin-bottom: 24px; }
    .card { background: var(--card-bg); border: 1px solid var(--border); border-radius: 8px; padding: 16px; }
    .card-label { font-size: 0.8rem; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 6px; }
    .card-value { font-size: 1.8rem; font-weight: 700; color: #fff; }
    .card-sub { font-size: 0.75rem; color: var(--text-muted); margin-top: 4px; }
    .controls { display: flex; gap: 12px; margin-bottom: 24px; }
    button { background: #21262d; color: var(--text); border: 1px solid var(--border); border-radius: 6px; padding: 8px 16px; font-size: 0.85rem; font-weight: 600; cursor: pointer; transition: 0.2s; }
    button:hover { background: #30363d; border-color: #8b949e; color: #fff; }
    #log { background: #000; border: 1px solid var(--border); border-radius: 8px; padding: 16px; height: 180px; overflow-y: auto; font-family: monospace; font-size: 0.75rem; color: #7ee787; }
  </style>
</head>
<body>
  <header>
    <h1>⚡ Actix Ultra-Fast JSON API <span style="font-size:0.9rem; color:var(--text-muted);">(Build 74)</span></h1>
    <div id="status-badge" class="badge badge-disconnected">Connecting...</div>
  </header>

  <div class="grid">
    <div class="card">
      <div class="card-label">Current Throughput</div>
      <div class="card-value" id="val-rps">0.0</div>
      <div class="card-sub">requests / second</div>
    </div>
    <div class="card">
      <div class="card-label">Total Requests</div>
      <div class="card-value" id="val-total-reqs">0</div>
      <div class="card-sub" id="val-uptime">Uptime: 0s</div>
    </div>
    <div class="card">
      <div class="card-label">Internal Latency (P50)</div>
      <div class="card-value" id="val-p50" style="color:var(--green)">0 μs</div>
      <div class="card-sub">0.000 ms median</div>
    </div>
    <div class="card">
      <div class="card-label">Internal Latency (P99)</div>
      <div class="card-value" id="val-p99" style="color:var(--yellow)">0 μs</div>
      <div class="card-sub">0.000 ms tail</div>
    </div>
    <div class="card">
      <div class="card-label">Ring Buffer Occupancy</div>
      <div class="card-value" id="val-buffer">0 / 65,536</div>
      <div class="card-sub" id="val-buffer-pushed">Pushed: 0 events</div>
    </div>
    <div class="card">
      <div class="card-label">Cache Hit Ratio</div>
      <div class="card-value" id="val-cache-ratio" style="color:var(--accent)">0.0%</div>
      <div class="card-sub" id="val-cache-keys">Active Keys: 0 (64 Shards)</div>
    </div>
  </div>

  <div class="controls">
    <button onclick="sendCommand('ping')">📡 Ping Server</button>
    <button onclick="sendCommand('get_snapshot')">📸 Get Snapshot</button>
    <button onclick="sendCommand('reset_metrics')">🔄 Reset Metrics</button>
    <button onclick="sendCommand('drain_buffer')">🧹 Drain Ring Buffer</button>
  </div>

  <div class="card-label" style="margin-bottom: 8px;">Live Stream Frames (WebSocket /ws/metrics)</div>
  <div id="log"></div>

  <script>
    const logEl = document.getElementById('log');
    const badgeEl = document.getElementById('status-badge');
    const wsProto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${wsProto}//${window.location.host}/ws/metrics`;

    let ws;

    function connect() {
      ws = new WebSocket(wsUrl);

      ws.onopen = () => {
        badgeEl.textContent = 'LIVE STREAMING';
        badgeEl.className = 'badge badge-connected';
        appendLog('[System] Connected to WebSocket telemetry stream');
      };

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          if (data.total_requests !== undefined) {
            document.getElementById('val-rps').textContent = data.current_rps.toLocaleString(undefined, {minimumFractionDigits: 1, maximumFractionDigits: 1});
            document.getElementById('val-total-reqs').textContent = data.total_requests.toLocaleString();
            document.getElementById('val-uptime').textContent = `Uptime: ${data.uptime_seconds}s | Active: ${data.active_requests}`;
            document.getElementById('val-p50').textContent = `${data.p50_us} μs`;
            document.getElementById('val-p99').textContent = `${data.p99_us} μs`;
            document.getElementById('val-buffer').textContent = `${data.ring_buffer_occupancy.toLocaleString()} / 65,536`;
            document.getElementById('val-buffer-pushed').textContent = `Pushed: ${data.ring_buffer_total_pushed.toLocaleString()} events`;
            document.getElementById('val-cache-ratio').textContent = `${data.cache_hit_ratio_pct.toFixed(1)}%`;
            document.getElementById('val-cache-keys').textContent = `Active Keys: ${data.cache_total_keys} (64 Shards)`;
          } else {
            appendLog(`[Response] ${event.data}`);
          }
        } catch (e) {
          appendLog(`[Raw] ${event.data}`);
        }
      };

      ws.onclose = () => {
        badgeEl.textContent = 'DISCONNECTED';
        badgeEl.className = 'badge badge-disconnected';
        appendLog('[System] WebSocket disconnected. Reconnecting in 2s...');
        setTimeout(connect, 2000);
      };
    }

    function appendLog(msg) {
      const line = document.createElement('div');
      line.textContent = `[${new Date().toLocaleTimeString()}] ${msg}`;
      logEl.prepend(line);
      if (logEl.children.length > 50) logEl.removeChild(logEl.lastChild);
    }

    function sendCommand(cmd) {
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ command: cmd }));
        appendLog(`[Sent] Command: ${cmd}`);
      }
    }

    connect();
  </script>
</body>
</html>
"#;

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}
