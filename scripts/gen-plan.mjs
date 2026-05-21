#!/usr/bin/env node
// Generate docs/plans/implementation-plan.xlsx (4 sheets: Phases, Tasks, Risks, Milestones)
//
// Usage:
//   node scripts/gen-plan.mjs
//
// Requirements:
//   pnpm install   # exceljs を含むルート devDependencies を入れておく

import ExcelJS from "exceljs";
import path from "node:path";
import url from "node:url";
import fs from "node:fs/promises";

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, "..");
const OUT = path.join(ROOT, "docs", "plans", "implementation-plan.xlsx");

// ---------- Data ----------

const phases = [
  { phase: "P0", goal: "リポジトリ骨格 + ドキュメント + 計画書", duration: "1-2 日", exit: "構造確認、cargo check / pnpm install 成功" },
  { phase: "P1", goal: "Host Backend MVP (Axum + PostgreSQL + 最小 API)", duration: "3-5 日", exit: "curl で /health, /workflows が応答" },
  { phase: "P2", goal: "Claude Code Bridge", duration: "5-7 日", exit: "ClaudeProcessHandle で単発プロンプト→応答取得" },
  { phase: "P3", goal: "Workflow Engine MVP (cron→ai→notify)", duration: "7-10 日", exit: "YAML 1本 end-to-end 実行" },
  { phase: "P4", goal: "Web Console MVP", duration: "5-7 日", exit: "ブラウザでワークフロー操作可能" },
  { phase: "P5", goal: "Flutter Mobile MVP (iOS)", duration: "7-10 日", exit: "実機で通知受信、ホーム画面に静的Widget" },
  { phase: "P6", goal: "VPN + Multi-device", duration: "3-5 日", exit: "iPhone 4G 経由でホストに到達" },
  { phase: "P7", goal: "Server-driven UI", duration: "10-14 日", exit: "ワークフロー結果で Widget が動的更新" },
  { phase: "P8", goal: "IoT (MQTT)", duration: "5-7 日", exit: "スマート照明をワークフローから制御" },
  { phase: "P9", goal: "Packaging (MSIX + リリースビルド)", duration: "5-7 日", exit: "インストーラ配布形式が完成" },
];

const tasks = [
  // P0
  { id: "T-001", phase: "P0", component: "repo", task: "ディレクトリ構成決定 + ドキュメント雛形作成", owner: "you", estimate_d: 0.5, depends: "", status: "in-progress", notes: "本作業" },
  { id: "T-002", phase: "P0", component: "repo", task: "git init + .gitignore", owner: "you", estimate_d: 0.1, depends: "", status: "done", notes: "" },
  { id: "T-003", phase: "P0", component: "repo", task: "Cargo / pnpm / Flutter ワークスペース初期化", owner: "you", estimate_d: 0.5, depends: "T-002", status: "in-progress", notes: "" },
  { id: "T-004", phase: "P0", component: "docs", task: "コンセプト文書 8本", owner: "you", estimate_d: 1, depends: "T-001", status: "in-progress", notes: "concept/00-07" },
  { id: "T-005", phase: "P0", component: "docs", task: "drawio インフラ図 + シーケンス", owner: "you", estimate_d: 0.5, depends: "T-001", status: "in-progress", notes: "" },
  { id: "T-006", phase: "P0", component: "docs", task: "Excel 実装計画 (本ファイル)", owner: "you", estimate_d: 0.5, depends: "T-001", status: "in-progress", notes: "" },

  // P1 — Host Backend MVP
  { id: "T-101", phase: "P1", component: "host-backend", task: "Axum hello-world + tracing + config", owner: "you", estimate_d: 0.5, depends: "T-003", status: "todo", notes: "" },
  { id: "T-102", phase: "P1", component: "host-backend", task: "sqlx + PostgreSQL マイグレーション (workflows, executions テーブル, JSONB)", owner: "you", estimate_d: 1, depends: "T-101", status: "todo", notes: "Docker Compose で開発用 postgres を起動" },
  { id: "T-103", phase: "P1", component: "host-backend", task: "/health, /workflows REST エンドポイント", owner: "you", estimate_d: 1, depends: "T-102", status: "todo", notes: "" },
  { id: "T-104", phase: "P1", component: "host-backend", task: "OpenAPI 雛形 (utoipa)", owner: "you", estimate_d: 0.5, depends: "T-103", status: "todo", notes: "" },
  { id: "T-105", phase: "P1", component: "host-backend", task: "WebSocket セッション (echo MVP)", owner: "you", estimate_d: 1, depends: "T-101", status: "todo", notes: "" },

  // P2 — Claude Code Bridge
  { id: "T-201", phase: "P2", component: "ai-bridge", task: "ClaudeProcessHandle 設計 (spawn / stdin / stdout / cancel)", owner: "you", estimate_d: 1, depends: "T-101", status: "todo", notes: "" },
  { id: "T-202", phase: "P2", component: "ai-bridge", task: "stream-json パーサ (NDJSON → enum)", owner: "you", estimate_d: 1, depends: "T-201", status: "todo", notes: "" },
  { id: "T-203", phase: "P2", component: "ai-bridge", task: "セッション管理 (--session-id, --resume)", owner: "you", estimate_d: 1, depends: "T-202", status: "todo", notes: "" },
  { id: "T-204", phase: "P2", component: "ai-bridge", task: "Semaphore + 同時実行制御", owner: "you", estimate_d: 0.5, depends: "T-201", status: "todo", notes: "" },
  { id: "T-205", phase: "P2", component: "ai-bridge", task: "MCP permission server (rmcp 統合)", owner: "you", estimate_d: 2, depends: "T-203", status: "todo", notes: "" },
  { id: "T-206", phase: "P2", component: "ai-bridge", task: "タイムアウト / kill cross-platform 対応", owner: "you", estimate_d: 1, depends: "T-201", status: "todo", notes: "Windows + Unix" },

  // P3 — Workflow Engine MVP
  { id: "T-301", phase: "P3", component: "workflow", task: "YAML DSL パーサ + JSONSchema", owner: "you", estimate_d: 1, depends: "T-103", status: "todo", notes: "" },
  { id: "T-302", phase: "P3", component: "workflow", task: "DAG ビルダ + トポロジカルソート", owner: "you", estimate_d: 1, depends: "T-301", status: "todo", notes: "" },
  { id: "T-303", phase: "P3", component: "workflow", task: "ノード実行 (ai / transform / action)", owner: "you", estimate_d: 2, depends: "T-302", status: "todo", notes: "" },
  { id: "T-304", phase: "P3", component: "workflow", task: "変数バインディング + テンプレート", owner: "you", estimate_d: 1, depends: "T-303", status: "todo", notes: "tinytemplate" },
  { id: "T-305", phase: "P3", component: "workflow", task: "Trigger Hub (cron / webhook)", owner: "you", estimate_d: 1, depends: "T-303", status: "todo", notes: "" },
  { id: "T-306", phase: "P3", component: "workflow", task: "実行ログ + 失敗時リトライ", owner: "you", estimate_d: 1, depends: "T-303", status: "todo", notes: "" },
  { id: "T-307", phase: "P3", component: "workflow", task: "End-to-End sample workflow", owner: "you", estimate_d: 0.5, depends: "T-306", status: "todo", notes: "morning-briefing" },

  // P4 — Web Console MVP
  { id: "T-401", phase: "P4", component: "web-console", task: "Next.js shadcn/ui セットアップ", owner: "you", estimate_d: 0.5, depends: "T-103", status: "todo", notes: "" },
  { id: "T-402", phase: "P4", component: "web-console", task: "ワークフロー一覧 / 実行ログ画面", owner: "you", estimate_d: 1, depends: "T-401", status: "todo", notes: "" },
  { id: "T-403", phase: "P4", component: "web-console", task: "Monaco エディタ + JSONSchema 連携", owner: "you", estimate_d: 1, depends: "T-402", status: "todo", notes: "" },
  { id: "T-404", phase: "P4", component: "sdk-ts", task: "sdk-ts (REST + WS) 自動生成", owner: "you", estimate_d: 1, depends: "T-104", status: "todo", notes: "openapi-typescript" },

  // P5 — Flutter Mobile MVP
  { id: "T-501", phase: "P5", component: "mobile", task: "Flutter iOS 初期化 + Riverpod scaffold", owner: "you", estimate_d: 0.5, depends: "T-003", status: "todo", notes: "" },
  { id: "T-502", phase: "P5", component: "mobile", task: "sdk-dart + Dio セットアップ", owner: "you", estimate_d: 0.5, depends: "T-104", status: "todo", notes: "" },
  { id: "T-503", phase: "P5", component: "mobile", task: "APNs 通知受信 (Apple Push)", owner: "you", estimate_d: 2, depends: "T-501", status: "todo", notes: "Apple Developer 契約要" },
  { id: "T-504", phase: "P5", component: "mobile", task: "Swift WidgetKit ターゲット追加 (静的 widget)", owner: "you", estimate_d: 2, depends: "T-501", status: "todo", notes: "App Group 設定" },
  { id: "T-505", phase: "P5", component: "mobile", task: "Platform Channel でアプリ↔ Widget データ共有", owner: "you", estimate_d: 1, depends: "T-504", status: "todo", notes: "" },

  // P6 — VPN
  { id: "T-601", phase: "P6", component: "infra", task: "Tailscale アカウント + tag 設計", owner: "you", estimate_d: 0.5, depends: "", status: "todo", notes: "" },
  { id: "T-602", phase: "P6", component: "infra", task: "ホストPC + iPhone を tailnet 参加", owner: "you", estimate_d: 0.5, depends: "T-601", status: "todo", notes: "" },
  { id: "T-603", phase: "P6", component: "infra", task: "ACL JSON (iris-host / iris-device)", owner: "you", estimate_d: 0.5, depends: "T-602", status: "todo", notes: "" },
  { id: "T-604", phase: "P6", component: "host-backend", task: "デバイスペアリングフロー (/pair/init)", owner: "you", estimate_d: 1, depends: "T-103", status: "todo", notes: "6桁コード認証" },

  // P7 — SDUI
  { id: "T-701", phase: "P7", component: "sdui", task: "JSONSchema 確定 (packages/proto/sdui.schema.json)", owner: "you", estimate_d: 1, depends: "T-301", status: "todo", notes: "" },
  { id: "T-702", phase: "P7", component: "host-backend", task: "SDUI 生成サービス + WS 配信 (JSON Patch)", owner: "you", estimate_d: 2, depends: "T-701", status: "todo", notes: "" },
  { id: "T-703", phase: "P7", component: "sdui-renderer-flutter", task: "Flutter レンダラ (10コンポーネント)", owner: "you", estimate_d: 4, depends: "T-701", status: "todo", notes: "VStack/HStack/Text/Button..." },
  { id: "T-704", phase: "P7", component: "mobile", task: "WidgetKit 用 SDUI サブセット renderer", owner: "you", estimate_d: 2, depends: "T-504", status: "todo", notes: "" },
  { id: "T-705", phase: "P7", component: "web-console", task: "SDUI プレビュー画面 (React)", owner: "you", estimate_d: 1, depends: "T-401", status: "todo", notes: "" },

  // P8 — IoT
  { id: "T-801", phase: "P8", component: "infra", task: "MQTT broker (mosquitto Docker / rumqttd)", owner: "you", estimate_d: 0.5, depends: "", status: "todo", notes: "" },
  { id: "T-802", phase: "P8", component: "host-backend", task: "MQTT クライアント + Device Gateway 拡張", owner: "you", estimate_d: 1, depends: "T-103", status: "todo", notes: "" },
  { id: "T-803", phase: "P8", component: "iot", task: "サンプル ESP32 デバイスドライバ", owner: "you", estimate_d: 2, depends: "T-802", status: "todo", notes: "照明制御 sample" },

  // P9 — Packaging
  { id: "T-901", phase: "P9", component: "packaging", task: "Windows MSIX パッケージング", owner: "you", estimate_d: 2, depends: "T-103", status: "todo", notes: "署名証明書" },
  { id: "T-902", phase: "P9", component: "packaging", task: "Flutter iOS / Windows リリースビルド", owner: "you", estimate_d: 1, depends: "T-501", status: "todo", notes: "" },
  { id: "T-903", phase: "P9", component: "packaging", task: "自動アップデート機構 (Sparkle 互換 or 自前)", owner: "you", estimate_d: 2, depends: "T-901", status: "todo", notes: "" },
];

const risks = [
  { id: "R-01", category: "AI 依存", risk: "claude CLI の仕様変更でブリッジが壊れる", likelihood: "Medium", impact: "High", mitigation: "stream-json を抽象化レイヤで吸収、CI で互換テスト、CLI バージョン固定オプション" },
  { id: "R-02", category: "ホスト可用性", risk: "ホストPCがスリープ/オフラインだと全機能停止", likelihood: "High", impact: "Medium", mitigation: "Wake-on-LAN / スマートプラグでの自動電源管理、デバイス側オフラインキャッシュ、SLA を明示" },
  { id: "R-03", category: "iOS 制約", risk: "WidgetKit の更新頻度・インタラクション制約", likelihood: "High", impact: "Medium", mitigation: "Push 起動 + Live Activities 検討、リアルタイム性が必要ならアプリ内で対応" },
  { id: "R-04", category: "セキュリティ", risk: "Claude Code が任意ファイル読み書き可能", likelihood: "Medium", impact: "High", mitigation: "--permission-mode plan デフォルト、--add-dir で sandbox 限定、permission-prompt-tool でデバイス承認" },
  { id: "R-05", category: "コスト", risk: "Claude のトークン消費が想定超過", likelihood: "Medium", impact: "Medium", mitigation: "total_cost_usd 監視 + 閾値打ち切り、軽量ノードは Haiku モデル指定、ワークフロー単位の上限" },
  { id: "R-06", category: "Apple", risk: "Apple Developer 契約・APNs 設定の手間", likelihood: "High", impact: "Low", mitigation: "P5 の早い段階で契約、開発期間にバッファ" },
  { id: "R-07", category: "VPN", risk: "Tailscale 障害時に全デバイスが疎通不能", likelihood: "Low", impact: "Medium", mitigation: "WireGuard 自前構成への切替手順を docs に常備" },
  { id: "R-08", category: "スコープ", risk: "機能拡大でリリースが遅延", likelihood: "High", impact: "Medium", mitigation: "Phase ごとに Exit 基準を厳格運用、追加機能は別 Phase へ" },
];

const milestones = [
  { m: "M1", name: "Personal Daily Workflow Live", target: "P3 完了時 (約 3-4 週)", deliverables: "毎日 1 本の自動ワークフローが自分のホストで動く、結果が通知される" },
  { m: "M2", name: "Multi-device + iPhone Widget", target: "P6-P7 完了時 (約 8-10 週)", deliverables: "iPhone ホーム画面ウィジェットがホストPCの SDUI で動的更新される" },
  { m: "M3", name: "IoT Integration", target: "P8 完了時 (約 11-12 週)", deliverables: "ワークフローからスマート照明・スマートプラグを制御できる" },
  { m: "M4", name: "Shippable Package", target: "P9 完了時 (約 13-14 週)", deliverables: "Windows MSIX + Flutter リリースビルド、家族/知人への配布可能" },
];

// ---------- Excel ----------

const wb = new ExcelJS.Workbook();
wb.creator = "IRIS-workflow";
wb.created = new Date();
wb.title = "IRIS-workflow Implementation Plan";

// Phases sheet
{
  const ws = wb.addWorksheet("Phases", { views: [{ state: "frozen", ySplit: 1 }] });
  ws.columns = [
    { header: "Phase", key: "phase", width: 8 },
    { header: "Goal", key: "goal", width: 50 },
    { header: "Duration", key: "duration", width: 14 },
    { header: "Exit Criteria", key: "exit", width: 60 },
  ];
  phases.forEach(p => ws.addRow(p));
  styleHeader(ws);
}

// Tasks sheet
{
  const ws = wb.addWorksheet("Tasks", { views: [{ state: "frozen", ySplit: 1 }] });
  ws.columns = [
    { header: "ID", key: "id", width: 8 },
    { header: "Phase", key: "phase", width: 8 },
    { header: "Component", key: "component", width: 18 },
    { header: "Task", key: "task", width: 55 },
    { header: "Owner", key: "owner", width: 12 },
    { header: "Estimate (d)", key: "estimate_d", width: 14 },
    { header: "Depends", key: "depends", width: 14 },
    { header: "Status", key: "status", width: 14 },
    { header: "Notes", key: "notes", width: 40 },
  ];
  tasks.forEach(t => ws.addRow(t));
  styleHeader(ws);

  // Conditional fill for status
  ws.getColumn("status").eachCell({ includeEmpty: false }, (cell, rowNumber) => {
    if (rowNumber === 1) return;
    const v = String(cell.value || "").toLowerCase();
    const color = v === "done" ? "C6EFCE" : v === "in-progress" ? "FFEB9C" : v === "blocked" ? "FFC7CE" : "F2F2F2";
    cell.fill = { type: "pattern", pattern: "solid", fgColor: { argb: color } };
  });
}

// Risks sheet
{
  const ws = wb.addWorksheet("Risks", { views: [{ state: "frozen", ySplit: 1 }] });
  ws.columns = [
    { header: "ID", key: "id", width: 8 },
    { header: "Category", key: "category", width: 16 },
    { header: "Risk", key: "risk", width: 60 },
    { header: "Likelihood", key: "likelihood", width: 12 },
    { header: "Impact", key: "impact", width: 10 },
    { header: "Mitigation", key: "mitigation", width: 70 },
  ];
  risks.forEach(r => ws.addRow(r));
  styleHeader(ws);
}

// Milestones sheet
{
  const ws = wb.addWorksheet("Milestones", { views: [{ state: "frozen", ySplit: 1 }] });
  ws.columns = [
    { header: "M#", key: "m", width: 6 },
    { header: "Name", key: "name", width: 36 },
    { header: "Target", key: "target", width: 32 },
    { header: "Deliverables", key: "deliverables", width: 70 },
  ];
  milestones.forEach(m => ws.addRow(m));
  styleHeader(ws);
}

await fs.mkdir(path.dirname(OUT), { recursive: true });
await wb.xlsx.writeFile(OUT);
console.log(`OK: wrote ${path.relative(ROOT, OUT)}`);

// ---------- helpers ----------
function styleHeader(ws) {
  const header = ws.getRow(1);
  header.font = { bold: true, color: { argb: "FFFFFFFF" } };
  header.fill = { type: "pattern", pattern: "solid", fgColor: { argb: "FF1F2937" } };
  header.height = 22;
  header.alignment = { vertical: "middle", horizontal: "left" };
  header.eachCell(cell => {
    cell.border = {
      bottom: { style: "thin", color: { argb: "FF000000" } },
    };
  });
  // wrap text in body
  ws.eachRow({ includeEmpty: false }, row => {
    row.alignment = { ...row.alignment, vertical: "middle", wrapText: true };
  });
}
