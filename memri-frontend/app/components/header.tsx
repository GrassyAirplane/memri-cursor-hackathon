"use client";

import { Search, Sparkles } from "lucide-react";

type HeaderProps = {
  search: string;
  onSearchChange: (value: string) => void;
  connected: boolean;
};

export function Header({ search, onSearchChange, connected }: HeaderProps) {
  return (
    <header className="flex h-12 flex-shrink-0 items-center justify-between border-b border-[var(--color-border)] bg-[var(--color-bg)] px-5">
      {/* Left section: Logo + Search */}
      <div className="flex items-center gap-4">
        {/* Logo */}
        <div className="flex items-center gap-2">
          <Sparkles className="h-4 w-4 text-[var(--color-primary)]" />
          <span
            className="text-sm font-semibold text-[var(--color-text)]"
            style={{ letterSpacing: "-0.02em" }}
          >
            Memri
          </span>
        </div>

        {/* Divider */}
        <div className="h-4 w-px bg-[var(--color-border)]" />

        {/* Search */}
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-[var(--color-text-tertiary)]" />
          <input
            type="text"
            value={search}
            onChange={(e) => onSearchChange(e.target.value)}
            placeholder="Search captures..."
            className="h-8 w-56 rounded-[var(--radius-sm)] border border-[var(--color-border)] bg-[var(--color-bg)] pl-8 pr-3 text-xs text-[var(--color-text)] placeholder:text-[var(--color-text-tertiary)] transition-all focus:border-[var(--color-primary)] focus:outline-none focus:ring-1 focus:ring-[var(--color-primary)]/20"
          />
        </div>
      </div>

      {/* Right section: Status */}
      <div className="flex items-center gap-2">
        <div
          className={`h-1.5 w-1.5 rounded-full ${
            connected ? "bg-[var(--color-success)]" : "bg-[var(--color-warning)]"
          }`}
        />
        <span className="text-[11px] font-medium text-[var(--color-text-tertiary)]">
          {connected ? "Live" : "Offline"}
        </span>
      </div>
    </header>
  );
}

