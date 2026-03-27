"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useState } from "react";
import { NAV_SECTIONS } from "@/lib/constants";

function ChevronIcon({ open }: { open: boolean }) {
  return (
    <svg
      className={`h-4 w-4 text-slate-500 transition-transform duration-200 ${open ? "rotate-90" : ""}`}
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      strokeWidth={2}
    >
      <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
    </svg>
  );
}

function SidebarSection({
  section,
  pathname,
}: {
  section: (typeof NAV_SECTIONS)[number];
  pathname: string;
}) {
  const isActive = section.items.some((item) => item.href === pathname);
  const [open, setOpen] = useState(isActive || section.title === "Getting Started");

  return (
    <div className="mb-1">
      <button
        onClick={() => setOpen(!open)}
        className="flex w-full items-center justify-between rounded-md px-3 py-2 text-sm font-semibold text-slate-300 transition-colors hover:bg-slate-800/60 hover:text-white"
      >
        {section.title}
        <ChevronIcon open={open} />
      </button>

      {open && (
        <ul className="ml-3 mt-0.5 space-y-0.5 border-l border-slate-800 pl-3">
          {section.items.map((item) => {
            const active = pathname === item.href;
            return (
              <li key={item.href}>
                <Link
                  href={item.href}
                  className={`block rounded-md px-3 py-1.5 text-sm transition-colors ${
                    active
                      ? "bg-blue-600/15 font-medium text-blue-400"
                      : "text-slate-400 hover:bg-slate-800/40 hover:text-slate-200"
                  }`}
                >
                  {item.title}
                </Link>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

export default function DocsLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();

  return (
    <div className="flex min-h-[calc(100vh-65px)]">
      {/* Left Sidebar */}
      <aside className="sticky top-[65px] hidden h-[calc(100vh-65px)] w-64 shrink-0 overflow-y-auto border-r border-slate-800/60 bg-slate-950 px-3 py-6 lg:block">
        <nav className="space-y-1">
          {NAV_SECTIONS.map((section) => (
            <SidebarSection
              key={section.title}
              section={section}
              pathname={pathname}
            />
          ))}
        </nav>
      </aside>

      {/* Main Content */}
      <div className="flex-1 overflow-x-hidden">
        <div className="mx-auto max-w-4xl px-6 py-10 lg:px-10">
          {children}
        </div>
      </div>
    </div>
  );
}
