"use client";

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { use, useState } from "react";
import Link from "next/link";
import { irisClient } from "@/lib/iris";
import { PageHeader } from "@/components/PageHeader";
import { StatusBadge } from "@/components/StatusBadge";

export default function WorkflowDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const qc = useQueryClient();
  const [running, setRunning] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const workflow = useQuery({
    queryKey: ["workflow", id],
    queryFn: () => irisClient.getWorkflow(id),
  });
  const executions = useQuery({
    queryKey: ["workflow-executions", id],
    queryFn: () => irisClient.listWorkflowExecutions(id, 20),
    refetchInterval: 5000,
  });

  async function run() {
    setRunning(true);
    setErr(null);
    try {
      await irisClient.runWorkflow(id, {});
      qc.invalidateQueries({ queryKey: ["workflow-executions", id] });
      qc.invalidateQueries({ queryKey: ["executions"] });
    } catch (e) {
      setErr(String(e));
    } finally {
      setRunning(false);
    }
  }

  return (
    <div>
      <PageHeader
        title={workflow.data?.name ?? id}
        subtitle={workflow.data?.description ?? `Workflow id: ${id}`}
        actions={
          <button
            type="button"
            onClick={run}
            disabled={running}
            className="rounded bg-neutral-900 text-white text-xs px-3 py-1.5 hover:bg-neutral-700 disabled:opacity-50 dark:bg-white dark:text-neutral-900"
          >
            {running ? "running…" : "Run now"}
          </button>
        }
      />
      <div className="px-6 grid grid-cols-1 lg:grid-cols-2 gap-6">
        <section>
          <h2 className="text-xs font-semibold text-neutral-500 uppercase tracking-wider mb-2">
            Definition
          </h2>
          <pre className="text-[11px] border border-neutral-200 dark:border-neutral-800 rounded-lg p-3 overflow-x-auto bg-neutral-50 dark:bg-neutral-950">
            {workflow.data
              ? JSON.stringify(workflow.data, null, 2)
              : "loading…"}
          </pre>
        </section>

        <section>
          <h2 className="text-xs font-semibold text-neutral-500 uppercase tracking-wider mb-2">
            Recent executions
          </h2>
          <div className="border border-neutral-200 dark:border-neutral-800 rounded-lg overflow-hidden">
            <table className="w-full text-sm">
              <thead className="bg-neutral-100 dark:bg-neutral-800 text-xs uppercase text-neutral-600">
                <tr>
                  <th className="text-left px-3 py-2">Status</th>
                  <th className="text-left px-3 py-2">Started</th>
                  <th className="text-left px-3 py-2">Trigger</th>
                </tr>
              </thead>
              <tbody>
                {executions.data?.map((e) => {
                  const trig = (e.trigger_data as { trigger?: string })
                    ?.trigger;
                  return (
                    <tr
                      key={e.execution_id}
                      className="border-t border-neutral-200 dark:border-neutral-800 hover:bg-neutral-50 dark:hover:bg-neutral-900"
                    >
                      <td className="px-3 py-2">
                        <Link
                          href={`/executions/${e.execution_id}`}
                          className="text-blue-600 dark:text-blue-400 hover:underline"
                        >
                          <StatusBadge status={e.status} />
                        </Link>
                      </td>
                      <td className="px-3 py-2 text-neutral-500 text-[12px]">
                        {new Date(e.started_at).toLocaleString()}
                      </td>
                      <td className="px-3 py-2 text-neutral-500 text-[11px] font-mono">
                        {trig ?? "manual"}
                      </td>
                    </tr>
                  );
                })}
                {executions.data?.length === 0 && (
                  <tr>
                    <td
                      colSpan={3}
                      className="px-3 py-6 text-center text-neutral-500"
                    >
                      No executions yet.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </section>
      </div>

      {err && (
        <div className="mt-4 mx-6 rounded border border-rose-300 bg-rose-50 dark:bg-rose-950/40 dark:border-rose-900 p-3 text-sm text-rose-700 dark:text-rose-300">
          {err}
        </div>
      )}
    </div>
  );
}
