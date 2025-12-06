"use client";

import { Monitor } from "lucide-react";

export type ClipData = {
  capture_id: number;
  timestamp_ms: number;
  app_name?: string;
  window_name?: string;
  preview_text?: string;
};

type ClipReferenceProps = {
  clip: ClipData;
  onClick?: () => void;
};

// Inline annotated clip reference - highlighted text that's clickable
export function ClipReference({ clip, onClick }: ClipReferenceProps) {
  const timestamp = new Date(clip.timestamp_ms);
  const timeStr = timestamp.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });

  const label = clip.app_name || "capture";

  return (
    <button
      onClick={onClick}
      className="mx-0.5 inline-flex items-center gap-1 rounded-md bg-[var(--color-primary)]/15 px-1.5 py-0.5 text-[var(--color-primary)] transition-all hover:bg-[var(--color-primary)]/25 hover:underline"
      style={{
        fontSize: "inherit",
        lineHeight: "inherit",
      }}
      title={`${clip.app_name || "Capture"} - ${clip.window_name || ""} at ${timeStr}`}
    >
      <Monitor className="h-3 w-3" />
      <span className="font-medium">{label}</span>
      <span className="text-[var(--color-primary)]/70">({timeStr})</span>
    </button>
  );
}

// Parse clip references and return segments for inline rendering
// Supports: [[CLIP:capture_id]] format
export type MessageSegment = 
  | { type: "text"; content: string }
  | { type: "clip"; clip: ClipData };

export function parseClipReferences(
  content: string,
  capturesLookup?: Map<number, { timestamp_ms: number; app_name?: string; window_name?: string }>
): {
  text: string;
  clips: ClipData[];
  segments: MessageSegment[];
} {
  const clips: ClipData[] = [];
  const segments: MessageSegment[] = [];
  
  // Match [[CLIP:ID]] pattern
  const clipRegex = /\[\[CLIP:(\d+)\]\]/gi;
  let lastIndex = 0;
  let match;
  
  while ((match = clipRegex.exec(content)) !== null) {
    // Add text before this clip
    if (match.index > lastIndex) {
      const textBefore = content.slice(lastIndex, match.index);
      if (textBefore.trim()) {
        segments.push({ type: "text", content: textBefore });
      }
    }
    
    const captureId = parseInt(match[1], 10);
    const lookupData = capturesLookup?.get(captureId);
    
    const clipData: ClipData = {
      capture_id: captureId,
      timestamp_ms: lookupData?.timestamp_ms || Date.now(),
      app_name: lookupData?.app_name,
      window_name: lookupData?.window_name,
    };
    
    clips.push(clipData);
    segments.push({ type: "clip", clip: clipData });
    
    lastIndex = match.index + match[0].length;
  }
  
  // Add remaining text after last clip
  if (lastIndex < content.length) {
    const textAfter = content.slice(lastIndex);
    if (textAfter.trim()) {
      segments.push({ type: "text", content: textAfter });
    }
  }
  
  // If no clips found, just return the whole content as text
  if (segments.length === 0 && content.trim()) {
    segments.push({ type: "text", content });
  }
  
  // Build plain text version (without clip markers)
  const text = content.replace(/\[\[CLIP:\d+\]\]/gi, "").trim();

  return { text, clips, segments };
}

