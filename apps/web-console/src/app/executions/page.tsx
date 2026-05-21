"use client";

import { useQuery } from "@tanstack/react-query";
import Link from "next/link";
import { irisClient } from "@/lib/iris";
import { PageHeader } from "@/components/PageHeader";
import { StatusBadge } from "@/components/StatusBadge";

export default function ExecutionsPage() {
  const list = useQuery({
    queryKey: ["executions", { limit: 200 }],
    queryFn: () => irisClient.listExecutions({ limit: 200 }),
    refetchInterval: 5000,
  });

  return (
    <div>
      <PageHeader
        title="Executions"
        subtitle="ワークフロー実行履歴 (最新200件)"
      />
      <div className="px-6">
        <div className="border border-neutral-200 dark:border-neutral-800 rounded-lg overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-neutral-100 dark:bg-neutral-800 text-xs uppercase text-neutral-600">
              <tr>
                <th className="text-left px-3 py-2">Status</th>
                <th className="text-left px-3 py-2">Workflow</th>
                <th className="text-left px-3 py-2">Started</th>
                <th className="text-left px-3 py-2">Duration</th>
                <th className="text-left px-3 py-2">Nodes</th>
                <th className="text-left px-3 py-2">Execution ID</th>
              </tr>
            </thead>
            <tbody>
              {list.data?.map((e) => {
                const dur =
                  new Date(e.finished_at).getTime() -
                  new Date(e.started_at).getTime();
                return (
                  <tr
                    key={e.execution_id}
                    className="border-t border-neutral-200 dark:border-neutral-800 hover:bg-neutral-50 dark:hover:bg-neutral-900"
                  >
                    <td className="px-3 py-2">
                      <StatusBadge status={e.status} />
                    </td>
                    <td className="px-3 py-2 font-mono text-[12px]">
                      <Link
                        href={`/workflows/${e.workflow_id}`}
                        className="text-blue-600 dark:text-blue-400 hover:underline"
                      >
                        {e.workflow_id}
                      </Link>
                    </td>
                    <td className="px-3 py-2 text-neutral-500 text-[12px]">
                      {new Date(e.started_at).toLocaleString()}
                    </td>
                    <td className="px-3 py-2 text-neutral-500 text-[12px]">
                      {dur}ms
                    </td>
                    <td className="px-3 py-2 text-neutral-500">
                      {e.nodes.length}
                    </td>
                    <td className="px-3 py-2 font-mono text-[11px]">
                      <Link
                        href={`/executions/${e.execution_id}`}
                        className="text-blue-600 dark:text-blue-400 hover:underline"
                      >
                        {e.execution_id.slice(0, 8)}…
                      </Link>
                    </td>
                  </tr>
                );
              })}
              {list.data?.length === 0 && (
                <tr>
                  <td colSpan={6} className="px-3 py-6 text-center text-neutral-500">
                    No executions yet.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
