# Windows MSIX パッケージング (P9)

Flutter Windows desktop (`apps/desktop`) を MSIX 形式で配布するための手順。

## 構成

- `apps/desktop/pubspec.yaml` の `dev_dependencies.msix` および `msix_config:`
  セクションに本体設定がある。
- `scripts/build-desktop-msix.ps1` がビルド〜パッケージ生成を一括実行する。

## 前提

- Windows 10/11
- Visual Studio 2022 Build Tools (Desktop development with C++ ワークロード)
- Flutter SDK with Windows desktop が有効化済み (`flutter config --enable-windows-desktop`)

## 手順

### A. 開発用 (署名なし / 自己署名)

開発機での動作確認だけなら署名なしでも `Add-AppxPackage` でインストール可能。
**Developer Mode** を有効化しておくこと (`Settings > For Developers > Developer Mode`).

```powershell
# プロジェクトルートで:
.\scripts\build-desktop-msix.ps1 -Install
```

`-Install` フラグを付けると `Add-AppxPackage` まで自動実行する。

### B. 自己署名証明書で sideload 配布

他PCで動かす場合は信頼された証明書での署名が必要。

```powershell
# 1) 自己署名証明書を生成
$cert = New-SelfSignedCertificate `
  -Type CodeSigningCert `
  -Subject "CN=IRIS-workflow Dev, O=Memorylab Inc., C=JP" `
  -KeyUsage DigitalSignature `
  -FriendlyName "IRIS dev cert" `
  -CertStoreLocation "Cert:\CurrentUser\My" `
  -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")

# 2) .pfx ファイルに書き出し
$pwd = ConvertTo-SecureString -String "REPLACE_ME" -Force -AsPlainText
Export-PfxCertificate -Cert $cert -FilePath iris-signing.pfx -Password $pwd

# 3) 公開鍵 (.cer) を信頼ストアにインポート (配布先 PC で)
Export-Certificate -Cert $cert -FilePath iris-signing.cer
Import-Certificate -FilePath iris-signing.cer -CertStoreLocation Cert:\LocalMachine\TrustedPeople

# 4) pubspec.yaml の msix_config に証明書を指定して再パッケージ
#    certificate_path: C:\path\to\iris-signing.pfx
#    certificate_password: REPLACE_ME (環境変数推奨)

$env:IRIS_SIGN_PASSWORD = "REPLACE_ME"
.\scripts\build-desktop-msix.ps1
```

なお `iris-signing.pfx` は **絶対に git commit しない**。`.gitignore` で
`*.pfx` を除外しておく。

### C. Microsoft Store 配布

Partner Center で予約した Identity / Publisher を `msix_config` に書き込み、
`store: true` に切替。証明書は Microsoft 側が付与するので不要。

詳細: <https://learn.microsoft.com/en-us/windows/msix/package/packaging-uwp-apps>

## 既知の制約

- iris-mcp-permission や host-backend は **別パッケージ**。MSIX 配布する場合は
  iris-desktop の MSIX に同梱するか、別途配布する。
- アイコンは `apps/desktop/windows/runner/resources/app_icon.ico`。Store 提出
  時には複数解像度の PNG を用意する必要あり。
- MSIX 内で `host-backend.exe` を起動する必要がある場合、`fullTrustProcess`
  capability と独立した実行ファイル登録が必要 (将来課題)。
