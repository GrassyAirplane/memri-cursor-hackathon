"use client";

import { ChevronLeft, ChevronRight, Monitor } from "lucide-react";

export type CaptureWindow = {
  window_name: string;
  app_name: string;
  text: string;
  confidence?: number | null;
  browser_url?: string | null;
  image_base64?: string | null;
};

export type Capture = {
  capture_id: number;
  frame_number: number;
  timestamp_ms: number;
  windows: CaptureWindow[];
};

type CapturePreviewProps = {
  selectedCapture: Capture | null;
  selectedIndex: number;
  totalCount: number;
  onPrev: () => void;
  onNext: () => void;
};

export function CapturePreview({
  selectedCapture,
  selectedIndex,
  totalCount,
  onPrev,
  onNext,
}: CapturePreviewProps) {
  const selectedWindow = selectedCapture?.windows?.[0];
  const hasPrev = selectedIndex > 0;
  const hasNext = selectedIndex >= 0 && selectedIndex < totalCount - 1;

  return (
    <div className="relative flex flex-1 flex-col bg-[var(--color-bg-elevated)]">
      {/* Main image container */}
      <div className="relative flex flex-1 items-center justify-center overflow-hidden p-8">
        {selectedWindow?.image_base64 ? (
          <div className="relative flex h-full w-full items-center justify-center">
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img
              src={`data:image/png;base64,${selectedWindow.image_base64}`}
              alt={selectedWindow.window_name || "Screen capture"}
              className="rounded-lg object-contain"
              style={{
                maxHeight: "100%",
                maxWidth: "100%",
                boxShadow: "0 8px 32px rgba(0, 0, 0, 0.12)",
              }}
            />
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center gap-3 text-[var(--color-text-tertiary)]">
            <Monitor className="h-12 w-12 opacity-30" />
            <span className="text-sm">No capture selected</span>
          </div>
        )}

        {/* Navigation arrows */}
        {selectedCapture && (
          <>
            <NavButton
              direction="left"
              onClick={onPrev}
              disabled={!hasPrev}
            />
            <NavButton
              direction="right"
              onClick={onNext}
              disabled={!hasNext}
            />
          </>
        )}
      </div>

      {/* Bottom info bar */}
      {selectedWindow && (
        <div className="flex items-center justify-between border-t border-[var(--color-border)] bg-[var(--color-bg)] px-4 py-2">
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2">
              <div className="h-2 w-2 rounded-full bg-[var(--color-primary)]" />
              <span className="text-xs font-medium text-[var(--color-text)]">
                {selectedWindow.app_name || "Unknown App"}
              </span>
            </div>
            <div className="h-3 w-px bg-[var(--color-border)]" />
            <span className="max-w-[300px] truncate text-xs text-[var(--color-text-secondary)]">
              {selectedWindow.window_name || "Untitled"}
            </span>
          </div>

          <div className="flex items-center gap-3">
            {selectedCapture && (
              <>
                <span className="text-[11px] font-medium text-[var(--color-text-tertiary)]">
                  {new Date(selectedCapture.timestamp_ms).toLocaleTimeString([], {
                    hour: "2-digit",
                    minute: "2-digit",
                    second: "2-digit",
                  })}
                </span>
                <div className="h-3 w-px bg-[var(--color-border)]" />
                <span className="text-[11px] text-[var(--color-text-tertiary)]">
                  {selectedIndex + 1} / {totalCount}
                </span>
              </>
            )}
          </div>
        </div>
      )}

      {/* OCR text panel (collapsible) */}
      {selectedWindow?.text && (
        <div className="max-h-24 overflow-y-auto border-t border-[var(--color-border)] bg-[var(--color-bg)] px-4 py-3">
          <p className="font-mono text-[11px] leading-relaxed text-[var(--color-text-secondary)]">
            {selectedWindow.text}
          </p>
        </div>
      )}
    </div>
  );
}

type NavButtonProps = {
  direction: "left" | "right";
  onClick: () => void;
  disabled: boolean;
};

function NavButton({ direction, onClick, disabled }: NavButtonProps) {
  const Icon = direction === "left" ? ChevronLeft : ChevronRight;
  const position = direction === "left" ? "left-4" : "right-4";

  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={`absolute top-1/2 ${position} -translate-y-1/2 rounded-full border border-[var(--color-border)] bg-[var(--color-card-bg)] p-2 transition-all hover:border-[var(--color-primary)] hover:bg-[var(--color-hover)] disabled:cursor-not-allowed disabled:opacity-30 disabled:hover:border-[var(--color-border)] disabled:hover:bg-[var(--color-card-bg)]`}
      style={{
        boxShadow: disabled ? "none" : "0 2px 8px rgba(0, 0, 0, 0.08)",
      }}
    >
      <Icon className="h-4 w-4 text-[var(--color-text)]" />
    </button>
  );
}

