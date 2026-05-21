"use client";

import { useQuery } from "@tanstack/react-query";
import { irisClient } from "@/lib/iris";
import { PageHeader } from "@/components/PageHeader";

export default function DevicesPage() {
  const devices = useQuery({
    queryKey: ["devices"],
    queryFn: () => irisClient.listDevices(),
    refetchInterval: 5000,
  });

  return (
    <div>
      <PageHeader
        title="Devices"
        subtitle="登録済みデバイス。WebSocket 接続中のデバイスへ通知やウィジェット更新が配信されます。"
      />
      <div className="px-6">
        <div className="border border-neutral-200 dark:border-neutral-800 rounded-lg overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-neutral-100 dark:bg-neutral-800 text-xs uppercase text-neutral-600">
              <tr>
                <th className="text-left px-3 py-2">Name</th>
                <th className="text-left px-3 py-2">Kind</th>
                <th className="text-left px-3 py-2">Capabilities</th>
                <th className="text-left px-3 py-2">Registered</th>
                <th className="text-left px-3 py-2">ID</th>
              </tr>
            </thead>
            <tbody>
              {devices.data?.map((d) => (
                <tr
                  key={d.id}
                  className="border-t border-neutral-200 dark:border-neutral-800 hover:bg-neutral-50 dark:hover:bg-neutral-900"
                >
                  <td className="px-3 py-2 font-medium">{d.name}</td>
                  <td className="px-3 py-2 text-neutral-500">{d.kind}</td>
                  <td className="px-3 py-2">
                    <div className="flex gap-1 flex-wrap">
                      {d.capabilities.map((c) => (
                        <span
                          key={c}
                          className="rounded-full bg-neutral-200 dark:bg-neutral-800 px-2 py-0.5 text-[10px]"
                        >
                          {c}
                        </span>
                      ))}
                    </div>
                  </td>
                  <td className="px-3 py-2 text-neutral-500 text-[12px]">
                    {new Date(d.registered_at).toLocaleString()}
                  </td>
                  <td className="px-3 py-2 font-mono text-[10px] text-neutral-500">
                    {d.id}
                  </td>
                </tr>
              ))}
              {devices.data?.length === 0 && (
                <tr>
                  <td
                    colSpan={5}
                    className="px-3 py-6 text-center text-neutral-500"
                  >
                    No devices registered. iOS / Win / IoT クライアントから{" "}
                    <code className="font-mono text-[11px]">POST /devices</code>
                    で登録。
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
