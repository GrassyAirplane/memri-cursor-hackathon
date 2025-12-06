"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { Plus, PlayCircle, Square, Trash2, Sparkles, X, Link2 } from "lucide-react";
import { MEMRI_API_KEY, MEMRI_API_URL } from "../constants";

type Capture = {
  capture_id: number;
  timestamp_ms: number;
  windows: {
    app_name: string;
    window_name: string;
    text: string;
    browser_url?: string | null;
  }[];
};

type Workflow = {
  id: string;
  title: string;
  steps: string;
  clipIds: number[];
  updatedAt: number;
};

export default function WorkflowDashboard() {
  const headers = useMemo(() => {
    const base: Record<string, string> = { "Content-Type": "application/json" };
    if (MEMRI_API_KEY) base["x-api-key"] = MEMRI_API_KEY;
    return base;
  }, []);

  const [captures, setCaptures] = useState<Capture[]>([]);
  const [workflows, setWorkflows] = useState<Workflow[]>([]);
  const [selectedWorkflow, setSelectedWorkflow] = useState<Workflow | null>(null);
  const [recording, setRecording] = useState(false);
  const [sessionStart, setSessionStart] = useState<number | null>(null);
  const [loadingSummary, setLoadingSummary] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Fetch captures metadata (no images) for context
  const fetchCaptures = useCallback(async (): Promise<Capture[] | undefined> => {
    try {
      const res = await fetch(`${MEMRI_API_URL}/captures`, { headers });
      if (!res.ok) return;
      const data = (await res.json()) as Capture[];
      // Sort chronological ascending
      const sorted = [...data].sort((a, b) => a.timestamp_ms - b.timestamp_ms);
      setCaptures(sorted);
      return sorted;
    } catch (err) {
      console.error("Failed to fetch captures", err);
    }
  }, [headers]);

  useEffect(() => {
    fetchCaptures();
  }, [fetchCaptures]);

  // Start / stop session
  const toggleRecording = async () => {
    if (!recording) {
      setRecording(true);
      setSessionStart(Date.now());
      setError(null);
      return;
    }

    // Stop session
    setRecording(false);
    const sessionEnd = Date.now();
    if (!sessionStart) return;

    // Refresh captures to include newest screenshots
    const latest = await fetchCaptures();
    const source = latest ?? captures;

    // Gather captures in range (with small tolerance)
    const startWindow = sessionStart - 2000;
    const endWindow = sessionEnd + 2000;
    const inRange = source.filter(
      (c) => c.timestamp_ms >= startWindow && c.timestamp_ms <= endWindow,
    );
    if (inRange.length === 0) {
      setError("No captures found in this session window.");
      return;
    }

    setLoadingSummary(true);
    try {
      const summary = await summarizeWorkflow(inRange, headers);
      const clipIds = inRange.map((c) => c.capture_id);
      const newWorkflow: Workflow = {
        id: `wf-${Date.now()}`,
        title: summary.title || "Untitled workflow",
        steps: summary.steps || summary.raw || "No steps generated.",
        clipIds,
        updatedAt: Date.now(),
      };
      setWorkflows((prev) => [newWorkflow, ...prev]);
    } catch (err) {
      console.error(err);
      setError("Failed to generate workflow.");
    } finally {
      setLoadingSummary(false);
      setSessionStart(null);
    }
  };

  const deleteWorkflow = (id: string) => {
    setWorkflows((prev) => prev.filter((w) => w.id !== id));
    if (selectedWorkflow?.id === id) setSelectedWorkflow(null);
  };

  const updateWorkflowTitle = (id: string, title: string) => {
    setWorkflows((prev) =>
      prev.map((w) => (w.id === id ? { ...w, title, updatedAt: Date.now() } : w)),
    );
  };

  const saveSteps = (id: string, steps: string) => {
    setWorkflows((prev) =>
      prev.map((w) => (w.id === id ? { ...w, steps, updatedAt: Date.now() } : w)),
    );
  };

  const captureLookup = useMemo(() => {
    const map = new Map<number, Capture>();
    captures.forEach((c) => map.set(c.capture_id, c));
    return map;
  }, [captures]);

  return (
    <div className="flex h-screen flex-col bg-[var(--color-bg)] text-[var(--color-text)]">
      {/* Top bar */}
      <header className="flex h-12 flex-shrink-0 items-center justify-between border-b border-[var(--color-border)] bg-[var(--color-bg)] px-5">
        <div className="flex items-center gap-3">
          <Link href="/" className="text-xs text-[var(--color-text-secondary)] hover:text-[var(--color-primary)] transition-all">
            ← Back
          </Link>
          <div className="flex items-center gap-2">
            <Sparkles className="h-4 w-4 text-[var(--color-primary)]" />
            <span className="text-sm font-semibold" style={{ letterSpacing: "-0.02em" }}>
              Workflow Dashboard
            </span>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={toggleRecording}
            className="inline-flex items-center gap-2 rounded-md border border-[var(--color-border)] bg-[var(--color-bg-elevated)] px-3 py-1.5 text-xs font-medium text-[var(--color-text)] transition-all hover:border-[var(--color-primary)] hover:bg-[var(--color-hover)]"
          >
            {recording ? (
              <>
                <Square className="h-3.5 w-3.5 text-[var(--color-warning)]" />
                Stop session
              </>
            ) : (
              <>
                <PlayCircle className="h-3.5 w-3.5 text-[var(--color-primary)]" />
                Start session
              </>
            )}
          </button>
          <button
            onClick={() =>
              setWorkflows((prev) => [
                {
                  id: `wf-${Date.now()}`,
                  title: "Untitled workflow",
                  steps: "Describe the steps here...",
                  clipIds: [],
                  updatedAt: Date.now(),
                },
                ...prev,
              ])
            }
            className="inline-flex items-center gap-2 rounded-md bg-[var(--color-primary)] px-3 py-1.5 text-xs font-medium text-white transition-all hover:brightness-110"
          >
            <Plus className="h-3.5 w-3.5" />
            New workflow
          </button>
        </div>
      </header>

      {/* Content */}
      <div className="flex flex-1 flex-col overflow-hidden px-6 py-5">
        {error && (
          <div className="mb-3 rounded-md border border-[var(--color-warning)] bg-[var(--color-warning)]/10 px-3 py-2 text-xs text-[var(--color-text)]">
            {error}
          </div>
        )}
        {loadingSummary && (
          <div className="mb-3 rounded-md border border-[var(--color-border)] bg-[var(--color-bg-elevated)] px-3 py-2 text-xs text-[var(--color-text-secondary)]">
            Summarizing session into workflow steps...
          </div>
        )}

        <div className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
          {workflows.map((wf) => (
            <div
              key={wf.id}
              className="group flex flex-col rounded-[var(--radius-md)] border border-[var(--color-border)] bg-[var(--color-bg-elevated)] p-4 transition-all hover:border-[var(--color-primary)]"
            >
              <div className="flex items-start justify-between">
                <input
                  value={wf.title}
                  onChange={(e) => updateWorkflowTitle(wf.id, e.target.value)}
                  className="w-full border-none bg-transparent text-sm font-semibold text-[var(--color-text)] focus:outline-none focus:ring-0"
                />
                <button
                  onClick={() => deleteWorkflow(wf.id)}
                  className="ml-2 rounded-md p-1 text-[var(--color-text-tertiary)] transition-all hover:bg-[var(--color-hover)] hover:text-[var(--color-warning)]"
                  aria-label="Delete workflow"
                >
                  <Trash2 className="h-4 w-4" />
                </button>
              </div>

              <div className="mt-2 line-clamp-3 text-xs text-[var(--color-text-secondary)] whitespace-pre-wrap">
                {wf.steps}
              </div>

              <div className="mt-3 flex flex-wrap gap-1">
                {wf.clipIds.slice(0, 3).map((id) => {
                  const cap = captureLookup.get(id);
                  const ts = cap ? new Date(cap.timestamp_ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }) : "";
                  return (
                    <span
                      key={id}
                      className="inline-flex items-center gap-1 rounded-full bg-[var(--color-primary)]/10 px-2 py-1 text-[11px] text-[var(--color-primary)]"
                    >
                      <Link2 className="h-3 w-3" />
                      {ts || `Clip ${id}`}
                    </span>
                  );
                })}
                {wf.clipIds.length > 3 && (
                  <span className="rounded-full bg-[var(--color-border)] px-2 py-1 text-[11px] text-[var(--color-text-secondary)]">
                    +{wf.clipIds.length - 3} more
                  </span>
                )}
              </div>

              <div className="mt-4 flex items-center justify-between text-[11px] text-[var(--color-text-tertiary)]">
                <span>
                  Updated {new Date(wf.updatedAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
                </span>
                <button
                  onClick={() => setSelectedWorkflow(wf)}
                  className="text-[var(--color-primary)] transition-all hover:underline"
                >
                  Open
                </button>
              </div>
            </div>
          ))}

          {workflows.length === 0 && (
            <div className="rounded-[var(--radius-md)] border border-dashed border-[var(--color-border)] bg-[var(--color-bg-elevated)] p-6 text-sm text-[var(--color-text-secondary)]">
              No workflows yet. Start a session or create one manually.
            </div>
          )}
        </div>
      </div>

      {/* Modal for workflow details */}
      {selectedWorkflow && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 px-4">
          <div className="relative w-full max-w-3xl rounded-[var(--radius-md)] border border-[var(--color-border)] bg-[var(--color-bg)] shadow-lg">
            <button
              onClick={() => setSelectedWorkflow(null)}
              className="absolute right-3 top-3 rounded-md p-1 text-[var(--color-text-tertiary)] transition-all hover:bg-[var(--color-hover)]"
              aria-label="Close"
            >
              <X className="h-4 w-4" />
            </button>
            <div className="p-4">
              <input
                value={selectedWorkflow.title}
                onChange={(e) => updateWorkflowTitle(selectedWorkflow.id, e.target.value)}
                className="w-full border-none bg-transparent text-lg font-semibold text-[var(--color-text)] focus:outline-none focus:ring-0"
              />
              <div className="mt-3 text-xs text-[var(--color-text-tertiary)]">
                Clips: {selectedWorkflow.clipIds.length} · Updated{" "}
                {new Date(selectedWorkflow.updatedAt).toLocaleString([], { hour: "2-digit", minute: "2-digit" })}
              </div>

              <div className="mt-4 space-y-2">
                <label className="text-xs font-medium text-[var(--color-text-secondary)]">Steps</label>
                <textarea
                  value={selectedWorkflow.steps}
                  onChange={(e) => saveSteps(selectedWorkflow.id, e.target.value)}
                  className="min-h-[180px] w-full rounded-[var(--radius-sm)] border border-[var(--color-border)] bg-[var(--color-bg-elevated)] p-3 text-sm text-[var(--color-text)] focus:border-[var(--color-primary)] focus:outline-none"
                />
              </div>

              <div className="mt-4">
                <div className="text-xs font-medium text-[var(--color-text-secondary)]">Clip links</div>
                <div className="mt-2 flex flex-wrap gap-1.5">
                  {selectedWorkflow.clipIds.map((id) => {
                    const cap = captureLookup.get(id);
                    const ts = cap ? new Date(cap.timestamp_ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }) : "";
                    const title = cap?.windows?.[0]?.window_name || cap?.windows?.[0]?.app_name || `Clip ${id}`;
                    return (
                      <span
                        key={id}
                        className="inline-flex items-center gap-1 rounded-full bg-[var(--color-primary)]/12 px-3 py-1.5 text-[11px] font-medium text-[var(--color-primary)]"
                      >
                        <Link2 className="h-3 w-3" />
                        <span className="truncate max-w-[180px]">{title}</span>
                        <span className="text-[var(--color-text-tertiary)]">· {ts}</span>
                      </span>
                    );
                  })}
                  {selectedWorkflow.clipIds.length === 0 && (
                    <span className="rounded-full bg-[var(--color-border)] px-3 py-1.5 text-[11px] text-[var(--color-text-tertiary)]">
                      No clips linked
                    </span>
                  )}
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// Summarize a range of captures into workflow steps using the assistant endpoint (non-stream)
async function summarizeWorkflow(captures: Capture[], headers: Record<string, string>) {
  // Build compact context
  const context = captures
    .map((cap) => {
      const ts = new Date(cap.timestamp_ms).toISOString();
      const win = cap.windows?.[0];
      const app = win?.app_name || "Unknown app";
      const title = win?.window_name || "Untitled window";
      const text = (win?.text || "").slice(0, 400).replace(/\s+/g, " ");
      return `- [${ts}] App: ${app} | Window: "${title}" | Text: ${text}`;
    })
    .join("\n");

  const prompt = `You are an expert workflow documenter. Derive the precise numbered steps from the following chronological captures. 
Return ONLY:
Title: <short descriptive title>
Steps:
1. ...
2. ...

Captures:
${context}`;

  const res = await fetch(`${MEMRI_API_URL}/assistant`, {
    method: "POST",
    headers,
    body: JSON.stringify({ prompt, model: "claude-sonnet-4-5", max_tokens: 800 }),
  });
  if (!res.ok) throw new Error("Assistant summary failed");
  const data = await res.json();
  const content: string = data?.content || data?.message?.content || "";

  // Naive parsing for Title and Steps
  const titleMatch = content.match(/Title:\s*(.+)/i);
  const stepsMatch = content.match(/Steps:\s*([\s\S]+)/i);

  return {
    raw: content.trim(),
    title: titleMatch ? titleMatch[1].trim() : "",
    steps: stepsMatch ? stepsMatch[1].trim() : content.trim(),
  };
}

