-- ワークフロー実行履歴。
-- 各実行は1行。nodes は NodeExecution の配列を JSONB で保存。
-- trigger_data は実行を起動したデータ (REST body / cron data / webhook body / fs event)。

CREATE TABLE IF NOT EXISTS workflow_executions (
    id            UUID         PRIMARY KEY,
    workflow_id   TEXT         NOT NULL,
    status        TEXT         NOT NULL,
    trigger_data  JSONB        NOT NULL DEFAULT 'null'::jsonb,
    nodes         JSONB        NOT NULL DEFAULT '[]'::jsonb,
    started_at    TIMESTAMPTZ  NOT NULL,
    finished_at   TIMESTAMPTZ  NOT NULL,
    error         TEXT
);

CREATE INDEX IF NOT EXISTS workflow_executions_workflow_idx ON workflow_executions (workflow_id);
CREATE INDEX IF NOT EXISTS workflow_executions_started_idx ON workflow_executions (started_at DESC);
