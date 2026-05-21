#!/usr/bin/env node
// Quick WebSocket smoke test for host-backend.
//
// Usage:
//   1. Start backend: cargo run -p host-backend
//   2. node scripts/test-ws.mjs

import WebSocket from "ws";

const BASE = "http://127.0.0.1:8787";

async function http(method, path, body) {
  const res = await fetch(`${BASE}${path}`, {
    method,
    headers: { "Content-Type": "application/json" },
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  try {
    return { status: res.status, body: JSON.parse(text) };
  } catch {
    return { status: res.status, body: text };
  }
}

async function main() {
  // Pre-create an SDUI spec
  await http("POST", "/sdui-specs", {
    id: "ws-test",
    type: "ComponentTree",
    root: { type: "Text", value: "hello" },
  });

  // Register a device with notification + widget capabilities
  const dev = await http("POST", "/devices", {
    kind: "ios",
    name: "ws-test-phone",
    capabilities: ["widget", "notification", "sdui"],
  });
  console.log("registered device:", dev.body.id);

  // Connect WS
  const ws = new WebSocket("ws://127.0.0.1:8787/ws");
  const received = [];
  ws.on("message", (data) => {
    const ev = JSON.parse(data.toString());
    received.push(ev);
    console.log("recv:", ev.type, ev);
  });

  await new Promise((res, rej) => {
    ws.once("open", res);
    ws.once("error", rej);
    setTimeout(() => rej(new Error("WS open timeout")), 3000);
  });
  console.log("ws connected");

  // Send hello
  ws.send(JSON.stringify({ type: "hello", device_id: dev.body.id }));
  await new Promise((res) => setTimeout(res, 200));

  // Trigger 3 events: widget create, widget patch, notification, ping
  const widget = await http("POST", "/widgets", {
    name: "ws-widget",
    sdui_spec_id: "ws-test",
    target: { type: "all" },
    bindings: { x: 1 },
  });
  console.log("widget created (id=" + widget.body.id + ")");

  await http("PATCH", `/widgets/${widget.body.id}/bindings`, {
    bindings: { x: 42 },
  });

  await http("POST", "/notifications", {
    target: { type: "capability", capability: "notification" },
    title: "WS Test",
    body: "Hello over WS!",
    priority: "high",
  });

  // Wait briefly for messages to arrive
  await new Promise((res) => setTimeout(res, 500));

  ws.close();
  await new Promise((res) => ws.once("close", res));

  console.log("\n=== summary ===");
  console.log("events received:", received.length);
  for (const e of received) {
    console.log("  -", e.type);
  }

  // Assertions
  const types = received.map((e) => e.type);
  const expected = [
    "widget-created",
    "widget-updated",
    "notification-delivered",
  ];
  for (const e of expected) {
    if (!types.includes(e)) {
      console.error(`FAIL: expected '${e}' not received`);
      process.exit(1);
    }
  }
  console.log("\nOK: all expected event types received");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
