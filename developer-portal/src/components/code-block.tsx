"use client";

import { useState } from "react";

interface CodeBlockProps {
  code: string;
  language?: string;
  filename?: string;
}

export function CodeBlock({ code, language = "typescript", filename }: CodeBlockProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="group relative my-4 overflow-hidden rounded-xl border border-slate-800/60 bg-slate-900/60">
      {(filename || language) && (
        <div className="flex items-center justify-between border-b border-slate-800/60 px-4 py-2">
          <span className="text-xs text-slate-500">{filename || language}</span>
          <button
            onClick={handleCopy}
            className="rounded-md px-2 py-1 text-xs text-slate-500 transition-colors hover:bg-slate-800 hover:text-slate-300"
          >
            {copied ? "Copied!" : "Copy"}
          </button>
        </div>
      )}
      {!filename && !language && (
        <button
          onClick={handleCopy}
          className="absolute right-3 top-3 rounded-md px-2 py-1 text-xs text-slate-500 opacity-0 transition-all hover:bg-slate-800 hover:text-slate-300 group-hover:opacity-100"
        >
          {copied ? "Copied!" : "Copy"}
        </button>
      )}
      <pre className="!m-0 !rounded-none !border-0 !bg-transparent px-5 py-4 text-sm leading-relaxed">
        <code className="text-slate-300">{code}</code>
      </pre>
    </div>
  );
}

interface LanguageTabsProps {
  tabs: { label: string; language: string; code: string }[];
}

export function LanguageTabs({ tabs }: LanguageTabsProps) {
  const [active, setActive] = useState(0);

  return (
    <div className="my-4 overflow-hidden rounded-xl border border-slate-800/60 bg-slate-900/60">
      <div className="flex border-b border-slate-800/60">
        {tabs.map((tab, i) => (
          <button
            key={tab.label}
            onClick={() => setActive(i)}
            className={`px-4 py-2.5 text-xs font-medium transition-colors ${
              active === i
                ? "border-b-2 border-blue-500 text-blue-400"
                : "text-slate-500 hover:text-slate-300"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>
      <CodeBlock
        code={tabs[active].code}
        language={tabs[active].language}
      />
    </div>
  );
}
