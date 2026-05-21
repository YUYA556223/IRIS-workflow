# @iris/proto

IRIS-workflow の共有スキーマ (JSONSchema / OpenAPI / Protobuf) を集約する場所です。

## ファイル

| ファイル | 内容 |
| --- | --- |
| `workflow.schema.json` | ワークフロー DSL (YAML/JSON) のスキーマ |
| `sdui.schema.json` | Server-driven UI コンポーネントツリーのスキーマ |
| `device.schema.json` | デバイスレジストリ / MQTT メッセージのスキーマ |

## 利用

- Rust: `serde_json` + `schemars` で生成・参照
- TypeScript: `openapi-typescript` / `json-schema-to-typescript` で型生成
- Dart: `json_serializable` + 自作 codegen
