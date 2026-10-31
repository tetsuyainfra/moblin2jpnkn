mod config;
mod xmpp;

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, State};
use axum::routing::get;
use axum::Router;
use base64::Engine;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use config::Config;

const NS_FRAMING: &str = "urn:ietf:params:xml:ns:xmpp-framing";
const NS_SASL: &str = "urn:ietf:params:xml:ns:xmpp-sasl";
const NS_BIND: &str = "urn:ietf:params:xml:ns:xmpp-bind";
const NS_SESSION: &str = "urn:ietf:params:xml:ns:xmpp-session";

static CLIENT_SEQ: AtomicU64 = AtomicU64::new(0);

#[derive(PartialEq)]
enum Phase {
    Handshake,
    Auth,
    Connected,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().without_time().init();

    let path = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("MOBLIN2JPNKN_CONFIG").ok())
        .unwrap_or_else(|| config::DEFAULT_CONFIG_PATH.to_string());
    let cfg = match config::load(&path) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            error!("config error: {e}");
            std::process::exit(1);
        }
    };

    let app = Router::new()
        .route("/", get(|| async { "HTTP OK\n" }))
        .route("/health", get(|| async { "OK" }))
        .route(&cfg.ws_path, get(ws_handler))
        .with_state(cfg.clone());

    let addr = format!("{}:{}", cfg.host, cfg.port);
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("failed to bind {addr}: {e}");
            std::process::exit(1);
        }
    };
    info!("listening on http://{addr} (ws path {})", cfg.ws_path);
    info!("ws_path is http://{addr}{}", cfg.ws_path);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
}

async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut term = signal(SignalKind::terminate()).expect("SIGTERM handler");
    tokio::select! {
        _ = term.recv() => {}
        _ = tokio::signal::ctrl_c() => {}
    }
    info!("shutting down");
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(cfg): State<Arc<Config>>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
) -> axum::response::Response {
    ws.protocols(["xmpp"])
        .on_upgrade(move |socket| session(socket, cfg, remote))
}

async fn send(socket: &mut WebSocket, xml: String) -> bool {
    socket.send(Message::Text(xml.into())).await.is_ok()
}

async fn session(mut socket: WebSocket, cfg: Arc<Config>, remote: SocketAddr) {
    info!("{remote} connected");
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let mut phase = Phase::Handshake;
    let mut jid = String::new();
    let mut mqtt_task: Option<JoinHandle<()>> = None;

    loop {
        tokio::select! {
            msg = socket.recv() => {
                let text = match msg {
                    Some(Ok(Message::Text(t))) => t,
                    // curl はバイナリフレームで送ることがある
                    Some(Ok(Message::Binary(b))) => match String::from_utf8(b.to_vec()) {
                        Ok(s) => s.into(),
                        Err(_) => continue,
                    },
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(_)) => continue,
                };
                info!("{remote} received message: {}", text);
                let stanza = match xmpp::parse(text.as_str()) {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("{remote} failed to parse message: {e}");
                        break;
                    }
                };
                let ok = match phase {
                    Phase::Handshake => on_handshake(&mut socket, &mut phase, &stanza, remote).await,
                    Phase::Auth => on_auth(&mut socket, &mut phase, &stanza, &mut jid, remote).await,
                    Phase::Connected => {
                        on_connected(&mut socket, &stanza, &jid, &cfg, &tx, &mut mqtt_task, remote).await
                    }
                };
                if !ok {
                    break;
                }
            }
            Some(xml) = rx.recv() => {
                if !send(&mut socket, xml).await {
                    break;
                }
            }
        }
    }

    if let Some(t) = mqtt_task {
        t.abort();
    }
    info!("{remote} disconnected");
}

async fn on_handshake(
    socket: &mut WebSocket,
    phase: &mut Phase,
    stanza: &xmpp::Stanza,
    remote: SocketAddr,
) -> bool {
    if stanza.name != "open" {
        warn!("{remote} unexpected <{}> in handshake", stanza.name);
        return false;
    }
    *phase = Phase::Auth;
    send(socket, open_frame()).await
        && send(
            socket,
            format!(
                r#"<stream:features xmlns:stream="http://etherx.jabber.org/streams"><mechanisms xmlns="{NS_SASL}"><mechanism>PLAIN</mechanism></mechanisms></stream:features>"#
            ),
        )
        .await
}

fn open_frame() -> String {
    format!(r#"<open xmlns="{NS_FRAMING}" from="osp.dummy" id="jpnkn-chat" version="1.0"/>"#)
}

async fn on_auth(
    socket: &mut WebSocket,
    phase: &mut Phase,
    stanza: &xmpp::Stanza,
    jid: &mut String,
    remote: SocketAddr,
) -> bool {
    if stanza.name != "auth" || stanza.attr("mechanism") != Some("PLAIN") {
        warn!("{remote} unexpected <{}> in auth", stanza.name);
        return false;
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(stanza.text.trim())
        .unwrap_or_default();
    let decoded = String::from_utf8_lossy(&decoded);
    // authzid\0authcid\0password。Moblinはユーザー名 "moblin" のみ許可し、パスワードは検証しない。
    let authcid = decoded.split('\0').nth(1).unwrap_or("");
    if authcid == "moblin" {
        info!("{remote} authenticated as {authcid}");
        *jid = format!("{authcid}@osp.dummy/RESOURCE");
        *phase = Phase::Connected;
        send(socket, format!(r#"<success xmlns="{NS_SASL}"/>"#)).await
    } else {
        warn!("{remote} authentication failed");
        let _ = send(
            socket,
            format!(r#"<failure xmlns="{NS_SASL}"><not-authorized/></failure>"#),
        )
        .await;
        false
    }
}

async fn on_connected(
    socket: &mut WebSocket,
    stanza: &xmpp::Stanza,
    jid: &str,
    cfg: &Arc<Config>,
    tx: &mpsc::UnboundedSender<String>,
    mqtt_task: &mut Option<JoinHandle<()>>,
    remote: SocketAddr,
) -> bool {
    match stanza.name.as_str() {
        "open" => {
            send(socket, open_frame()).await
                && send(
                    socket,
                    format!(
                        r#"<stream:features xmlns:stream="http://etherx.jabber.org/streams"><bind xmlns="{NS_BIND}"/><session xmlns="{NS_SESSION}"/></stream:features>"#
                    ),
                )
                .await
        }
        "iq" => {
            let id = xmpp::escape(stanza.attr("id").unwrap_or(""));
            let mut result = String::new();
            for child in &stanza.children {
                match child.as_str() {
                    "bind" => result.push_str(&format!(
                        r#"<bind xmlns="{NS_BIND}"><jid>{}</jid></bind>"#,
                        xmpp::escape(jid)
                    )),
                    "session" => {}
                    other => warn!("{remote} unknown iq child: {other}"),
                }
            }
            send(
                socket,
                format!(r#"<iq xmlns="jabber:client" type="result" id="{id}">{result}</iq>"#),
            )
            .await
        }
        "presence" => {
            if !send(socket, r#"<presence xmlns="jabber:client"/>"#.to_string()).await {
                return false;
            }
            if mqtt_task.is_none() {
                match spawn_mqtt(cfg, remote, jid.to_string(), tx.clone()) {
                    Ok(h) => *mqtt_task = Some(h),
                    Err(e) => {
                        error!("{remote} MQTT setup failed: {e}");
                        return false;
                    }
                }
            }
            true
        }
        "close" => {
            let _ = send(socket, format!(r#"<close xmlns="{NS_FRAMING}"/>"#)).await;
            false
        }
        other => {
            warn!("{remote} unknown stanza: {other}");
            true
        }
    }
}

fn spawn_mqtt(
    cfg: &Config,
    remote: SocketAddr,
    jid: String,
    tx: mpsc::UnboundedSender<String>,
) -> Result<JoinHandle<()>, String> {
    let client_id = format!(
        "moblin2jpnkn-{}-{}",
        std::process::id(),
        CLIENT_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let sep = if cfg.mqtt_url.contains('?') { '&' } else { '?' };
    let url = format!("{}{sep}client_id={client_id}", cfg.mqtt_url);
    let mut opts = MqttOptions::parse_url(url).map_err(|e| e.to_string())?;
    if let Some(user) = &cfg.mqtt_username {
        opts.set_credentials(user, cfg.mqtt_password.clone().unwrap_or_default());
    }
    let topics = cfg.mqtt_topics.clone();
    let (client, mut eventloop) = AsyncClient::new(opts, 64);

    Ok(tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => {
                    info!("{remote} connected to MQTT broker");
                    for t in &topics {
                        if let Err(e) = client.try_subscribe(t, QoS::AtMostOnce) {
                            error!("{remote} subscribe error: {e}");
                        }
                    }
                }
                Ok(Event::Incoming(Packet::Publish(p))) => {
                    match chat_xml(&p.payload, &jid) {
                        Some(xml) => {
                            if tx.send(xml).is_err() {
                                break;
                            }
                        }
                        None => warn!("{remote} ignored invalid MQTT payload"),
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    warn!("{remote} MQTT error: {e}");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }))
}

fn value_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// body: "投稿者<>メール欄<>日時<>本文<>スレッド名" の4番目を本文として XMPP メッセージ化する。
fn chat_xml(payload: &[u8], to: &str) -> Option<String> {
    let v: Value = serde_json::from_slice(payload).ok()?;
    let body_raw = v.get("body").map(value_str).unwrap_or_default();
    let text = body_raw.split("<>").nth(3)?;
    let no = v.get("no").map(value_str).unwrap_or_default();
    let bbsid = v.get("bbsid").map(value_str).unwrap_or_default();
    let key = v.get("threadkey").map(value_str).unwrap_or_default();
    Some(format!(
        r#"<message xmlns="jabber:client" from="system@example.com/{}" to="{}" type="chat" id="{}"><body>{}</body></message>"#,
        xmpp::escape(&no),
        xmpp::escape(to),
        xmpp::escape(&format!("{bbsid}_{key}_{no}")),
        xmpp::escape(text),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_xml_extracts_body() {
        let p = br#"{"body":"a<>b<>c<>hello <&><>t/1","no":"1","bbsid":"x","threadkey":"2"}"#;
        let xml = chat_xml(p, "moblin@osp.dummy/RESOURCE").unwrap();
        assert!(xml.contains("<body>hello &lt;&amp;&gt;</body>"));
        assert!(xml.contains(r#"id="x_2_1""#));
        assert!(xml.contains(r#"from="system@example.com/1""#));
    }
}
