//! MQTT 統合 (P8)。
//!
//! `MqttBus` は単一の `rumqttc::AsyncClient` を保持し、専用タスクで eventloop を
//! 駆動する。受信メッセージは内部 `broadcast` チャネルに転送され、購読者は
//! `Receiver` でフィルタリングする (購読 topic はトリガ側で個別管理)。
//!
//! - `IRIS_MQTT_BROKER` (例: `tcp://127.0.0.1:1883`) を設定すると有効化
//! - `trigger: { type: mqtt, topic: "..." }` で workflow を起動
//! - `action: builtin/mqtt-publish` でメッセージ送信

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
    /// 接続失敗時はエラーを返す (タスクは再接続を rumqttc に任せる)。
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
    /// 受信側で topic マッチを自前フィルタする (broadcast の特性上、全 sub 共通)。
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

/// MQTT topic wildcard マッチ。`+` (単一階層), `#` (末尾複数階層) をサポート。
pub fn topic_matches(filter: &str, topic: &str) -> bool {
    let f_parts: Vec<&str> = filter.split('/').collect();
    let t_parts: Vec<&str> = topic.split('/').collect();
    let mut fi = 0;
    let mut ti = 0;
    while fi < f_parts.len() {
        match f_parts[fi] {
            "#" => return true,
            "+" => {
                if ti >= t_parts.len() {
                    return false;
                }
            }
            literal => {
                if ti >= t_parts.len() || t_parts[ti] != literal {
                    return false;
                }
            }
        }
        fi += 1;
        ti += 1;
    }
    ti == t_parts.len()
}

fn parse_broker_url(url: &str) -> anyhow::Result<(String, u16)> {
    // 受け付ける形式:
    //   tcp://host:port
    //   mqtt://host:port
    //   host:port
    //   host (default port 1883)
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
                let payload_vec = p.payload.to_vec();
                let payload_str = std::str::from_utf8(&payload_vec)
                    .ok()
                    .map(|s| s.to_owned());
                let msg = MqttMessage {
                    topic: p.topic.clone(),
                    payload: payload_vec,
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

#[cfg(test)]
mod tests {
    use super::topic_matches;

    #[test]
    fn exact() {
        assert!(topic_matches("home/light", "home/light"));
        assert!(!topic_matches("home/light", "home/door"));
    }

    #[test]
    fn plus_wildcard() {
        assert!(topic_matches("home/+/state", "home/light/state"));
        assert!(topic_matches("home/+/state", "home/door/state"));
        assert!(!topic_matches("home/+/state", "home/state"));
        assert!(!topic_matches("home/+/state", "home/light/x"));
    }

    #[test]
    fn hash_wildcard() {
        assert!(topic_matches("home/#", "home/light/state"));
        assert!(topic_matches("home/#", "home"));
        assert!(!topic_matches("garage/#", "home/light"));
    }
}
