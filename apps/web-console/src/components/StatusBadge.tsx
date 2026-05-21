import type { ExecutionStatus, NodeStatus } from "@iris/sdk-ts";

const COLORS: Record<string, string> = {
  success:
    "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-300",
  failed: "bg-rose-100 text-rose-800 dark:bg-rose-900/40 dark:text-rose-300",
  skipped:
    "bg-neutral-200 text-neutral-700 dark:bg-neutral-800 dark:text-neutral-400",
};

export function StatusBadge({
  status,
}: {
  status: ExecutionStatus | NodeStatus | string;
}) {
  return (
    <span
      className={`inline-flex items-center rounded px-2 py-0.5 text-[11px] font-medium ${
        COLORS[status] ?? COLORS.skipped
      }`}
    >
      {status}
    </span>
  );
}
