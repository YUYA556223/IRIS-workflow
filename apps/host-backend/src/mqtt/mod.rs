//! MQTT 統合 (P8)。
//!
//! `MqttBus` は単一の `rumqttc::AsyncClient` を保持し、専用タスクで eventloop を
//! 駆動する。受信メッセージは内部 `broadcast` チャネルに転送され、購読者は
//! `Receiver` でフィルタリングする (購読 topic はトリガ側で個別管理)。
//!
//! - `IRIS_MQTT_BROKER` (例: `tcp://127.0.0.1:1883`) を設定すると有効化
//! - `trigger: { type: mqtt, topic: "..." }` で workflow を起動
//! - `action: builtin/mqtt-publish` でメッセージ送信
//!
//! Topic wildcard マッチング (`+`, `#`) は `rumqttc::matches(topic, filter)` を使う
//! (自前実装は重複を避けて削除済)。

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use rumqttc::{AsyncClient, Event, EventLoop, Incoming, MqttOptions, QoS};
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub struct MqttMessage {
    pub topic: String,
    pub payload: Vec<u8>,
    /// UTF-8 として解釈可能なら文字列、不可能なら `None`。
    pub payload_str: Option<String>,
    pub qos: u8,
}

pub struct MqttBus {
    client: AsyncClient,
    incoming_tx: broadcast::Sender<MqttMessage>,
}

impl MqttBus {
    /// ブローカに接続し、eventloop を回す background task を spawn する。
    pub async fn connect(broker_url: &str, client_id: &str) -> anyhow::Result<Arc<Self>> {
        let (host, port) = parse_broker_url(broker_url)?;
        let mut options = MqttOptions::new(client_id, host.clone(), port);
        options.set_keep_alive(Duration::from_secs(30));
        options.set_clean_session(true);

        let (client, eventloop) = AsyncClient::new(options, 64);
        let (tx, _) = broadcast::channel::<MqttMessage>(256);
        let bus = Arc::new(Self {
            client,
            incoming_tx: tx.clone(),
        });

        tokio::spawn(eventloop_task(eventloop, tx));
        tracing::info!(broker = %broker_url, %client_id, "mqtt bus connected");
        Ok(bus)
    }

    /// 指定トピックを SUBSCRIBE し、全 incoming メッセージを受信する Receiver を返す。
    /// 受信側で `rumqttc::matches` フィルタを当てる (broadcast の特性上、全 sub 共通)。
    pub async fn subscribe(&self, topic: &str) -> anyhow::Result<broadcast::Receiver<MqttMessage>> {
        self.client
            .subscribe(topic, QoS::AtMostOnce)
            .await
            .with_context(|| format!("mqtt subscribe '{}'", topic))?;
        Ok(self.incoming_tx.subscribe())
    }

    pub async fn publish(
        &self,
        topic: &str,
        payload: impl Into<Vec<u8>>,
        retain: bool,
    ) -> anyhow::Result<()> {
        self.client
            .publish(topic, QoS::AtMostOnce, retain, payload)
            .await
            .with_context(|| format!("mqtt publish '{}'", topic))?;
        Ok(())
    }
}

fn parse_broker_url(url: &str) -> anyhow::Result<(String, u16)> {
    // 受け付ける形式: `tcp://host:port`, `mqtt://host:port`, `host:port`, `host`.
    let stripped = url
        .strip_prefix("tcp://")
        .or_else(|| url.strip_prefix("mqtt://"))
        .unwrap_or(url);
    let (host, port) = match stripped.rsplit_once(':') {
        Some((h, p)) => (
            h.to_owned(),
            p.parse::<u16>()
                .with_context(|| format!("invalid port in '{}'", url))?,
        ),
        None => (stripped.to_owned(), 1883_u16),
    };
    if host.is_empty() {
        anyhow::bail!("empty mqtt broker host in '{}'", url);
    }
    Ok((host, port))
}

async fn eventloop_task(mut eventloop: EventLoop, tx: broadcast::Sender<MqttMessage>) {
    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Incoming::Publish(p))) => {
                // UTF-8 valid なら String を 1 回だけ確保し、bytes はそこから派生させる。
                let (payload, payload_str) = match String::from_utf8(p.payload.to_vec()) {
                    Ok(s) => (s.as_bytes().to_vec(), Some(s)),
                    Err(e) => (e.into_bytes(), None),
                };
                let msg = MqttMessage {
                    topic: p.topic.clone(),
                    payload,
                    payload_str,
                    qos: p.qos as u8,
                };
                let _ = tx.send(msg);
            }
            Ok(Event::Incoming(other)) => {
                tracing::trace!(?other, "mqtt incoming event");
            }
            Ok(Event::Outgoing(_)) => {}
            Err(e) => {
                tracing::warn!(error = %e, "mqtt eventloop error, will retry");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}
