"use client";

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import Link from "next/link";
import { irisClient } from "@/lib/iris";
import { PageHeader } from "@/components/PageHeader";
import type { ExecutionResult } from "@iris/sdk-ts";

export default function WorkflowsPage() {
  const qc = useQueryClient();
  const [running, setRunning] = useState<string | null>(null);
  const [lastResult, setLastResult] = useState<ExecutionResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const workflows = useQuery({
    queryKey: ["workflows"],
    queryFn: () => irisClient.listWorkflows(),
    refetchInterval: 10000,
  });

  async function run(id: string) {
    setRunning(id);
    setError(null);
    setLastResult(null);
    try {
      const result = await irisClient.runWorkflow(id, {});
      setLastResult(result);
      qc.invalidateQueries({ queryKey: ["executions"] });
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(null);
    }
  }

  return (
    <div>
      <PageHeader
        title="Workflows"
        subtitle="登録済みワークフロー一覧。実行ボタンで manual トリガを起動できます。"
      />
      <div className="px-6">
        <div className="border border-neutral-200 dark:border-neutral-800 rounded-lg overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-neutral-100 dark:bg-neutral-800 text-left text-xs uppercase text-neutral-600">
              <tr>
                <th className="px-3 py-2">ID</th>
                <th className="px-3 py-2">Name</th>
                <th className="px-3 py-2">Trigger</th>
                <th className="px-3 py-2">Nodes</th>
                <th className="px-3 py-2"></th>
              </tr>
            </thead>
            <tbody>
              {workflows.data?.map((wf) => (
                <tr
                  key={wf.id}
                  className="border-t border-neutral-200 dark:border-neutral-800 hover:bg-neutral-50 dark:hover:bg-neutral-900"
                >
                  <td className="px-3 py-2 font-mono text-[12px]">
                    <Link
                      href={`/workflows/${wf.id}`}
                      className="text-blue-600 dark:text-blue-400 hover:underline"
                    >
                      {wf.id}
                    </Link>
                  </td>
                  <td className="px-3 py-2">{wf.name}</td>
                  <td className="px-3 py-2 font-mono text-[11px] text-neutral-500">
                    {triggerLabel(wf.trigger)}
                  </td>
                  <td className="px-3 py-2 text-neutral-600">
                    {wf.nodes.length}
                  </td>
                  <td className="px-3 py-2 text-right">
                    <button
                      type="button"
                      disabled={running === wf.id}
                      onClick={() => run(wf.id)}
                      className="rounded bg-neutral-900 text-white text-xs px-3 py-1 hover:bg-neutral-700 disabled:opacity-50 dark:bg-white dark:text-neutral-900"
                    >
                      {running === wf.id ? "running…" : "run"}
                    </button>
                  </td>
                </tr>
              ))}
              {workflows.data?.length === 0 && (
                <tr>
                  <td
                    colSpan={5}
                    className="px-3 py-6 text-center text-neutral-500"
                  >
                    No workflows loaded. Set IRIS_WORKFLOWS_DIR or POST a workflow JSON.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

        {error && (
          <div className="mt-4 rounded border border-rose-300 bg-rose-50 dark:bg-rose-950/40 dark:border-rose-900 p-3 text-sm text-rose-700 dark:text-rose-300">
            {error}
          </div>
        )}

        {lastResult && (
          <div className="mt-4 border border-neutral-200 dark:border-neutral-800 rounded-lg p-3 bg-neutral-50/50 dark:bg-neutral-950 text-sm">
            <div className="text-xs uppercase tracking-wider text-neutral-500 mb-1">
              Last result · status:{" "}
              <span
                className={
                  lastResult.status === "success"
                    ? "text-emerald-600"
                    : "text-rose-600"
                }
              >
                {lastResult.status}
              </span>
            </div>
            <pre className="text-[11px] overflow-x-auto whitespace-pre-wrap">
              {JSON.stringify(lastResult, null, 2)}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}

function triggerLabel(t: { type: string; [k: string]: unknown }): string {
  switch (t.type) {
    case "manual":
      return "manual";
    case "cron":
      return `cron(${t.schedule})`;
    case "webhook":
      return `webhook(/hooks/${t.path})`;
    case "fs-watch":
      return `fs(${t.path})`;
    case "mqtt":
      return `mqtt(${t.topic})`;
    default:
      return t.type;
  }
}
