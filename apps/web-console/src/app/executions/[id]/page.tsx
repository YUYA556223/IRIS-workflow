"use client";

import { useQuery } from "@tanstack/react-query";
import { use } from "react";
import { irisClient } from "@/lib/iris";
import { PageHeader } from "@/components/PageHeader";
import { StatusBadge } from "@/components/StatusBadge";

export default function ExecutionDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const exec = useQuery({
    queryKey: ["execution", id],
    queryFn: () => irisClient.getExecution(id),
  });

  return (
    <div>
      <PageHeader
        title={`Execution ${id.slice(0, 8)}…`}
        subtitle={exec.data ? `Workflow: ${exec.data.workflow_id}` : ""}
        actions={
          exec.data && <StatusBadge status={exec.data.status} />
        }
      />
      {exec.data && (
        <div className="px-6 space-y-6">
          <section>
            <h2 className="text-xs font-semibold text-neutral-500 uppercase tracking-wider mb-2">
              Summary
            </h2>
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-sm">
              <Field label="Started" value={new Date(exec.data.started_at).toLocaleString()} />
              <Field label="Finished" value={new Date(exec.data.finished_at).toLocaleString()} />
              <Field
                label="Duration"
                value={
                  String(
                    new Date(exec.data.finished_at).getTime() -
                      new Date(exec.data.started_at).getTime(),
                  ) + "ms"
                }
              />
              <Field label="Nodes" value={String(exec.data.nodes.length)} />
            </div>
            {exec.data.error && (
              <div className="mt-3 rounded border border-rose-300 bg-rose-50 dark:bg-rose-950/40 dark:border-rose-900 p-3 text-sm text-rose-700 dark:text-rose-300">
                {exec.data.error}
              </div>
            )}
          </section>

          <section>
            <h2 className="text-xs font-semibold text-neutral-500 uppercase tracking-wider mb-2">
              Trigger data
            </h2>
            <pre className="text-[11px] border border-neutral-200 dark:border-neutral-800 rounded-lg p-3 overflow-x-auto bg-neutral-50 dark:bg-neutral-950">
              {JSON.stringify(exec.data.trigger_data, null, 2)}
            </pre>
          </section>

          <section>
            <h2 className="text-xs font-semibold text-neutral-500 uppercase tracking-wider mb-2">
              Nodes
            </h2>
            <div className="space-y-3">
              {exec.data.nodes.map((n) => (
                <div
                  key={n.node_id}
                  className="border border-neutral-200 dark:border-neutral-800 rounded-lg p-3 bg-neutral-50/40 dark:bg-neutral-950"
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <code className="text-sm font-bold">{n.node_id}</code>
                      <span className="text-xs text-neutral-500">[{n.kind}]</span>
                    </div>
                    <StatusBadge status={n.status} />
                  </div>
                  <div className="text-[11px] text-neutral-500 mt-1">
                    {new Date(n.started_at).toLocaleTimeString()} →{" "}
                    {new Date(n.finished_at).toLocaleTimeString()} (
                    {new Date(n.finished_at).getTime() -
                      new Date(n.started_at).getTime()}
                    ms)
                  </div>
                  {n.error && (
                    <div className="mt-2 text-xs text-rose-600 dark:text-rose-400">
                      {n.error}
                    </div>
                  )}
                  {n.output !== null && n.output !== undefined && (
                    <pre className="mt-2 text-[11px] bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 rounded p-2 overflow-x-auto">
                      {JSON.stringify(n.output, null, 2)}
                    </pre>
                  )}
                </div>
              ))}
            </div>
          </section>
        </div>
      )}
    </div>
  );
}

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div className="border border-neutral-200 dark:border-neutral-800 rounded p-3">
      <div className="text-[10px] uppercase tracking-wider text-neutral-500">
        {label}
      </div>
      <div className="text-sm mt-1 font-mono">{value}</div>
    </div>
  );
}
