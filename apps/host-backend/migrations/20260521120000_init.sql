-- IRIS-workflow host-backend 初期スキーマ。
--
-- 設計方針:
--  * id 列はドメイン型 (DeviceId / WidgetId) と 1:1 で UUID。
--  * 種別 (kind / priority など) は TEXT に kebab-case で格納。
--    対応する Rust 側 enum に sqlx::Type を derive して直接マップする。
--  * 構造データ (capabilities / target / root / bindings) は JSONB。
--    インデックスは必要になり次第追加 (JSONB containment 検索を想定し GIN を視野に)。

CREATE TABLE IF NOT EXISTS devices (
    id            UUID         PRIMARY KEY,
    kind          TEXT         NOT NULL,
    name          TEXT         NOT NULL,
    capabilities  JSONB        NOT NULL DEFAULT '[]'::jsonb,
    registered_at TIMESTAMPTZ  NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS devices_kind_idx ON devices (kind);
CREATE INDEX IF NOT EXISTS devices_caps_idx ON devices USING GIN (capabilities);

CREATE TABLE IF NOT EXISTS sdui_specs (
    id        TEXT   PRIMARY KEY,
    kind      TEXT   NOT NULL,
    root      JSONB  NOT NULL,
    bindings  JSONB  NOT NULL DEFAULT '{}'::jsonb
);

CREATE TABLE IF NOT EXISTS widgets (
    id            UUID         PRIMARY KEY,
    name          TEXT         NOT NULL,
    sdui_spec_id  TEXT         NOT NULL REFERENCES sdui_specs (id) ON DELETE RESTRICT,
    target        JSONB        NOT NULL,
    bindings      JSONB        NOT NULL DEFAULT '{}'::jsonb,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ  NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS widgets_sdui_spec_id_idx ON widgets (sdui_spec_id);

-- 実行ログ・ワークフロー実行履歴は P3 で追加。
-- (workflow_executions, ai_sessions など)
