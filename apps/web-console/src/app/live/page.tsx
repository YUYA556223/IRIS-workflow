"use client";

import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { irisClient, IRIS_BASE_URL } from "@/lib/iris";
import { wsUrl, type DeliveryEvent } from "@iris/sdk-ts";
import { PageHeader } from "@/components/PageHeader";

interface LogEntry {
  receivedAt: string;
  event: DeliveryEvent;
}

export default function LivePage() {
  // For the browser to receive WS pushes, it must register itself as a device
  // with at least one capability. The page does that on mount once.
  const [deviceId, setDeviceId] = useState<string | null>(null);
  const [status, setStatus] = useState<"connecting" | "open" | "closed" | "error">(
    "connecting",
  );
  const [log, setLog] = useState<LogEntry[]>([]);
  const wsRef = useRef<WebSocket | null>(null);

  // Existing browser device for this tab (recreated on every load — fine for a console).
  useEffect(() => {
    let mounted = true;
    irisClient
      .registerDevice({
        kind: "browser",
        name: `web-console-${Math.random().toString(36).slice(2, 6)}`,
        capabilities: ["notification", "widget", "sdui"],
      })
      .then((d) => {
        if (mounted) setDeviceId(d.id);
      })
      .catch(() => setStatus("error"));
    return () => {
      mounted = false;
    };
  }, []);

  // Open WS as soon as we have a device id.
  useEffect(() => {
    if (!deviceId) return;
    const url = wsUrl(IRIS_BASE_URL);
    const ws = new WebSocket(url);
    wsRef.current = ws;
    ws.onopen = () => {
      ws.send(JSON.stringify({ type: "hello", device_id: deviceId }));
      setStatus("open");
    };
    ws.onmessage = (msg) => {
      try {
        const ev = JSON.parse(msg.data) as DeliveryEvent;
        setLog((prev) => [
          { receivedAt: new Date().toISOString(), event: ev },
          ...prev,
        ].slice(0, 200));
      } catch {
        /* skip */
      }
    };
    ws.onclose = () => setStatus("closed");
    ws.onerror = () => setStatus("error");
    return () => ws.close();
  }, [deviceId]);

  return (
    <div>
      <PageHeader
        title="Live"
        subtitle="WS で配信されるイベントの流れを確認できます。タブ自体が browser デバイスとして登録されます。"
        actions={
          <span
            className={`text-xs px-2 py-0.5 rounded ${
              status === "open"
                ? "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-300"
                : status === "error"
                  ? "bg-rose-100 text-rose-800 dark:bg-rose-900/40 dark:text-rose-300"
                  : "bg-neutral-200 text-neutral-700 dark:bg-neutral-800 dark:text-neutral-400"
            }`}
          >
            ws: {status}
          </span>
        }
      />
      <div className="px-6">
        <div className="border border-neutral-200 dark:border-neutral-800 rounded-lg overflow-hidden">
          <div className="bg-neutral-100 dark:bg-neutral-800 text-xs px-3 py-2 text-neutral-600 flex justify-between">
            <span>{log.length} events</span>
            <button
              type="button"
              onClick={() => setLog([])}
              className="text-neutral-500 hover:text-neutral-900 dark:hover:text-white"
            >
              clear
            </button>
          </div>
          <ul className="max-h-[70vh] overflow-y-auto divide-y divide-neutral-200 dark:divide-neutral-800">
            {log.map((entry, i) => (
              <li key={i} className="px-3 py-2 text-sm">
                <div className="flex items-center gap-3">
                  <span className="font-mono text-[10px] text-neutral-500 shrink-0 w-20">
                    {new Date(entry.receivedAt).toLocaleTimeString()}
                  </span>
                  <span className="font-semibold text-[12px]">
                    {entry.event.type}
                  </span>
                  {"title" in entry.event && (
                    <span className="text-neutral-500">— {entry.event.title}</span>
                  )}
                  {"body" in entry.event && (
                    <span className="text-neutral-400 truncate">
                      {entry.event.body}
                    </span>
                  )}
                </div>
                <pre className="mt-1 ml-23 text-[10px] text-neutral-500 overflow-x-auto">
                  {JSON.stringify(entry.event, null, 2)}
                </pre>
              </li>
            ))}
            {log.length === 0 && (
              <li className="px-3 py-6 text-center text-neutral-500 text-sm">
                Waiting for events…
              </li>
            )}
          </ul>
        </div>
      </div>
    </div>
  );
}
