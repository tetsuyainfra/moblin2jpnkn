use std::collections::HashMap;
use std::fs;

pub const DEFAULT_CONFIG_PATH: &str = "/etc/moblin2jpnkn/config";

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub ws_path: String,
    pub mqtt_url: String,
    pub mqtt_username: Option<String>,
    pub mqtt_password: Option<String>,
    pub mqtt_topics: Vec<String>,
}

/// KEY=VALUE 形式（`#` コメント、前後の引用符可）を読み込む。環境変数が設定ファイルより優先される。
pub fn load(path: &str) -> Result<Config, String> {
    let mut map = HashMap::new();
    match fs::read_to_string(path) {
        Ok(text) => {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    let v = v.trim();
                    let v = v
                        .strip_prefix('"')
                        .and_then(|s| s.strip_suffix('"'))
                        .or_else(|| v.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
                        .unwrap_or(v);
                    map.insert(k.trim().to_string(), v.to_string());
                }
            }
        }
        Err(e) => return Err(format!("failed to read {path}: {e}")),
    }

    let get = |key: &str| -> Option<String> {
        std::env::var(key)
            .ok()
            .or_else(|| map.get(key).cloned())
            .filter(|v| !v.is_empty())
    };

    let mqtt_url = get("MQTT_URL").ok_or("MQTT_URL is required")?;
    let mqtt_topics: Vec<String> = get("MQTT_TOPIC")
        .ok_or("MQTT_TOPIC is required")?
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if mqtt_topics.is_empty() {
        return Err("MQTT_TOPIC must contain at least one topic".into());
    }
    let port = match get("PORT") {
        Some(p) => p.parse().map_err(|_| format!("invalid PORT: {p}"))?,
        None => 5443,
    };

    Ok(Config {
        host: get("HOST").unwrap_or_else(|| "0.0.0.0".into()),
        port,
        ws_path: get("WS_PATH").unwrap_or_else(|| "/ws".into()),
        mqtt_url,
        mqtt_username: get("MQTT_USERNAME"),
        mqtt_password: get("MQTT_PASSWORD"),
        mqtt_topics,
    })
}
