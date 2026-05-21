# Packaging

P9 (Packaging) で実装する配布形式の設定置き場。

- `windows-msix/` : Windows MSIX パッケージ設定 (Flutter Windows + host-backend を同梱)
- `macos-pkg/`   : (将来) macOS pkg / dmg

## Windows MSIX

`msix` Flutter プラグインで Flutter desktop を MSIX 化し、`host-backend.exe` を同梱する想定。
署名証明書は `iris-signing.pfx` (このリポジトリにはコミットしない)。

## 認証情報

`AzureSignTool` / `signtool` で署名する場合の手順は P9 で詳述。
