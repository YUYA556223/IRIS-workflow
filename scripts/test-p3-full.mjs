#!/usr/bin/env node
// E2E test for P3.1 (triggers), P3.2 (execution history), P3.3 (parallel nodes).
//
// Prerequisites:
//   - host-backend running on 127.0.0.1:8787
//   - Postgres (DATABASE_URL) configured for execution history persistence
//   - IRIS_WORKFLOWS_DIR set (so example workflows load)
//
// What it tests:
//   1. Webhook trigger: POST /workflows then POST /hooks/<path> → execution recorded
//   2. Parallel execution: workflow with 3 independent nodes — verify started_at
//      timestamps are close (within 200ms) indicating concurrent spawn
//   3. Execution history API: GET /executions, /executions/:id, /workflows/:id/executions
//   4. Persistence: list returns the executions we just ran

const BASE = "http://127.0.0.1:8787";

async function http(method, path, body) {
  const res = await fetch(`${BASE}${path}`, {
    method,
    headers: { "Content-Type": "application/json" },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  let parsed;
  try {
    parsed = JSON.parse(text);
  } catch {
    parsed = text;
  }
  if (!res.ok) {
    throw new Error(`HTTP ${res.status} ${method} ${path}: ${text}`);
  }
  return { status: res.status, body: parsed };
}

function assert(cond, msg) {
  if (!cond) {
    console.error("FAIL:", msg);
    process.exit(1);
  }
}

async function main() {
  // ============ 1. Webhook trigger ============
  console.log("=== 1. Webhook trigger ===");

  const wfWebhook = {
    id: "p3-webhook-test",
    name: "Webhook Test",
    trigger: { type: "webhook", path: "p3-hook" },
    nodes: [
      {
        id: "echo",
        type: "transform",
        using: "builtin/pass-through",
        with: { data: { received: "{{ trigger.message }}" } },
      },
    ],
    edges: [],
  };
  await http("POST", "/workflows", wfWebhook);
  console.log("  workflow upserted (triggers should sync)");

  // POST to the webhook path → workflow should run with body as trigger_data
  const webhookRes = await http("POST", "/hooks/p3-hook", {
    message: "hello via webhook",
  });
  console.log("  webhook→exec status:", webhookRes.body.status);
  console.log("  echo output:", JSON.stringify(webhookRes.body.nodes[0].output));
  assert(webhookRes.body.status === "success", "webhook execution must succeed");
  assert(
    webhookRes.body.nodes[0].output.received === "hello via webhook",
    "echo must reflect trigger.message via template",
  );

  // ============ 2. Parallel execution ============
  console.log("\n=== 2. Parallel node execution ===");

  // 3 independent transform nodes — all in the initial wave, no edges between them.
  const wfParallel = {
    id: "p3-parallel-test",
    name: "Parallel Test",
    trigger: { type: "manual" },
    nodes: [
      { id: "a", type: "transform", using: "builtin/now", with: {} },
      { id: "b", type: "transform", using: "builtin/now", with: {} },
      { id: "c", type: "transform", using: "builtin/now", with: {} },
    ],
    edges: [],
  };
  await http("POST", "/workflows", wfParallel);

  const parRes = await http("POST", "/workflows/p3-parallel-test/run", {});
  console.log("  parallel exec status:", parRes.body.status);

  const starts = parRes.body.nodes.map((n) => new Date(n.started_at).getTime());
  const spread = Math.max(...starts) - Math.min(...starts);
  console.log("  3 node start-time spread:", spread, "ms");
  assert(parRes.body.status === "success", "parallel exec must succeed");
  assert(spread < 200, `nodes should start nearly simultaneously, got ${spread}ms spread`);

  // ============ 3. Execution history API ============
  console.log("\n=== 3. Execution history API ===");

  // GET /executions (all, most-recent first)
  const allExecs = await http("GET", "/executions?limit=10");
  console.log("  total recent executions:", allExecs.body.length);
  assert(allExecs.body.length >= 2, "should have at least 2 executions");

  // GET /executions/:id
  const byId = await http("GET", `/executions/${webhookRes.body.execution_id}`);
  console.log("  fetched by id:", byId.body.workflow_id);
  assert(
    byId.body.execution_id === webhookRes.body.execution_id,
    "fetch by id must round-trip",
  );

  // GET /workflows/:id/executions
  const wfScoped = await http(
    "GET",
    "/workflows/p3-parallel-test/executions",
  );
  console.log("  parallel-test executions:", wfScoped.body.length);
  assert(
    wfScoped.body.length >= 1,
    "workflow-scoped executions endpoint must return our run",
  );

  // ============ 4. Failure propagation ============
  console.log("\n=== 4. Failure propagation (taint downstream) ===");

  const wfFail = {
    id: "p3-fail-test",
    name: "Fail Test",
    trigger: { type: "manual" },
    nodes: [
      // a will fail (unknown action)
      { id: "a", type: "action", using: "builtin/does-not-exist", with: {} },
      // b depends on a, should be skipped
      { id: "b", type: "transform", using: "builtin/now", with: {} },
      // c is independent, should still succeed (parallel-safe!)
      { id: "c", type: "transform", using: "builtin/now", with: {} },
    ],
    edges: [{ from: "a", to: "b" }],
  };
  await http("POST", "/workflows", wfFail);
  const failRes = await http("POST", "/workflows/p3-fail-test/run", {});
  console.log("  status:", failRes.body.status);
  const byNode = Object.fromEntries(
    failRes.body.nodes.map((n) => [n.node_id, n.status]),
  );
  console.log("  node statuses:", byNode);
  assert(failRes.body.status === "failed", "overall must be failed");
  assert(byNode.a === "failed", "node a must be failed");
  assert(byNode.b === "skipped", "downstream b must be skipped");
  assert(byNode.c === "success", "independent c must still succeed");

  // ============ 5. Cron trigger ============
  console.log("\n=== 5. Cron trigger (waits ~6s) ===");
  const cronWf = {
    id: "p3-cron-test",
    name: "Cron Test",
    trigger: { type: "cron", schedule: "*/3 * * * * *" }, // sec min hr dom mon dow → every 3s
    nodes: [{ id: "tick", type: "transform", using: "builtin/now", with: {} }],
    edges: [],
  };
  await http("POST", "/workflows", cronWf);
  console.log("  cron workflow registered (every 3s)");
  await new Promise((r) => setTimeout(r, 6500));
  const cronExecs = await http(
    "GET",
    "/workflows/p3-cron-test/executions",
  );
  console.log("  cron-fired executions in 6.5s:", cronExecs.body.length);
  assert(cronExecs.body.length >= 1, "cron should have fired at least once");
  for (const e of cronExecs.body.slice(0, 3)) {
    console.log("    -", e.execution_id, "trigger:", e.trigger_data.trigger);
  }

  // ============ 6. FS-watch trigger ============
  console.log("\n=== 6. FS-watch trigger ===");
  const os = await import("node:os");
  const fs = await import("node:fs");
  const pathMod = await import("node:path");
  const tmpDir = pathMod.join(os.tmpdir(), `iris-fs-test-${Date.now()}`);
  fs.mkdirSync(tmpDir);
  const fsWf = {
    id: "p3-fs-test",
    name: "FS Test",
    trigger: { type: "fs-watch", path: tmpDir },
    nodes: [
      {
        id: "noted",
        type: "transform",
        using: "builtin/pass-through",
        with: { data: { kind: "{{ trigger.kind }}" } },
      },
    ],
    edges: [],
  };
  await http("POST", "/workflows", fsWf);
  console.log("  fs-watch workflow registered for", tmpDir);
  fs.writeFileSync(pathMod.join(tmpDir, "trigger.txt"), "hello");
  await new Promise((r) => setTimeout(r, 1500));
  const fsExecs = await http("GET", "/workflows/p3-fs-test/executions");
  console.log("  fs-watch fired executions:", fsExecs.body.length);
  assert(
    fsExecs.body.length >= 1,
    "fs-watch should have fired on file create",
  );
  console.log("    sample kind:", fsExecs.body[0].nodes[0].output);

  // ============ Cleanup ============
  await http("DELETE", "/workflows/p3-webhook-test");
  await http("DELETE", "/workflows/p3-parallel-test");
  await http("DELETE", "/workflows/p3-fail-test");
  await http("DELETE", "/workflows/p3-cron-test");
  await http("DELETE", "/workflows/p3-fs-test");
  try {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  } catch {}

  console.log("\nOK: all P3.1-3.3 checks passed");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
