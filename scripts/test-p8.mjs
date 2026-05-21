#!/usr/bin/env node
// E2E for P8: MQTT trigger + mqtt-publish action.
//
// Requires:
//   - IRIS_MQTT_BROKER=tcp://127.0.0.1:1883 when starting host-backend
//   - mosquitto running on 1883 (docker compose -f infra/docker/docker-compose.yml up -d mosquitto)
//   - Node `mqtt` package (will install on demand)

import { http, assert } from "./_helpers.mjs";

async function main() {
  let mqtt;
  try {
    mqtt = (await import("mqtt")).default;
  } catch {
    console.error("FAIL: 'mqtt' npm package not installed. Run: pnpm add -D -w mqtt");
    process.exit(1);
  }

  // 1. MQTT trigger: workflow listens on a topic, we publish via mqtt.js
  console.log("=== 1. MQTT trigger ===");
  const wfMqttTrig = {
    id: "p8-mqtt-trigger",
    name: "MQTT Trigger",
    trigger: { type: "mqtt", topic: "iris/test/+" },
    nodes: [
      {
        id: "echo",
        type: "transform",
        using: "builtin/pass-through",
        with: {
          data: {
            topic: "{{ trigger.topic }}",
            payload: "{{ trigger.payload }}",
          },
        },
      },
    ],
    edges: [],
  };
  await http("POST", "/workflows", wfMqttTrig);
  console.log("  workflow registered (mqtt trigger should subscribe)");
  await new Promise((r) => setTimeout(r, 500));

  const client = mqtt.connect("mqtt://127.0.0.1:1883");
  await new Promise((res, rej) => {
    client.once("connect", res);
    client.once("error", rej);
  });
  console.log("  mqtt.js connected");

  // Publish a few messages
  client.publish("iris/test/door", "open", { qos: 0 });
  await new Promise((r) => setTimeout(r, 300));
  client.publish("iris/test/light", "on", { qos: 0 });
  await new Promise((r) => setTimeout(r, 800));

  const execs = await http(
    "GET",
    "/workflows/p8-mqtt-trigger/executions?limit=10",
  );
  console.log("  triggered executions:", execs.body.length);
  assert(execs.body.length >= 2, "should have 2 mqtt-triggered executions");
  const topics = execs.body.map((e) => e.nodes[0].output.topic).sort();
  console.log("  topics:", topics);
  assert(
    topics.includes("iris/test/door") && topics.includes("iris/test/light"),
    "both topic publishes should fire",
  );

  // 2. mqtt-publish action: workflow publishes, mqtt.js subscriber catches it
  console.log("\n=== 2. mqtt-publish action ===");
  const received = [];
  await new Promise((res, rej) => {
    client.subscribe("iris/out/#", { qos: 0 }, (err) => (err ? rej(err) : res()));
  });
  client.on("message", (topic, payload) => {
    if (topic.startsWith("iris/out/")) {
      received.push({ topic, payload: payload.toString() });
    }
  });

  const wfMqttPub = {
    id: "p8-mqtt-publish",
    name: "MQTT Publish",
    trigger: { type: "manual" },
    nodes: [
      {
        id: "send",
        type: "action",
        using: "builtin/mqtt-publish",
        with: {
          topic: "iris/out/greeting",
          payload: "hello from workflow",
        },
      },
    ],
    edges: [],
  };
  await http("POST", "/workflows", wfMqttPub);
  const r = await http("POST", "/workflows/p8-mqtt-publish/run", {});
  console.log("  run status:", r.body.status);
  console.log("  send output:", r.body.nodes[0].output);
  await new Promise((r2) => setTimeout(r2, 500));
  console.log("  mqtt.js received:", received);
  assert(received.length >= 1, "subscriber should have received the published message");
  assert(received[0].payload === "hello from workflow", "payload round-trip");

  // Cleanup
  client.end();
  await http("DELETE", "/workflows/p8-mqtt-trigger");
  await http("DELETE", "/workflows/p8-mqtt-publish");

  console.log("\nOK: all P8 checks passed");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
