# iris-mcp-permission

Claude Code の `--permission-prompt-tool` を受け、ホスト (`host-backend`) の
`POST /permission/request` HTTP API に転送する **stdio JSON-RPC 2.0 サーバ**。

## ビルド

```powershell
cargo build -p iris-mcp-permission --release
```

出力: `target/release/iris-mcp-permission.exe`

## 使い方

1. ホスト (`host-backend`) を起動しておく (デフォルト `127.0.0.1:8787`)。
2. Claude Code 用の MCP 設定 JSON を用意:

```json
{
  "mcpServers": {
    "iris-permission": {
      "command": "iris-mcp-permission",
      "env": { "IRIS_BACKEND_URL": "http://127.0.0.1:8787" }
    }
  }
}
```

3. Claude Code 起動引数に `--mcp-config <path>` と
   `--permission-prompt-tool mcp__iris-permission__prompt` を渡す。

これで Claude が許可要する tool を使うたびに、本 bridge が
`POST /permission/request` を叩き、応答が返るまでブロックする。

## プロトコル概要

- JSON-RPC 2.0 over stdio (改行区切り)
- 対応メソッド:
  - `initialize` — `protocolVersion` 等を返す
  - `tools/list` — `prompt` tool 定義を返す
  - `tools/call` — `prompt` を呼ぶと `IRIS_BACKEND_URL/permission/request` に転送
  - `notifications/*` — 無視
- `tools/call` の引数 `arguments` (JSON object) はそのまま host に転送される:
  - `tool_name`: 許可要する Claude tool 名 (Bash / Edit / ...)
  - `tool_input`: 元の tool 入力引数 (任意 object)
- ホストからの応答は `{ behavior: "allow"|"deny", updatedInput?, message? }`。
  `content[].text` に JSON.stringify した形で Claude へ返す。

## ログ

stderr に `tracing` で出力 (stdout は JSON-RPC 用に予約)。

```powershell
$env:RUST_LOG = 'info'
```

## 環境変数

| 変数 | 既定 | 説明 |
|---|---|---|
| `IRIS_BACKEND_URL` | `http://127.0.0.1:8787` | ホスト host-backend の base URL |
| `RUST_LOG` | (未設定 = 通常) | tracing フィルタ |

## トラブルシュート

- 対話 stdin に対しては起動拒否 (`if std::io::stdin().is_terminal()`)。
  必ず Claude Code の `--mcp-config` から spawn される形で実行する。
- ホスト不到達の場合は MCP 経由で `-32603` エラーが Claude に返る。
- `IRIS_PERMISSION_TIMEOUT_SECS` (ホスト側設定) で 120 秒以内にデバイス応答が
  無いと Claude 側で deny 扱いになる。
