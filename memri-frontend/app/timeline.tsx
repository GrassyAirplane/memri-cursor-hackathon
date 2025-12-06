"use client";

import React, { useEffect, useRef, useState, useCallback, useMemo } from "react";
import { Chrome, AppWindow, Monitor, Music, Video, FileText, Mail, MessageSquare, Terminal, Folder, Image, Code } from "lucide-react";

// Map common app names to icons
const APP_ICONS: Record<string, React.ElementType> = {
  chrome: Chrome,
  firefox: Chrome,
  edge: Chrome,
  safari: Chrome,
  brave: Chrome,
  opera: Chrome,
  vivaldi: Chrome,
  explorer: Folder,
  finder: Folder,
  code: Code,
  "visual studio": Code,
  vscode: Code,
  cursor: Code,
  terminal: Terminal,
  powershell: Terminal,
  cmd: Terminal,
  iterm: Terminal,
  spotify: Music,
  "apple music": Music,
  vlc: Video,
  netflix: Video,
  youtube: Video,
  word: FileText,
  docs: FileText,
  notion: FileText,
  outlook: Mail,
  gmail: Mail,
  mail: Mail,
  slack: MessageSquare,
  discord: MessageSquare,
  teams: MessageSquare,
  zoom: Video,
  photos: Image,
  preview: Image,
};

function getAppIcon(appName?: string): React.ElementType {
  if (!appName) return Monitor;
  const lower = appName.toLowerCase();
  for (const [key, Icon] of Object.entries(APP_ICONS)) {
    if (lower.includes(key)) return Icon;
  }
  return AppWindow;
}

export type CaptureNode = {
  id: string;
  capture_id: number;
  frame_number: number;
  timestamp_ms: number;
  thumbnailUrl?: string;
  fullResolutionUrl?: string;
  metadata?: {
    applicationName?: string;
    windowTitle?: string;
    screenIndex?: number;
  };
};

type TimelineProps = {
  captures: CaptureNode[];
  selectedId: string | null;
  onSelect: (capture: CaptureNode) => void;
  className?: string;
};

export function Timeline({ captures, selectedId, onSelect, className = "" }: TimelineProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [startX, setStartX] = useState(0);
  const [scrollLeft, setScrollLeft] = useState(0);
  const [focusedIndex, setFocusedIndex] = useState<number | null>(null);

  // Group captures by date for separators
  const groupedCaptures = useMemo(() => {
    const groups: { date: string; captures: CaptureNode[] }[] = [];
    let currentDate = "";
    let currentGroup: CaptureNode[] = [];

    captures.forEach((capture) => {
      const date = new Date(capture.timestamp_ms);
      const dateStr = formatDateSeparator(date);
      
      if (dateStr !== currentDate) {
        if (currentGroup.length > 0) {
          groups.push({ date: currentDate, captures: currentGroup });
        }
        currentDate = dateStr;
        currentGroup = [capture];
      } else {
        currentGroup.push(capture);
      }
    });

    if (currentGroup.length > 0) {
      groups.push({ date: currentDate, captures: currentGroup });
    }

    return groups;
  }, [captures]);

  // Auto-scroll to selected thumbnail
  useEffect(() => {
    if (!selectedId || !scrollRef.current) return;
    
    const selectedElement = scrollRef.current.querySelector(`[data-id="${selectedId}"]`);
    if (selectedElement) {
      selectedElement.scrollIntoView({
        behavior: "smooth",
        block: "nearest",
        inline: "center",
      });
    }
  }, [selectedId]);

  // Mouse drag scrolling
  const handleMouseDown = (e: React.MouseEvent) => {
    if (!scrollRef.current) return;
    setIsDragging(true);
    setStartX(e.pageX - scrollRef.current.offsetLeft);
    setScrollLeft(scrollRef.current.scrollLeft);
    if (scrollRef.current) {
      scrollRef.current.style.cursor = "grabbing";
    }
  };

  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!isDragging || !scrollRef.current) return;
    e.preventDefault();
    const x = e.pageX - scrollRef.current.offsetLeft;
    const walk = (x - startX) * 1.5;
    scrollRef.current.scrollLeft = scrollLeft - walk;
  }, [isDragging, startX, scrollLeft]);

  const handleMouseUp = useCallback(() => {
    setIsDragging(false);
    if (scrollRef.current) {
      scrollRef.current.style.cursor = "grab";
    }
  }, []);

  useEffect(() => {
    if (isDragging) {
      window.addEventListener("mousemove", handleMouseMove);
      window.addEventListener("mouseup", handleMouseUp);
      return () => {
        window.removeEventListener("mousemove", handleMouseMove);
        window.removeEventListener("mouseup", handleMouseUp);
      };
    }
  }, [isDragging, handleMouseMove, handleMouseUp]);

  // Keyboard navigation
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (captures.length === 0) return;
    
    const currentIndex = captures.findIndex((c) => c.id === selectedId);
    
    switch (e.key) {
      case "ArrowLeft":
        e.preventDefault();
        if (currentIndex > 0) {
          onSelect(captures[currentIndex - 1]);
          setFocusedIndex(currentIndex - 1);
        }
        break;
      case "ArrowRight":
        e.preventDefault();
        if (currentIndex < captures.length - 1) {
          onSelect(captures[currentIndex + 1]);
          setFocusedIndex(currentIndex + 1);
        }
        break;
      case "Home":
        e.preventDefault();
        onSelect(captures[0]);
        setFocusedIndex(0);
        break;
      case "End":
        e.preventDefault();
        onSelect(captures[captures.length - 1]);
        setFocusedIndex(captures.length - 1);
        break;
      case "Enter":
      case " ":
        e.preventDefault();
        if (focusedIndex !== null && captures[focusedIndex]) {
          onSelect(captures[focusedIndex]);
        }
        break;
    }
  };

  const selectedIndex = captures.findIndex((c) => c.id === selectedId);

  return (
    <nav
      ref={containerRef}
      role="navigation"
      aria-label="Screen capture timeline"
      className={`px-[var(--space-lg)] py-[var(--space-md)] ${className}`}
      onKeyDown={handleKeyDown}
      tabIndex={0}
    >
      {/* Header row - matches input bar top row */}
      <div className="mb-2 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-xs font-medium text-[var(--color-text-secondary)]">
            Timeline
          </span>
          <span className="rounded-full bg-[var(--color-bg-elevated)] px-2 py-0.5 text-[10px] font-medium text-[var(--color-text-tertiary)]">
            {captures.length}
          </span>
        </div>
        <span className="text-xs text-[var(--color-text-tertiary)]">
          {selectedIndex >= 0 ? `${selectedIndex + 1} of ${captures.length}` : "—"}
        </span>
      </div>

      {/* Timeline scrubber */}
      <div className="relative h-full">
        {/* Scroll container */}
        <div
          ref={scrollRef}
          role="list"
          aria-label="Chronological screen captures"
          className="flex h-full items-end gap-1 overflow-x-auto transition-fast"
          style={{
            cursor: isDragging ? "grabbing" : "grab",
            scrollbarWidth: "none",
            msOverflowStyle: "none",
          }}
          onMouseDown={handleMouseDown}
        >
          {groupedCaptures.map((group, groupIdx) => (
            <div key={groupIdx} className="flex items-end gap-1">
              {/* Date separator */}
              {groupIdx > 0 && (
                <div className="mx-2 flex h-full flex-col items-center justify-end pb-1">
                  <div
                    className="whitespace-nowrap rounded-full px-2 py-0.5 text-[9px] font-medium text-[var(--color-text-tertiary)]"
                    style={{
                      background: "var(--color-bg-elevated)",
                      border: "1px solid var(--color-border)",
                    }}
                  >
                    {group.date}
                  </div>
                </div>
              )}
              
              {/* Bars for this date group */}
              {group.captures.map((capture) => (
                <ThumbnailNode
                  key={capture.id}
                  capture={capture}
                  isSelected={capture.id === selectedId}
                  isFocused={focusedIndex === captures.indexOf(capture)}
                  onClick={() => onSelect(capture)}
                  onFocus={() => setFocusedIndex(captures.indexOf(capture))}
                />
              ))}
            </div>
          ))}
          
          {captures.length === 0 && (
            <div className="flex h-full w-full items-center justify-center">
              <p className="text-sm text-[var(--color-text-tertiary)]">
                No screen captures yet. Start recording to see your timeline.
              </p>
            </div>
          )}
        </div>
      </div>
    </nav>
  );
}

type ThumbnailNodeProps = {
  capture: CaptureNode;
  isSelected: boolean;
  isFocused: boolean;
  onClick: () => void;
  onFocus: () => void;
};

function ThumbnailNode({ capture, isSelected, isFocused, onClick, onFocus }: ThumbnailNodeProps) {
  const [isHovered, setIsHovered] = useState(false);
  const timestamp = new Date(capture.timestamp_ms);
  const AppIcon = getAppIcon(capture.metadata?.applicationName);

  return (
    <div
      role="listitem"
      aria-label={`Screen capture from ${formatTimestamp(timestamp)}`}
      className="relative flex flex-shrink-0 flex-col items-center"
      style={{ minWidth: "16px" }}
    >
      {/* App icon */}
      <div
        className="mb-1.5 flex items-center justify-center transition-all"
        style={{
          opacity: isSelected ? 1 : isHovered ? 0.85 : 0.4,
          transform: isSelected ? "scale(1.2)" : isHovered ? "scale(1.1)" : "scale(1)",
          transitionDuration: "200ms",
        }}
      >
        <AppIcon 
          className="h-4 w-4"
          style={{
            color: isSelected ? "var(--color-primary)" : "var(--color-text-secondary)",
          }}
        />
      </div>

      {/* Bar - taller and wider for better visibility */}
      <button
        data-id={capture.id}
        onClick={onClick}
        onFocus={onFocus}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
        aria-pressed={isSelected}
        className="group relative block transition-all"
        style={{
          width: isSelected ? "10px" : isHovered ? "9px" : "8px",
          height: isSelected ? "36px" : isHovered ? "30px" : "24px",
          borderRadius: "4px",
          background: isSelected
            ? "var(--color-primary)"
            : isHovered
            ? "var(--color-text-secondary)"
            : "var(--color-border)",
          opacity: isSelected || isHovered ? 1 : 0.5,
          cursor: "pointer",
          transitionDuration: "200ms",
          transitionTimingFunction: "cubic-bezier(0.34, 1.56, 0.64, 1)", // Bouncy easing
          outline: isFocused ? "2px solid var(--color-primary)" : "none",
          outlineOffset: "2px",
          boxShadow: isSelected 
            ? "0 0 12px rgba(0, 212, 184, 0.5), 0 2px 4px rgba(0, 0, 0, 0.1)" 
            : isHovered
            ? "0 2px 6px rgba(0, 0, 0, 0.1)"
            : "none",
        }}
      >
        <span className="sr-only">Capture at {formatTimestamp(timestamp)}</span>
      </button>

      {/* Hover tooltip */}
      {isHovered && (
        <div
          className="pointer-events-none absolute bottom-full left-1/2 mb-2 -translate-x-1/2 whitespace-nowrap transition-all"
          style={{
            animation: "fadeInUp 150ms ease-out",
            background: "var(--color-card-bg)",
            border: "1px solid var(--color-border)",
            borderRadius: "var(--radius-sm)",
            padding: "8px 12px",
            boxShadow: "0 4px 16px rgba(0, 0, 0, 0.12)",
            zIndex: 50,
          }}
        >
          <div className="flex items-center gap-2">
            <AppIcon className="h-4 w-4 text-[var(--color-text-secondary)]" />
            <div className="text-xs font-medium text-[var(--color-text)]">
              {capture.metadata?.applicationName || "Unknown"}
            </div>
          </div>
          <div className="mt-1 text-[11px] text-[var(--color-text-tertiary)]">
            {formatFullTimestamp(timestamp)}
          </div>
          {capture.metadata?.windowTitle && (
            <div className="mt-0.5 max-w-[180px] truncate text-[10px] text-[var(--color-text-tertiary)]">
              {capture.metadata.windowTitle}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// Helper functions for date formatting
function formatDateSeparator(date: Date): string {
  const now = new Date();
  const diffDays = Math.floor((now.getTime() - date.getTime()) / (1000 * 60 * 60 * 24));
  
  if (diffDays === 0) return "Today";
  if (diffDays === 1) return "Yesterday";
  if (diffDays < 7) {
    return date.toLocaleDateString("en-US", { weekday: "short" });
  }
  return date.toLocaleDateString("en-US", { month: "short", day: "numeric" });
}

function formatTimelineTimestamp(date: Date): string {
  return date.toLocaleTimeString("en-US", {
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
  }).replace(" ", "");
}

function formatFullTimestamp(date: Date): string {
  const now = new Date();
  const diffDays = Math.floor((now.getTime() - date.getTime()) / (1000 * 60 * 60 * 24));
  
  const timeStr = date.toLocaleTimeString("en-US", {
    hour: "numeric",
    minute: "2-digit",
    second: "2-digit",
    hour12: true,
  });
  
  if (diffDays === 0) return `Today, ${timeStr}`;
  if (diffDays === 1) return `Yesterday, ${timeStr}`;
  
  return date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
  });
}

function formatTimestamp(date: Date): string {
  const now = new Date();
  const diffDays = Math.floor((now.getTime() - date.getTime()) / (1000 * 60 * 60 * 24));
  
  if (diffDays === 0) {
    return date.toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit", hour12: true });
  }
  if (diffDays < 7) {
    return date.toLocaleDateString("en-US", {
      weekday: "short",
      hour: "numeric",
      minute: "2-digit",
      hour12: true,
    });
  }
  return date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
  });
}
