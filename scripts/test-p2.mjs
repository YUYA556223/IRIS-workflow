#!/usr/bin/env node
// E2E test for P2.1 (permission flow) + P2.2 (SSE).
//
// 1. SSE streaming: POST /ai/prompt/stream and verify multiple events arrive
// 2. Permission flow: WS subscribe → POST /permission/request → catch the
//    broadcast → POST /permission/respond → verify the request resolves.

import { BASE, http, assert } from "./_helpers.mjs";

async function main() {
  // ===== 1. SSE =====
  console.log("=== 1. SSE /ai/prompt/stream ===");
  const res = await fetch(`${BASE}/ai/prompt/stream`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      prompt: "Reply with exactly: hi sse",
      permission_mode: "plan",
    }),
  });
  assert(res.ok, `SSE HTTP ${res.status}`);
  const ct = res.headers.get("content-type") ?? "";
  assert(ct.startsWith("text/event-stream"), `wrong content-type: ${ct}`);

  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let buf = "";
  const eventTypes = new Set();
  let eventCount = 0;
  let resultText = null;

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buf += decoder.decode(value, { stream: true });
    let idx;
    while ((idx = buf.indexOf("\n\n")) !== -1) {
      const frame = buf.slice(0, idx);
      buf = buf.slice(idx + 2);
      let evType = null;
      let data = "";
      for (const line of frame.split("\n")) {
        if (line.startsWith("event:")) evType = line.slice(6).trim();
        else if (line.startsWith("data:")) data += line.slice(5).trim();
      }
      if (evType) eventTypes.add(evType);
      eventCount += 1;
      if (data) {
        try {
          const obj = JSON.parse(data);
          if (obj.type === "result" && obj.result) resultText = obj.result;
        } catch {
          /* skip */
        }
      }
    }
  }
  console.log("  SSE events:", eventCount, [...eventTypes].join(", "));
  console.log("  Final text:", resultText);
  assert(eventCount >= 2, "expected at least 2 SSE events");
  assert(eventTypes.has("result"), "expected a result event");

  // ===== 2. Permission flow =====
  console.log("\n=== 2. Permission flow (broadcast → respond) ===");
  const WebSocket = (await import("ws")).default;

  // Register a device that listens for permission requests
  const dev = (
    await http("POST", "/devices", {
      kind: "browser",
      name: "p2-permission-listener",
      capabilities: ["notification"],
    })
  ).body;
  console.log("  listener device:", dev.id);

  // Open WS first, register hello, wait for subscription to settle
  const ws = new WebSocket("ws://127.0.0.1:8787/ws");
  await new Promise((res, rej) => {
    ws.once("open", res);
    ws.once("error", rej);
  });
  ws.send(JSON.stringify({ type: "hello", device_id: dev.id }));
  await new Promise((r) => setTimeout(r, 300));

  // Set up the catcher BEFORE we fire the request
  const requestIdPromise = new Promise((resolve, reject) => {
    const t = setTimeout(
      () => reject(new Error("timeout waiting for permission-requested event")),
      6000,
    );
    ws.on("message", (data) => {
      try {
        const ev = JSON.parse(data.toString());
        if (ev.type === "permission-requested") {
          clearTimeout(t);
          resolve(ev.request_id);
        }
      } catch {
        /* skip */
      }
    });
  });

  // Fire the request (will block server-side until we respond)
  const requestPromise = fetch(`${BASE}/permission/request`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      tool_name: "Bash",
      tool_input: { command: "echo hello" },
    }),
  }).then((r) => r.json());

  const requestId = await requestIdPromise;
  console.log("  caught request_id:", requestId);

  // Respond
  const respondRes = await http("POST", "/permission/respond", {
    request_id: requestId,
    behavior: "allow",
    message: "auto-approved for test",
  });
  console.log("  respond status:", respondRes.status, respondRes.body);
  assert(respondRes.body.accepted, "respond must be accepted");

  // The original /permission/request should now resolve
  const result = await requestPromise;
  console.log("  permission resolved:", result);
  assert(result.behavior === "allow", `expected allow, got ${result.behavior}`);
  assert(result.message === "auto-approved for test", "message round-trips");

  ws.close();

  console.log("\nOK: P2.1 + P2.2 checks passed");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
