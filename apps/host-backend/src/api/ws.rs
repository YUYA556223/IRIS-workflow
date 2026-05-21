//! WebSocket セッション。
//!
//! プロトコル (skeleton):
//!   client → server (hello):
//!     `{ "type": "hello", "device_id": "<uuid>" }`
//!   client → server (event):
//!     `{ "type": "event", "name": "action.invoke", "payload": { ... } }`
//!   server → client:
//!     `DeliveryEvent` (`{ "type": "notification-delivered", ... }` 等)
//!
//! デバイスは事前に `POST /devices` で登録されている必要がある。

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;

use crate::{
    domain::{Device, DeviceId},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/ws", get(upgrade))
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum ClientMessage {
    Hello {
        device_id: DeviceId,
    },
    Event {
        name: String,
        #[serde(default)]
        payload: serde_json::Value,
    },
}

async fn upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle(socket, state))
}

async fn handle(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // 1. Wait for hello
    let device_id = match recv_hello(&mut receiver).await {
        Some(id) => id,
        None => {
            tracing::warn!("ws closed before hello");
            return;
        }
    };

    // 2. Verify device is registered
    let device: Device = match state.devices.get(device_id).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            tracing::warn!(%device_id, "ws hello from unregistered device");
            let _ = sender.send(Message::Close(None)).await;
            return;
        }
        Err(e) => {
            tracing::error!(error = %e, "device lookup failed");
            return;
        }
    };

    tracing::info!(%device_id, name = %device.name, kind = ?device.kind, "ws connected");
    let device_clone = device.clone();

    // 3. Subscribe to delivery hub
    let mut rx = state.delivery.subscribe();

    // 4. Send loop: forward matching envelopes to client
    let mut send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    if !envelope.target.matches(&device_clone) {
                        continue;
                    }
                    let payload = match serde_json::to_string(&envelope.event) {
                        Ok(p) => p,
                        Err(e) => {
                            tracing::error!(error = %e, "serialize event");
                            continue;
                        }
                    };
                    if sender.send(Message::Text(payload)).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(lagged = n, "ws receiver lagged");
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // 5. Recv loop: handle inbound events
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(t) => match serde_json::from_str::<ClientMessage>(&t) {
                    Ok(ClientMessage::Event { name, payload }) => {
                        // TODO(P3): action ハンドラへルーティング (workflow trigger 経由)
                        tracing::info!(%device_id, name, ?payload, "ws event");
                    }
                    Ok(ClientMessage::Hello { .. }) => {
                        tracing::debug!(%device_id, "unexpected re-hello, ignored");
                    }
                    Err(e) => {
                        tracing::warn!(%device_id, error = %e, raw = %t, "bad client msg");
                    }
                },
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // 6. Wait for either side to finish, abort the other
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    tracing::info!(%device_id, "ws disconnected");
}

async fn recv_hello<S>(receiver: &mut S) -> Option<DeviceId>
where
    S: StreamExt<Item = Result<Message, axum::Error>> + Unpin,
{
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(t) = msg {
            if let Ok(ClientMessage::Hello { device_id }) = serde_json::from_str(&t) {
                return Some(device_id);
            }
        }
    }
    None
}
