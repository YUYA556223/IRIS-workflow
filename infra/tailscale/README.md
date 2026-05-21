# Tailscale tailnet セットアップ

IRIS-workflow のデバイス間通信は Tailscale (mesh VPN) を経由します。

## 1. アカウント作成 & インストール

1. [tailscale.com](https://tailscale.com/) でアカウントを作成
2. ホストPC (Windows) に Tailscale クライアントをインストール → サインイン
3. iPhone / 他PC / IoT デバイス (Raspberry Pi 等) にも同様にインストール

## 2. tag の設定

[Tailscale 管理画面](https://login.tailscale.com/admin/acls) で `acls.json` を以下のように設定:

```json
{
  "tagOwners": {
    "tag:iris-host":   ["autogroup:admin"],
    "tag:iris-device": ["autogroup:admin"]
  },
  "acls": [
    {
      "action": "accept",
      "src":    ["tag:iris-device"],
      "dst":    ["tag:iris-host:3000,8787"]
    },
    {
      "action": "accept",
      "src":    ["tag:iris-host"],
      "dst":    ["tag:iris-device:*"]
    }
  ]
}
```

ホストPC を `tag:iris-host`、iPhone・他PC・IoT を `tag:iris-device` でタグ付け。

## 3. ホスト名で疎通確認

iPhone のブラウザから:

```
http://<host-pc-name>.tailnet.ts.net:8787/health
```

を叩いて JSON が返れば成功。

## 4. 代替: 自前 WireGuard

Tailscale を使いたくない場合は `wireguard-tools` で同等構成可能。鍵管理・サーバ運用の手間が増えるため、個人用途では Tailscale 推奨。
