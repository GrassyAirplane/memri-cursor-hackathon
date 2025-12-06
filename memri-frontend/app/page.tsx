"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Search, Sparkles, ChevronLeft, ChevronRight, Star } from "lucide-react";
import { InputBar, Model } from "./input-bar";
import { Timeline, type CaptureNode } from "./timeline";
import { type Capture, type ChatMessage, ClipReference, parseClipReferences, type ClipData } from "./components";
import { MEMRI_API_KEY, MEMRI_API_URL } from "./constants";

// Initial welcome messages from the assistant (use fixed timestamp to avoid hydration mismatch)
const getWelcomeMessages = (): ChatMessage[] => [
  {
    id: -1,
    role: "assistant",
    content: "👋 Hey there! I'm your personal memory assistant. I've been watching your screen and can help you recall anything you've seen or done.",
    created_at_ms: 0, // Will be hidden in UI for welcome messages
  },
  {
    id: -2,
    role: "assistant", 
    content: "Here's what I can help with:\n\n• 🎬 \"What YouTube videos did I watch today?\"\n• 💻 \"When was I last in VS Code?\"\n• 💬 \"Find my Slack messages from this morning\"\n• 🔍 \"What was I researching yesterday?\"\n\nJust ask naturally and I'll find the relevant moments!",
    created_at_ms: 0, // Will be hidden in UI for welcome messages
  },
];

export default function Home() {
  const [captures, setCaptures] = useState<Capture[]>([]);
  const [selectedCapture, setSelectedCapture] = useState<Capture | null>(null);
  const [chat, setChat] = useState<ChatMessage[]>(getWelcomeMessages);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(false);
  const [streaming, setStreaming] = useState(false);
  const [connected, setConnected] = useState(true);
  const [model, setModel] = useState<Model>({
    id: "claude-sonnet-4-5",
    name: "Claude Sonnet 4.5",
    description: "Most capable model",
  });
  const chatEndRef = useRef<HTMLDivElement | null>(null);
  const [chatWidth, setChatWidth] = useState(420);
  const [isDragging, setIsDragging] = useState(false);
  const [imageCache, setImageCache] = useState<Record<number, string>>({});
  const [loadingImages, setLoadingImages] = useState<Set<number>>(new Set());
  const pendingFetchesRef = useRef<Set<number>>(new Set());

  const headers = useMemo(() => {
    const base: Record<string, string> = { "Content-Type": "application/json" };
    if (MEMRI_API_KEY) base["x-api-key"] = MEMRI_API_KEY;
    return base;
  }, []);

  // Simple bold renderer: turns **text** into <strong>text</strong>
  const renderRichText = (text: string) => {
    const parts = text.split(/(\*\*[^*]+\*\*)/g);
    return parts.map((part, idx) => {
      const isBold = part.startsWith("**") && part.endsWith("**") && part.length > 4;
      if (isBold) {
        return (
          <strong key={`b-${idx}`} className="font-semibold">
            {part.slice(2, -2)}
          </strong>
        );
      }
      return <span key={`t-${idx}`}>{part}</span>;
    });
  };

  // Fetch images for specific capture IDs (with caching)
  const fetchImages = useCallback(async (captureIds: number[]) => {
    // Filter out IDs that are already being fetched
    const toFetch = captureIds.filter((id) => !pendingFetchesRef.current.has(id));
    if (toFetch.length === 0) return;
    
    // Mark as pending
    toFetch.forEach((id) => pendingFetchesRef.current.add(id));
    
    // Mark as loading in state
    setLoadingImages((prev) => {
      const next = new Set(prev);
      toFetch.forEach((id) => next.add(id));
      return next;
    });

    try {
      const res = await fetch(
        `${MEMRI_API_URL}/captures/images?ids=${toFetch.join(",")}`,
        { headers }
      );
      if (!res.ok) {
        console.error("Failed to fetch images:", res.status);
        return;
      }
      const data = (await res.json()) as Record<string, string>;
      
      // Update cache
      setImageCache((prev) => {
        const updated = { ...prev };
        for (const [id, base64] of Object.entries(data)) {
          updated[parseInt(id, 10)] = base64;
        }
        return updated;
      });
    } catch (err) {
      console.error("Error fetching images:", err);
    } finally {
      // Remove from pending and loading
      toFetch.forEach((id) => pendingFetchesRef.current.delete(id));
      setLoadingImages((prev) => {
        const next = new Set(prev);
        toFetch.forEach((id) => next.delete(id));
        return next;
      });
    }
  }, [headers]);

  const fetchCaptures = useCallback(async () => {
    const res = await fetch(`${MEMRI_API_URL}/captures`, { headers });
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

  // Track when we last finished streaming to prevent immediate overwrites
  const lastStreamEndRef = useRef<number>(0);
  
  const fetchChat = useCallback(async () => {
    // Don't fetch while streaming - it could overwrite the local message
    if (streaming) return;
    
    // Don't fetch for 5 seconds after streaming ends to let backend save
    const timeSinceStreamEnd = Date.now() - lastStreamEndRef.current;
    if (lastStreamEndRef.current > 0 && timeSinceStreamEnd < 5000) {
      return;
    }
    
    const res = await fetch(`${MEMRI_API_URL}/chat?limit=50`, { headers });
    if (!res.ok) return;
    const data = (await res.json()) as ChatMessage[];
    
    // Prepend welcome messages if no real messages exist
    if (data.length === 0) {
      setChat(getWelcomeMessages());
    } else {
      setChat((prevChat) => {
        // DB returns newest first, so reverse to get oldest first (chronological order)
        const dbMessages = [...data].reverse();
        
        // Check if we have local messages (negative ID) that might not be in DB yet
        const localMessages = prevChat.filter(m => m.id < 0 && m.created_at_ms > 0);
        
        if (localMessages.length > 0) {
          // Find the most recent assistant message in local state
          const localAssistant = localMessages.find(m => m.role === 'assistant');
          
          if (localAssistant) {
            // Check if DB has this message (compare by content length)
            const lastDbAssistant = dbMessages.filter(m => m.role === 'assistant').pop();
            
            // If local has more content, keep it instead of DB version
            if (!lastDbAssistant || localAssistant.content.length > lastDbAssistant.content.length) {
              // Remove the incomplete DB version if it exists, add local version
              const filtered = dbMessages.filter(m => 
                !(m.role === 'assistant' && lastDbAssistant && m.id === lastDbAssistant.id)
              );
              const merged = [...filtered, localAssistant];
              // Sort by created_at_ms to maintain chronological order
              return merged.sort((a, b) => a.created_at_ms - b.created_at_ms);
            }
          }
        }
        
        // Return DB messages sorted chronologically (oldest first)
        return dbMessages.sort((a, b) => a.created_at_ms - b.created_at_ms);
      });
    }
  }, [headers, streaming]);

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

    let sseBuffer = ""; // Buffer for handling partial SSE lines
    
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      
      // Parse SSE format - extract data after "data: " prefix
      const rawChunk = decoder.decode(value, { stream: true });
      sseBuffer += rawChunk;
      
      // Process complete lines only
      const lines = sseBuffer.split('\n');
      sseBuffer = lines.pop() || ""; // Keep incomplete last line in buffer
      
      for (const line of lines) {
        if (line.startsWith('data: ')) {
          const textData = line.slice(6); // Remove "data: " prefix
          if (textData && textData !== '[DONE]') {
            assistantText += textData;
          }
        }
      }
      
      if (assistantText) {
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
    }
    
    // Process any remaining buffer
    if (sseBuffer.startsWith('data: ')) {
      const textData = sseBuffer.slice(6);
      if (textData && textData !== '[DONE]') {
        assistantText += textData;
      }
    }
    
    // Final update with complete message
    const finalContent = assistantText;
    setChat((prev) => {
      const base = prev.filter((m) => m.id !== -9999);
      return [
        ...base,
        {
          id: -Date.now(), // Use negative timestamp as temp ID (will be replaced on next fetch)
          role: "assistant",
          content: finalContent,
          created_at_ms: Date.now(),
        },
      ];
    });

    setStreaming(false);
    lastStreamEndRef.current = Date.now(); // Prevent fetchChat from overwriting for 3 seconds
    
    // Don't fetch immediately - just let the message stay as-is
    // The periodic refresh (if any) will sync later
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
    return [...source].sort((a, b) => a.timestamp_ms - b.timestamp_ms);
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

  // Fetch images for selected capture and ±4 surrounding captures
  useEffect(() => {
    if (selectedIndex < 0 || timeline.length === 0) return;
    
    const start = Math.max(0, selectedIndex - 4);
    const end = Math.min(timeline.length - 1, selectedIndex + 4);
    const idsToFetch: number[] = [];
    
    for (let i = start; i <= end; i++) {
      idsToFetch.push(timeline[i].capture_id);
    }
    
    fetchImages(idsToFetch);
  }, [selectedIndex, timeline, fetchImages]);

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
  
  // Get the cached image for the selected capture
  const selectedImage = selectedCapture 
    ? imageCache[selectedCapture.capture_id] 
    : null;
  const isImageLoading = selectedCapture 
    ? loadingImages.has(selectedCapture.capture_id) 
    : false;

  const handleSend = async (text: string, selectedModel: Model) => {
    await sendToAssistant(text, selectedModel.id);
  };

  // Navigate to a specific capture from a clip reference
  const handleClipClick = (clip: ClipData) => {
    const capture = captures.find((c) => c.capture_id === clip.capture_id);
    if (capture) {
      setSelectedCapture(capture);
    } else {
      // Try to find by timestamp if capture_id doesn't match
      const byTimestamp = captures.find((c) => c.timestamp_ms === clip.timestamp_ms);
      if (byTimestamp) {
        setSelectedCapture(byTimestamp);
      }
    }
  };

  // Convert captures to timeline format
  const timelineNodes: CaptureNode[] = useMemo(() => {
    return timeline.map((cap) => ({
      id: `${cap.capture_id}`,
      capture_id: cap.capture_id,
      frame_number: cap.frame_number,
      timestamp_ms: cap.timestamp_ms,
      thumbnailUrl: cap.windows?.[0]?.image_base64
        ? `data:image/png;base64,${cap.windows[0].image_base64}`
        : undefined,
      metadata: {
        applicationName: cap.windows?.[0]?.app_name,
        windowTitle: cap.windows?.[0]?.window_name,
      },
    }));
  }, [timeline]);

  const handleTimelineSelect = (node: CaptureNode) => {
    const capture = timeline.find((c) => c.capture_id === node.capture_id);
    if (capture) {
      setSelectedCapture(capture);
    }
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
                placeholder="Search captures... (e.g YouTube)"
                className="h-9 w-64 rounded-[var(--radius-sm)] border border-[var(--color-border)] bg-[var(--color-bg)] pl-9 pr-3 text-sm text-[var(--color-text)] placeholder:text-[var(--color-text-tertiary)] transition-fast focus:border-[var(--color-primary)] focus:outline-none"
              />
            </div>
          </div>
          <div className="flex items-center gap-3">
            <a
              href="/workflows"
              className="inline-flex items-center gap-2 rounded-md border border-[var(--color-border)] bg-[var(--color-bg-elevated)] px-3 py-1.5 text-xs font-medium text-[var(--color-text)] transition-all hover:border-[var(--color-primary)] hover:bg-[var(--color-hover)]"
            >
              <Star className="h-3.5 w-3.5 text-red-500" />
              Workflows
            </a>
            <div className="flex items-center gap-2 text-xs text-[var(--color-text-secondary)]">
              <div className={`h-2 w-2 rounded-full ${connected ? 'bg-[var(--color-success)]' : 'bg-[var(--color-warning)]'}`} />
              <span>{connected ? 'Connected' : 'Offline'}</span>
            </div>
          </div>
        </div>

        {/* Preview area */}
        <div className="flex flex-1 flex-col" style={{ minHeight: 0 }}>
          {/* Info bar - static, pushes content down */}
          {selectedWindow && (
            <div className="flex flex-shrink-0 items-center justify-between border-b border-[var(--color-border)] bg-[var(--color-bg)] px-4 py-2">
              <div className="flex items-center gap-3">
                <div className="flex items-center gap-2">
                  <div className="h-2 w-2 rounded-full bg-[var(--color-primary)]" />
                  <span className="text-sm font-medium text-[var(--color-text)]">
                    {selectedWindow.app_name || 'Unknown App'}
                  </span>
                </div>
                <div className="h-4 w-px bg-[var(--color-border)]" />
                <span className="max-w-[400px] truncate text-sm text-[var(--color-text-secondary)]">
                  {selectedWindow.window_name || 'Untitled'}
                </span>
              </div>
              <div className="flex items-center gap-3">
                <span className="text-xs text-[var(--color-text-tertiary)]">
                  {selectedCapture ? new Date(selectedCapture.timestamp_ms).toLocaleTimeString([], {
                    hour: '2-digit',
                    minute: '2-digit',
                    second: '2-digit',
                  }) : ''}
                </span>
                <div className="h-4 w-px bg-[var(--color-border)]" />
                <span className="text-xs text-[var(--color-text-tertiary)]">
                  {selectedIndex + 1} / {timeline.length}
                </span>
              </div>
            </div>
          )}

          {/* Image container */}
          <div 
            className="relative flex flex-1 items-center justify-center overflow-hidden bg-[var(--color-bg-elevated)] p-6"
          >
            {isImageLoading ? (
              <div className="flex flex-col items-center justify-center gap-3">
                <div className="h-8 w-8 animate-spin rounded-full border-2 border-[var(--color-border)] border-t-[var(--color-primary)]" />
                <span className="text-sm text-[var(--color-text-tertiary)]">Loading capture...</span>
              </div>
            ) : selectedImage ? (
              // eslint-disable-next-line @next/next/no-img-element
              <img
                src={`data:image/png;base64,${selectedImage}`}
                alt={selectedWindow?.window_name || "Screen capture"}
                className="rounded-[var(--radius-sm)] object-contain"
                style={{
                  boxShadow: '0 4px 20px rgba(0, 0, 0, 0.1)',
                  maxHeight: '100%',
                  maxWidth: '100%',
                }}
              />
            ) : (
              <div className="flex h-full w-full items-center justify-center text-sm text-[var(--color-text-tertiary)]">
                No capture selected
              </div>
            )}

            {/* Navigation buttons */}
            {selectedCapture && (
              <>
                <button
                  onClick={goPrev}
                  disabled={selectedIndex <= 0}
                  className="button-press absolute left-4 top-1/2 -translate-y-1/2 rounded-full border border-[var(--color-border)] bg-[var(--color-card-bg)] p-2.5 transition-all hover:border-[var(--color-primary)] hover:bg-[var(--color-hover)] disabled:opacity-30"
                  style={{
                    boxShadow: '0 2px 8px rgba(0,0,0,0.08)',
                  }}
                >
                  <ChevronLeft className="h-4 w-4" />
                </button>
                <button
                  onClick={goNext}
                  disabled={selectedIndex === timeline.length - 1}
                  className="button-press absolute right-4 top-1/2 -translate-y-1/2 rounded-full border border-[var(--color-border)] bg-[var(--color-card-bg)] p-2.5 transition-all hover:border-[var(--color-primary)] hover:bg-[var(--color-hover)] disabled:opacity-30"
                  style={{
                    boxShadow: '0 2px 8px rgba(0,0,0,0.08)',
                  }}
                >
                  <ChevronRight className="h-4 w-4" />
                </button>
              </>
            )}
          </div>

          {/* OCR text panel - static at bottom of preview */}
          {selectedWindow?.text && (
            <div className="max-h-20 flex-shrink-0 overflow-y-auto border-t border-[var(--color-border)] bg-[var(--color-bg)] px-4 py-2">
              <p className="font-mono text-[11px] leading-relaxed text-[var(--color-text-secondary)]">
                {selectedWindow.text}
              </p>
            </div>
          )}

          {/* Timeline scrubber - at absolute bottom */}
          <div className="flex-shrink-0 border-t border-[var(--color-border)] bg-[var(--color-bg)]">
            <Timeline
              captures={timelineNodes}
              selectedId={selectedCapture ? `${selectedCapture.capture_id}` : null}
              onSelect={handleTimelineSelect}
            />
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
        <div className="flex-1 overflow-y-auto overflow-x-hidden px-[var(--space-md)] py-[var(--space-lg)] smooth-scroll">
          <div className="flex flex-col gap-[var(--space-md)]">
            {chat.map((m, idx) => {
              const isUser = m.role !== 'assistant';
              const prevMessage = idx > 0 ? chat[idx - 1] : null;
              const isSameSender = prevMessage && prevMessage.role === m.role;
              const marginTop = isSameSender ? 'var(--space-xs)' : 'var(--space-md)';
              
              // Build captures lookup for clip reference parsing
              const capturesLookup = new Map(
                captures.map((c) => [
                  c.capture_id,
                  {
                    timestamp_ms: c.timestamp_ms,
                    app_name: c.windows?.[0]?.app_name,
                    window_name: c.windows?.[0]?.window_name,
                  },
                ])
              );
              
              // Parse clip references from assistant messages
              const { segments } = isUser 
                ? { segments: [{ type: "text" as const, content: m.content }] } 
                : parseClipReferences(m.content, capturesLookup);
              
              return (
                <div
                  key={`${m.id}-${m.created_at_ms}`}
                  className={`message-enter flex ${isUser ? 'justify-end' : 'justify-start'}`}
                  style={{ marginTop: idx === 0 ? '0' : marginTop }}
                >
                  <div className={`flex max-w-[85%] flex-col ${isUser ? 'items-end' : 'items-start'}`}>
                    {/* Message Bubble */}
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
                      {/* Render message with inline clip references */}
                      <div className="whitespace-pre-wrap">
                        {segments.map((segment, segIdx) => {
                          if (segment.type === "text") {
                            return (
                              <span key={segIdx} className="whitespace-pre-wrap">
                                {renderRichText(segment.content)}
                              </span>
                            );
                          } else {
                            return (
                              <ClipReference
                                key={`clip-${segment.clip.capture_id}-${segIdx}`}
                                clip={segment.clip}
                                onClick={() => handleClipClick(segment.clip)}
                              />
                            );
                          }
                        })}
                      </div>
                    </div>
                    {/* Timestamp - hide for welcome messages (created_at_ms === 0) */}
                    {m.created_at_ms > 0 && (
                      <div 
                        className={`mt-1 flex items-center gap-1 text-[11px] font-medium text-[var(--color-text-tertiary)] ${isUser ? 'flex-row-reverse' : 'flex-row'}`}
                        style={{ letterSpacing: '0em' }}
                      >
                        <span>{new Date(m.created_at_ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</span>
                      </div>
                    )}
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
