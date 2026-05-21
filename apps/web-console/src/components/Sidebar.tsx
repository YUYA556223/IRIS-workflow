"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

const NAV = [
  { href: "/", label: "Overview", icon: "▦" },
  { href: "/workflows", label: "Workflows", icon: "⬡" },
  { href: "/executions", label: "Executions", icon: "▶" },
  { href: "/devices", label: "Devices", icon: "▲" },
  { href: "/live", label: "Live", icon: "●" },
];

export function Sidebar() {
  const path = usePathname() ?? "/";
  return (
    <aside className="w-56 shrink-0 border-r border-neutral-200 dark:border-neutral-800 bg-neutral-50 dark:bg-neutral-950 flex flex-col">
      <div className="px-4 py-5 border-b border-neutral-200 dark:border-neutral-800">
        <Link href="/" className="block">
          <div className="text-sm uppercase tracking-widest text-neutral-500">
            IRIS
          </div>
          <div className="text-lg font-bold">workflow</div>
        </Link>
      </div>
      <nav className="flex-1 p-2 space-y-1">
        {NAV.map((n) => {
          const active = n.href === "/" ? path === "/" : path.startsWith(n.href);
          return (
            <Link
              key={n.href}
              href={n.href}
              className={`flex items-center gap-3 rounded px-3 py-2 text-sm transition ${
                active
                  ? "bg-neutral-900 text-white dark:bg-white dark:text-neutral-900"
                  : "text-neutral-600 hover:bg-neutral-200 dark:text-neutral-400 dark:hover:bg-neutral-800"
              }`}
            >
              <span className="w-4 text-center opacity-70">{n.icon}</span>
              <span>{n.label}</span>
            </Link>
          );
        })}
      </nav>
      <div className="p-3 text-[10px] text-neutral-500 border-t border-neutral-200 dark:border-neutral-800">
        host-backend @ {process.env.NEXT_PUBLIC_IRIS_BASE_URL ?? "127.0.0.1:8787"}
      </div>
    </aside>
  );
}
