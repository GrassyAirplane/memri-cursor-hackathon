"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Search, Sparkles, ChevronLeft, ChevronRight } from "lucide-react";
import { InputBar, Model } from "./input-bar";
import { MEMRI_API_KEY, MEMRI_API_URL } from "./constants";

type CaptureWindow = {
  window_name: string;
  app_name: string;
  text: string;
  confidence?: number | null;
  browser_url?: string | null;
  image_base64?: string | null;
};

type Capture = {
  capture_id: number;
  frame_number: number;
  timestamp_ms: number;
  windows: CaptureWindow[];
};

type ChatMessage = {
  id: number;
  role: string;
  content: string;
  created_at_ms: number;
};

export default function Home() {
  const [captures, setCaptures] = useState<Capture[]>([]);
  const [selectedCapture, setSelectedCapture] = useState<Capture | null>(null);
  const [chat, setChat] = useState<ChatMessage[]>([]);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(false);
  const [streaming, setStreaming] = useState(false);
  const [connected, setConnected] = useState(true);
  const [model, setModel] = useState<Model>({
    id: "claude-3-5-sonnet",
    name: "Claude 3.5 Sonnet",
    description: "Most capable model",
  });
  const chatEndRef = useRef<HTMLDivElement | null>(null);
  const [chatWidth, setChatWidth] = useState(420);
  const [isDragging, setIsDragging] = useState(false);

  const headers = useMemo(() => {
    const base: Record<string, string> = { "Content-Type": "application/json" };
    if (MEMRI_API_KEY) base["x-api-key"] = MEMRI_API_KEY;
    return base;
  }, []);

  const fetchCaptures = useCallback(async () => {
    const res = await fetch(`${MEMRI_API_URL}/captures?limit=50`, { headers });
    if (!res.ok) return;
    const data = (await res.json()) as Capture[];
    const sorted = [...data].sort((a, b) => a.timestamp_ms - b.timestamp_ms);
    setCaptures(sorted);
    if (!selectedCapture && sorted.length > 0) {
      setSelectedCapture(sorted[sorted.length - 1]);
    } else if (
      selectedCapture &&
      !sorted.some((cap) => cap.capture_id === selectedCapture.capture_id)
    ) {
      setSelectedCapture(sorted[sorted.length - 1] ?? null);
    }
  }, [headers, selectedCapture]);

  const fetchChat = useCallback(async () => {
    const res = await fetch(`${MEMRI_API_URL}/chat?limit=50`, { headers });
    if (!res.ok) return;
    const data = (await res.json()) as ChatMessage[];
    setChat(data.reverse());
  }, [headers]);

  const fetchData = useCallback(async () => {
    await Promise.all([fetchCaptures(), fetchChat()]);
  }, [fetchCaptures, fetchChat]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      if (!cancelled) {
        await fetchData();
      }
    })();
    const poll = setInterval(() => {
      if (!cancelled) {
        fetchData();
      }
    }, 30000);
    const es = new EventSource(`${MEMRI_API_URL}/events`, {
      withCredentials: false,
    });
    es.onopen = () => setConnected(true);
    es.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data.type === "capture") {
          if (!cancelled) fetchCaptures();
        } else if (data.type === "chat") {
          if (!cancelled) fetchChat();
        }
      } catch {
        // ignore malformed
      }
    };
    es.onerror = () => {
      es.close();
      setConnected(false);
    };
    return () => {
      cancelled = true;
      es.close();
      clearInterval(poll);
    };
  }, [fetchData, fetchCaptures, fetchChat]);

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [chat]);

  useEffect(() => {
    if (!isDragging) return;
    const onMove = (e: MouseEvent) => {
      const windowWidth = window.innerWidth;
      const newWidth = windowWidth - e.clientX;
      const min = 320;
      const max = 720;
      setChatWidth(Math.min(Math.max(newWidth, min), max));
    };
    const onUp = () => setIsDragging(false);
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    return () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
  }, [isDragging]);

  async function sendToAssistant(content: string, modelId?: string) {
    if (!content.trim()) return;
    setStreaming(true);
    const tempId = -Date.now();
    setChat((prev) => [
      ...prev,
      { id: tempId, role: "user", content, created_at_ms: Date.now() },
    ]);
    const res = await fetch(`${MEMRI_API_URL}/assistant/stream`, {
      method: "POST",
      headers,
      body: JSON.stringify({ prompt: content, model: modelId }),
    });
    if (!res.body) {
      setStreaming(false);
      return;
    }

    const reader = res.body.getReader();
    const decoder = new TextDecoder();
    let assistantText = "";

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      assistantText += decoder.decode(value, { stream: true });
      setChat((prev) => {
        const base = prev.filter((m) => m.id !== -9999);
        return [
          ...base,
          {
            id: -9999,
            role: "assistant",
            content: assistantText,
            created_at_ms: Date.now(),
          },
        ];
      });
    }

    setStreaming(false);
    await fetchChat();
  }

  const filteredCaptures = useMemo(() => {
    const term = search.trim().toLowerCase();
    const ordered = [...captures].sort((a, b) => a.timestamp_ms - b.timestamp_ms);
    if (!term) return ordered;
    return ordered.filter((cap) =>
      cap.windows.some((w) => {
        const haystack = `${w.window_name} ${w.app_name} ${w.text} ${w.browser_url || ""}`.toLowerCase();
        return haystack.includes(term);
      })
    );
  }, [captures, search]);

  const timeline = useMemo(() => {
    const source = filteredCaptures.length ? filteredCaptures : captures;
    const ordered = [...source].sort((a, b) => a.timestamp_ms - b.timestamp_ms);
    return ordered.slice(Math.max(ordered.length - 50, 0));
  }, [captures, filteredCaptures]);

  useEffect(() => {
    if (!timeline.length) return;
    const existsInTimeline = timeline.some((c) => c.capture_id === selectedCapture?.capture_id);
    if (!selectedCapture || !existsInTimeline) {
      setSelectedCapture(timeline[timeline.length - 1]);
    }
  }, [timeline, selectedCapture]);

  const selectedIndex = useMemo(() => {
    return timeline.findIndex((c) => c.capture_id === selectedCapture?.capture_id);
  }, [timeline, selectedCapture]);

  const goPrev = () => {
    if (selectedIndex > 0) {
      setSelectedCapture(timeline[selectedIndex - 1]);
    }
  };

  const goNext = () => {
    if (selectedIndex >= 0 && selectedIndex < timeline.length - 1) {
      setSelectedCapture(timeline[selectedIndex + 1]);
    }
  };

  const selectedWindow = selectedCapture?.windows?.[0];

  const handleSend = async (text: string, selectedModel: Model) => {
    await sendToAssistant(text, selectedModel.id);
  };

  return (
    <div className={`flex h-screen overflow-hidden bg-[var(--color-bg)] ${isDragging ? 'dragging' : ''}`}>
      {/* Main content area */}
      <div className="flex flex-1 flex-col" style={{ width: `calc(100% - ${chatWidth}px)` }}>
        {/* Top toolbar - Eden.so clean header */}
        <div className="flex h-14 items-center justify-between border-b border-[var(--color-border)] bg-[var(--color-bg)] px-6">
          <div className="flex items-center gap-4">
            <div className="flex items-center gap-2">
              <Sparkles className="h-5 w-5 text-[var(--color-primary)]" />
              <span className="text-base font-semibold text-[var(--color-text)]" style={{ letterSpacing: '-0.02em' }}>Memri</span>
            </div>
            <div className="h-5 w-px bg-[var(--color-border)]" />
            <div className="relative">
              <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-[var(--color-text-tertiary)]" />
              <input
                type="text"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder="Search captures..."
                className="h-9 w-64 rounded-[var(--radius-sm)] border border-[var(--color-border)] bg-[var(--color-bg)] pl-9 pr-3 text-sm text-[var(--color-text)] placeholder:text-[var(--color-text-tertiary)] transition-fast focus:border-[var(--color-primary)] focus:outline-none"
              />
            </div>
          </div>
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2 text-xs text-[var(--color-text-secondary)]">
              <div className={`h-2 w-2 rounded-full ${connected ? 'bg-[var(--color-success)]' : 'bg-[var(--color-warning)]'}`} />
              <span>{connected ? 'Connected' : 'Offline'}</span>
            </div>
          </div>
        </div>

        {/* Preview area */}
        <div className="flex flex-1 flex-col overflow-hidden">
          <div className="relative flex-1 bg-[var(--color-bg-elevated)]">
            {selectedWindow?.image_base64 ? (
              // eslint-disable-next-line @next/next/no-img-element
              <img
                src={`data:image/png;base64,${selectedWindow.image_base64}`}
                alt={selectedWindow.window_name}
                className="h-full w-full object-contain"
              />
            ) : (
              <div className="flex h-full items-center justify-center text-sm text-[var(--color-text-tertiary)]">
                No capture selected
              </div>
            )}

            {/* Preview overlay info - Eden.so subtle card */}
            {selectedWindow && (
              <div 
                className="absolute left-4 top-4 flex items-center gap-2 rounded-[var(--radius-sm)] border border-[var(--color-border)] bg-[var(--color-card-bg)]/95 px-3 py-2 backdrop-blur-sm"
                style={{
                  boxShadow: '0 1px 2px rgba(0,0,0,0.04)',
                }}
              >
                <div className="text-sm font-medium text-[var(--color-text)]">
                  {selectedWindow.window_name || 'Untitled'}
                </div>
                <div className="h-4 w-px bg-[var(--color-border)]" />
                <div className="text-xs text-[var(--color-text-secondary)]">
                  {selectedCapture ? new Date(selectedCapture.timestamp_ms).toLocaleTimeString() : ''}
                </div>
              </div>
            )}

            {/* Navigation buttons - Eden.so style */}
            {selectedCapture && (
              <>
                <button
                  onClick={goPrev}
                  disabled={selectedIndex <= 0}
                  className="button-press absolute left-4 top-1/2 -translate-y-1/2 rounded-[var(--radius-sm)] border border-[var(--color-border)] bg-[var(--color-card-bg)]/95 p-2 backdrop-blur-sm transition-fast hover:bg-[var(--color-hover)] disabled:opacity-30"
                  style={{
                    boxShadow: '0 1px 2px rgba(0,0,0,0.04)',
                  }}
                >
                  <ChevronLeft className="h-5 w-5" />
                </button>
                <button
                  onClick={goNext}
                  disabled={selectedIndex === timeline.length - 1}
                  className="button-press absolute right-4 top-1/2 -translate-y-1/2 rounded-[var(--radius-sm)] border border-[var(--color-border)] bg-[var(--color-card-bg)]/95 p-2 backdrop-blur-sm transition-fast hover:bg-[var(--color-hover)] disabled:opacity-30"
                  style={{
                    boxShadow: '0 1px 2px rgba(0,0,0,0.04)',
                  }}
                >
                  <ChevronRight className="h-5 w-5" />
                </button>
              </>
            )}

            {/* OCR text overlay */}
            {selectedWindow?.text && (
              <div 
                className="absolute bottom-4 left-4 right-4 max-h-32 overflow-y-auto rounded-[var(--radius-sm)] border border-[var(--color-border)] bg-[var(--color-card-bg)]/95 p-3 backdrop-blur-sm"
                style={{
                  boxShadow: '0 1px 2px rgba(0,0,0,0.04)',
                }}
              >
                <div className="text-xs text-[var(--color-text-secondary)] whitespace-pre-wrap leading-relaxed">
                  {selectedWindow.text}
                </div>
              </div>
            )}
          </div>

          {/* Timeline scrubber */}
          <div className="border-t border-[var(--color-border)] bg-[var(--color-bg)] px-6 py-4">
            <div className="mb-2 flex items-center justify-between text-xs text-[var(--color-text-secondary)]">
              <span>{timeline.length} captures</span>
              <span>
                {selectedCapture
                  ? `#${selectedIndex + 1} of ${timeline.length}`
                  : 'None selected'}
              </span>
            </div>
            <div className="flex gap-1 overflow-x-auto pb-2 smooth-scroll">
              {timeline.map((capture) => {
                const isActive = capture.capture_id === selectedCapture?.capture_id;
                return (
                  <button
                    key={capture.capture_id}
                    onClick={() => setSelectedCapture(capture)}
                    className={`group relative h-12 w-1.5 flex-shrink-0 rounded-sm transition-all ${
                      isActive
                        ? 'bg-[var(--color-primary)] scale-y-125'
                        : 'bg-[var(--color-border)] hover:bg-[var(--color-text-tertiary)] hover:scale-y-110'
                    }`}
                    title={`#${capture.frame_number} - ${new Date(capture.timestamp_ms).toLocaleString()}`}
                  >
                    <span className="sr-only">Frame {capture.frame_number}</span>
                  </button>
                );
              })}
            </div>
          </div>
        </div>
      </div>

      {/* Resize handle - Eden.so style */}
      <div
        className="relative w-1 cursor-col-resize bg-[var(--color-border)] hover:bg-[var(--color-primary)] transition-colors"
        onMouseDown={() => setIsDragging(true)}
      >
        <div className="absolute inset-y-0 -left-1 -right-1" />
      </div>

      {/* Chat panel - Eden.so message bubbles */}
      <div
        className="flex flex-col border-l border-[var(--color-border)] bg-[var(--color-bg)]"
        style={{ width: `${chatWidth}px` }}
      >
        {/* Chat header */}
        <div className="flex h-14 items-center justify-between border-b border-[var(--color-border)] bg-[var(--color-bg)] px-[var(--space-lg)]">
          <div className="flex items-center gap-2">
            <div className="text-sm font-semibold text-[var(--color-text)]">Chat</div>
            {streaming && (
              <div className="flex items-center gap-1.5">
                <div className="flex gap-1">
                  <div className="typing-dot h-1.5 w-1.5 rounded-full bg-[var(--color-text-tertiary)]" />
                  <div className="typing-dot h-1.5 w-1.5 rounded-full bg-[var(--color-text-tertiary)]" />
                  <div className="typing-dot h-1.5 w-1.5 rounded-full bg-[var(--color-text-tertiary)]" />
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Chat messages - Eden.so bubble style */}
        <div className="flex-1 overflow-y-auto px-[var(--space-md)] py-[var(--space-lg)] smooth-scroll">
          <div className="flex flex-col gap-[var(--space-md)]">
            {chat.map((m, idx) => {
              const isUser = m.role !== 'assistant';
              const prevMessage = idx > 0 ? chat[idx - 1] : null;
              const isSameSender = prevMessage && prevMessage.role === m.role;
              const marginTop = isSameSender ? 'var(--space-xs)' : 'var(--space-md)';
              
              return (
                <div
                  key={`${m.id}-${m.created_at_ms}`}
                  className={`message-enter flex ${isUser ? 'justify-end' : 'justify-start'}`}
                  style={{ marginTop: idx === 0 ? '0' : marginTop }}
                >
                  <div className={`flex max-w-[70%] flex-col ${isUser ? 'items-end' : 'items-start'}`}>
                    {/* Eden.so Message Bubble */}
                    <div
                      className={`rounded-[var(--radius-lg)] px-4 py-3 text-sm leading-relaxed ${
                        isUser
                          ? 'text-[var(--color-text)]'
                          : 'border border-[var(--color-border)] bg-[var(--message-bubble-ai-bg)] text-[var(--color-text)]'
                      }`}
                      style={{
                        background: isUser ? 'linear-gradient(135deg, #E6FFFE 0%, #F0EFFF 100%)' : undefined,
                        boxShadow: '0 1px 2px rgba(0,0,0,0.04)',
                        borderRadius: isUser
                          ? 'var(--radius-lg) var(--radius-lg) 4px var(--radius-lg)'
                          : 'var(--radius-lg) var(--radius-lg) var(--radius-lg) 4px',
                      }}
                    >
                      <div className="whitespace-pre-wrap">{m.content}</div>
                    </div>
                    {/* Eden.so Timestamp */}
                    <div 
                      className={`mt-1 flex items-center gap-1 text-[11px] font-medium text-[var(--color-text-tertiary)] ${isUser ? 'flex-row-reverse' : 'flex-row'}`}
                      style={{ letterSpacing: '0em' }}
                    >
                      <span>{new Date(m.created_at_ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</span>
                    </div>
                  </div>
                </div>
              );
            })}
            <div ref={chatEndRef} />
          </div>
        </div>

        {/* Input bar - Eden.so style */}
        <InputBar
          inline
          model={model}
          onModelChange={setModel}
          onSend={handleSend}
          disabled={loading || streaming}
        />
      </div>
    </div>
  );
}
