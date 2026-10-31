# moblin2jpnkn

Moblin のチャットクライアントと JPNKN 互換 MQTT ブローカーを接続する、XMPP-over-WebSocket サーバーです。MQTT の指定トピックを購読し、受信した投稿を XMPP チャットメッセージとして接続中の Moblin クライアントへ転送します。

## 機能

- XMPP-over-WebSocket エンドポイントを提供します（既定値: `/ws`）。
- MQTT ブローカーに接続し、設定されたトピックを購読します。
- MQTT の JSON メッセージから投稿本文を取り出して、XMPP メッセージとして送信します。
- `/` と `/health` の HTTP ヘルスチェックエンドポイントを提供します。
- SIGINT または SIGTERM で正常終了します。

## 必要環境

- Rust stable（ソースからビルドする場合）
- 接続可能な MQTT ブローカー
- Moblin クライアントから到達可能なサーバー環境

## 設定

設定ファイルは `KEY=VALUE` 形式です。空行と `#` で始まる行は無視され、値は必要に応じてシングルクォートまたはダブルクォートで囲めます。設定ファイルより環境変数が優先されます。

| キー | 必須 | 既定値 | 説明 |
| --- | --- | --- | --- |
| `MQTT_URL` | はい | なし | MQTT ブローカー URL（例: `mqtt://localhost:1883`） |
| `MQTT_TOPIC` | はい | なし | 購読するトピック。複数指定はカンマ区切り |
| `MQTT_USERNAME` | いいえ | なし | MQTT 認証ユーザー名 |
| `MQTT_PASSWORD` | いいえ | なし | MQTT 認証パスワード |
| `HOST` | いいえ | `0.0.0.0` | HTTP/WebSocket の待受ホスト |
| `PORT` | いいえ | `5443` | HTTP/WebSocket の待受ポート |
| `WS_PATH` | いいえ | `/ws` | XMPP-over-WebSocket のパス |

設定例は [`packaging/config.example`](packaging/config.example) を参照してください。

## ビルドと起動

```sh
cargo build --release
```

設定ファイルのパスは `--config`（短縮形 `-c`）で指定します。

```sh
./target/release/moblin2jpnkn --config ./config
```

設定パスの優先順位は、`--config`、`MOBLIN2JPNKN_CONFIG` 環境変数、`/etc/moblin2jpnkn/config` の順です。位置引数での指定には対応していません。利用できるオプションは `--help` で確認できます。

```sh
MOBLIN2JPNKN_CONFIG=./config ./target/release/moblin2jpnkn
# or
MOBLIN2JPNKN_CONFIG=/etc/moblin2jpnkn/config cargo run
```

miseを使い、コンフィグを指定できます。(楽)

```sh
cp ./packaging/config.example ./config.local
cp ./mise.local.toml-example ./mise.local.toml

vi ./config.local

cat ./mise.local.toml
[env]
MOBLIN2JPNKN_CONFIG="{{config_root}}/config.local"

cargo run           # development
cargo run --release # production
```


## 接続とメッセージ形式

Moblin クライアントの XMPP-over-WebSocket 接続先は `ws[s]://<ホスト>:<ポート><WS_PATH>` です。サーバーは WebSocket の `xmpp` サブプロトコルを提示します。

XMPP 認証は SASL PLAIN を使用します。ユーザー名 `moblin` のみ受け付け、パスワードは検証しません。通信をインターネット越しに利用する場合は、TLS 終端を行うリバースプロキシなどを設置してください。

MQTT のペイロードは、少なくとも `body` を含む JSON オブジェクトを想定しています。`body` は `<>` 区切りの文字列で、4 番目の要素がチャット本文として転送されます。メッセージ ID には `bbsid`、`threadkey`、`no` が使用されます。

例:

```json
{
	"body": "投稿者<>メール欄<>日時<>投稿本文<>スレッド名",
	"no": "1",
	"bbsid": "example",
	"threadkey": "thread-1"
}
```

`/` は `HTTP OK`、`/health` は `OK` を返します。どちらも設定した `HOST` と `PORT` で確認できます。

## Debian パッケージ

Debian パッケージは `cargo-deb` で作成できます。

```sh
cargo install cargo-deb
cargo deb

# Build and install on the system
cargo deb --install
```

パッケージには実行ファイル、設定例、systemd ユニットが含まれます。インストール後、`/etc/moblin2jpnkn/config` を編集して MQTT 接続情報を設定し、サービスを起動してください。

```sh
sudo systemctl enable --now moblin2jpnkn
sudo systemctl status moblin2jpnkn
```
