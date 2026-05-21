// Shared helpers for IRIS-workflow E2E test scripts (`scripts/test-*.mjs`).
//
// - `BASE`              : host-backend base URL (override with IRIS_BASE_URL env)
// - `http(method, path, body)` : fetch wrapper that JSON-parses (handles 204)
// - `assert(cond, msg)` : fail-fast helper

export const BASE = process.env.IRIS_BASE_URL ?? "http://127.0.0.1:8787";

export async function http(method, path, body) {
  const res = await fetch(`${BASE}${path}`, {
    method,
    headers: { "Content-Type": "application/json" },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  if (res.status === 204) return { status: 204, body: null };
  return { status: res.status, body: await res.json() };
}

export function assert(cond, msg) {
  if (!cond) {
    console.error("FAIL:", msg);
    process.exit(1);
  }
}
