#!/usr/bin/env node
// E2E for P3.4: when / retry / secrets / sub-workflow.

const BASE = "http://127.0.0.1:8787";

async function http(method, path, body) {
  const res = await fetch(`${BASE}${path}`, {
    method,
    headers: { "Content-Type": "application/json" },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  if (res.status === 204) return { status: 204, body: null };
  return { status: res.status, body: await res.json() };
}

function assert(cond, msg) {
  if (!cond) {
    console.error("FAIL:", msg);
    process.exit(1);
  }
}

async function main() {
  // ===== 1. node.when conditional =====
  console.log("=== 1. node.when conditional ===");
  const wfWhen = {
    id: "p3-4-when-test",
    name: "When Test",
    trigger: { type: "manual" },
    nodes: [
      // Branch into two sub-nodes; only one runs based on trigger.mode
      {
        id: "high_priority",
        type: "transform",
        using: "builtin/pass-through",
        when: '{{ trigger.mode }}',
        with: { data: "ran high" },
      },
      {
        id: "skipped",
        type: "transform",
        using: "builtin/pass-through",
        when: "false",
        with: { data: "should not run" },
      },
      // This node depends on both above, but neither failed → it should run.
      {
        id: "downstream",
        type: "transform",
        using: "builtin/pass-through",
        with: { data: "always" },
      },
    ],
    edges: [
      { from: "high_priority", to: "downstream" },
      { from: "skipped", to: "downstream" },
    ],
  };
  await http("POST", "/workflows", wfWhen);
  const r1 = await http("POST", "/workflows/p3-4-when-test/run", { mode: "yes" });
  const status = Object.fromEntries(r1.body.nodes.map((n) => [n.node_id, n.status]));
  console.log("  node statuses:", status);
  assert(status.high_priority === "success", "high_priority should run when mode=yes");
  assert(status.skipped === "skipped", "skipped should be Skipped (when=false)");
  assert(status.downstream === "success", "downstream should run (when=false isn't a failure)");

  // ===== 2. Retry =====
  console.log("\n=== 2. Retry policy ===");
  const wfRetry = {
    id: "p3-4-retry-test",
    name: "Retry Test",
    trigger: { type: "manual" },
    nodes: [
      {
        id: "flaky",
        type: "action",
        using: "builtin/fail",
        retry: { max_attempts: 3, delay_ms: 50, backoff: "exponential" },
        with: { reason: "boom" },
      },
    ],
    edges: [],
  };
  await http("POST", "/workflows", wfRetry);
  const r2 = await http("POST", "/workflows/p3-4-retry-test/run", {});
  const flaky = r2.body.nodes[0];
  console.log("  attempts:", flaky.attempts);
  console.log("  status:", flaky.status);
  assert(flaky.status === "failed", "intentional fail should mark Failed");
  assert(flaky.attempts === 3, `expected 3 attempts, got ${flaky.attempts}`);

  // ===== 3. Secrets =====
  console.log("\n=== 3. Secret resolution ===");
  // Note: IRIS_SECRET_P3_4_DEMO must be set in env BEFORE host-backend starts
  const wfSecret = {
    id: "p3-4-secret-test",
    name: "Secret Test",
    trigger: { type: "manual" },
    nodes: [
      {
        id: "use_secret",
        type: "transform",
        using: "builtin/pass-through",
        with: { data: { token: "{{ secrets.P3_4_DEMO }}" } },
      },
    ],
    edges: [],
  };
  await http("POST", "/workflows", wfSecret);
  const r3 = await http("POST", "/workflows/p3-4-secret-test/run", {});
  const tok = r3.body.nodes[0].output.token;
  console.log("  resolved token:", tok);
  assert(tok === "from-env", `expected 'from-env', got ${JSON.stringify(tok)}`);

  // ===== 4. Sub-workflow =====
  console.log("\n=== 4. Sub-workflow ===");
  await http("POST", "/workflows", {
    id: "p3-4-inner",
    name: "Inner",
    trigger: { type: "manual" },
    nodes: [
      {
        id: "echo",
        type: "transform",
        using: "builtin/pass-through",
        with: { data: { from: "inner", payload: "{{ trigger.payload }}" } },
      },
    ],
    edges: [],
  });
  await http("POST", "/workflows", {
    id: "p3-4-outer",
    name: "Outer",
    trigger: { type: "manual" },
    nodes: [
      {
        id: "nested",
        type: "workflow",
        with: {
          workflow_id: "p3-4-inner",
          trigger_data: { payload: "from outer" },
        },
      },
      {
        id: "consume",
        type: "transform",
        using: "builtin/pass-through",
        with: {
          data: {
            inner_status: "{{ nested.status }}",
            inner_payload: "{{ nested.nodes.0.output.payload }}",
          },
        },
      },
    ],
    edges: [{ from: "nested", to: "consume" }],
  });
  const r4 = await http("POST", "/workflows/p3-4-outer/run", {});
  console.log("  outer status:", r4.body.status);
  const consume = r4.body.nodes.find((n) => n.node_id === "consume");
  console.log("  consume.output:", consume?.output);
  assert(r4.body.status === "success", "outer must succeed");
  assert(consume.output.inner_status === "success", "inner should run successfully");
  assert(
    consume.output.inner_payload === "from outer",
    "outer.trigger should flow into inner via trigger_data",
  );

  // ===== Cleanup =====
  await http("DELETE", "/workflows/p3-4-when-test");
  await http("DELETE", "/workflows/p3-4-retry-test");
  await http("DELETE", "/workflows/p3-4-secret-test");
  await http("DELETE", "/workflows/p3-4-outer");
  await http("DELETE", "/workflows/p3-4-inner");

  console.log("\nOK: all P3.4 checks passed");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
