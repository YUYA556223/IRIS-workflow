#!/usr/bin/env node
// End-to-end smoke test for the workflow engine.
//
// 1. Register a device with notification capability
// 2. Open a WebSocket and wait for delivery events
// 3. Run the "hello-world" workflow
// 4. Verify the AI text arrives over WS as a notification

import WebSocket from "ws";

const BASE = "http://127.0.0.1:8787";

async function http(method, path, body) {
  const res = await fetch(`${BASE}${path}`, {
    method,
    headers: { "Content-Type": "application/json" },
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  let parsed;
  try {
    parsed = JSON.parse(text);
  } catch {
    parsed = text;
  }
  return { status: res.status, body: parsed };
}

async function main() {
  // 1. List workflows (should include hello-world)
  const list = await http("GET", "/workflows");
  console.log("workflows:", list.body.map((w) => w.id));
  if (!list.body.some((w) => w.id === "hello-world")) {
    throw new Error("hello-world workflow not loaded — set IRIS_WORKFLOWS_DIR");
  }

  // 2. Register a device
  const dev = await http("POST", "/devices", {
    kind: "ios",
    name: "wf-test-phone",
    capabilities: ["notification"],
  });
  console.log("device:", dev.body.id);

  // 3. Connect WS and wait for events
  const ws = new WebSocket("ws://127.0.0.1:8787/ws");
  const received = [];
  ws.on("message", (data) => {
    const ev = JSON.parse(data.toString());
    received.push(ev);
    console.log("ws recv:", ev.type, "→", ev.title || ev.result || ev.body || "(no text)");
  });
  await new Promise((res, rej) => {
    ws.once("open", res);
    ws.once("error", rej);
  });
  ws.send(JSON.stringify({ type: "hello", device_id: dev.body.id }));
  await new Promise((r) => setTimeout(r, 200));

  // 4. Run the workflow
  console.log("\nrunning hello-world workflow...");
  const run = await http("POST", "/workflows/hello-world/run", {});
  console.log("execution status:", run.body.status);
  console.log("nodes:");
  for (const n of run.body.nodes) {
    const o = n.output || {};
    const summary = o.text || o.receivers !== undefined ? `(${JSON.stringify(o).slice(0, 80)})` : "";
    console.log(`  - ${n.node_id} [${n.kind}] ${n.status} ${summary}`);
    if (n.error) console.log("     error:", n.error);
  }

  // 5. Wait a moment for WS events
  await new Promise((r) => setTimeout(r, 500));
  ws.close();

  // 6. Assertions
  const notif = received.find((e) => e.type === "notification-delivered");
  if (!notif) {
    console.error("FAIL: no notification-delivered event over WS");
    process.exit(1);
  }
  if (!notif.body || !notif.body.includes("IRIS-workflow")) {
    console.error("FAIL: notification body did not contain expected text:", notif.body);
    process.exit(1);
  }
  console.log("\nOK: workflow execution propagated to WS notification");
  console.log("    notification body:", notif.body);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
