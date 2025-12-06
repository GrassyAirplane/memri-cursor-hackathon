"use client";

import { useEffect, useMemo, useRef, useState } from "react";

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

const API_URL = process.env.NEXT_PUBLIC_MEMRI_API_URL || "http://127.0.0.1:8080";
const API_KEY = process.env.NEXT_PUBLIC_MEMRI_API_KEY || "";

export default function Home() {
  const [captures, setCaptures] = useState<Capture[]>([]);
  const [chat, setChat] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [streaming, setStreaming] = useState(false);
  const chatEndRef = useRef<HTMLDivElement | null>(null);

  const headers = useMemo(
    () =>
      API_KEY
        ? {
            "x-api-key": API_KEY,
            "Content-Type": "application/json",
          }
        : { "Content-Type": "application/json" },
    []
  );

  useEffect(() => {
    fetchData();
    const es = new EventSource(`${API_URL}/events`, {
      withCredentials: false,
    });
    es.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data.type === "capture") {
          fetchCaptures();
        } else if (data.type === "chat") {
          fetchChat();
        }
      } catch {
        // ignore malformed
      }
    };
    es.onerror = () => {
      es.close();
    };
    return () => es.close();
  }, []);

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [chat]);

  async function fetchData() {
    await Promise.all([fetchCaptures(), fetchChat()]);
  }

  async function fetchCaptures() {
    const res = await fetch(`${API_URL}/captures?limit=12`, { headers });
    if (!res.ok) return;
    const data = (await res.json()) as Capture[];
    setCaptures(data);
  }

  async function fetchChat() {
    const res = await fetch(`${API_URL}/chat?limit=50`, { headers });
    if (!res.ok) return;
    const data = (await res.json()) as ChatMessage[];
    setChat(data.reverse());
  }

  async function sendMessage() {
    if (!input.trim()) return;
    setLoading(true);
    await fetch(`${API_URL}/chat`, {
      method: "POST",
      headers,
      body: JSON.stringify({ role: "user", content: input }),
    });
    setInput("");
    await fetchChat();
    setLoading(false);
  }

  async function sendToAssistant() {
    if (!input.trim()) return;
    setStreaming(true);
    const res = await fetch(`${API_URL}/assistant/stream`, {
      method: "POST",
      headers,
      body: JSON.stringify({ prompt: input }),
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

  return (
    <div className="min-h-screen bg-slate-50 text-slate-900">
      <header className="border-b bg-white px-6 py-4 shadow-sm">
        <div className="mx-auto flex max-w-6xl items-center justify-between">
          <div>
            <div className="text-xs uppercase tracking-wide text-slate-500">
              Memri Vision
            </div>
            <div className="text-lg font-semibold">Screen + OCR + Chat</div>
          </div>
          <div className="text-xs text-slate-500">
            Backend: {API_URL.replace(/^https?:\/\//, "")}
          </div>
        </div>
      </header>
      <main className="mx-auto flex max-w-6xl flex-col gap-6 px-6 py-6 lg:flex-row">
        <section className="flex-1 rounded-2xl bg-white p-4 shadow-sm">
          <div className="mb-3 flex items-center justify-between">
            <div>
              <div className="text-xs uppercase tracking-wide text-slate-500">
                Chat
              </div>
              <div className="text-base font-semibold">Assistant + User</div>
            </div>
            {loading || streaming ? (
              <div className="text-xs text-blue-600">Sending...</div>
            ) : null}
          </div>
          <div className="h-[420px] overflow-y-auto rounded-xl border bg-slate-50 p-3">
            {chat.map((m) => (
              <div
                key={`${m.id}-${m.created_at_ms}`}
                className={`mb-3 rounded-lg p-3 ${
                  m.role === "assistant"
                    ? "bg-blue-50 border border-blue-100"
                    : "bg-white border"
                }`}
              >
                <div className="text-xs uppercase tracking-wide text-slate-500">
                  {m.role}
                </div>
                <div className="whitespace-pre-wrap text-sm leading-6 text-slate-800">
                  {m.content}
                </div>
              </div>
            ))}
            <div ref={chatEndRef} />
          </div>
          <div className="mt-3 flex gap-2">
            <textarea
              className="flex-1 rounded-lg border px-3 py-2 text-sm shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              rows={2}
              placeholder="Ask about recent screen context..."
              value={input}
              onChange={(e) => setInput(e.target.value)}
            />
            <div className="flex flex-col gap-2">
              <button
                className="rounded-lg bg-slate-900 px-4 py-2 text-sm font-semibold text-white shadow-sm hover:bg-slate-800 disabled:opacity-50"
                onClick={sendMessage}
                disabled={loading || streaming}
              >
                Save
              </button>
              <button
                className="rounded-lg bg-blue-600 px-4 py-2 text-sm font-semibold text-white shadow-sm hover:bg-blue-500 disabled:opacity-50"
                onClick={sendToAssistant}
                disabled={loading || streaming}
              >
                Ask Claude
              </button>
            </div>
          </div>
        </section>

        <section className="flex-1 rounded-2xl bg-white p-4 shadow-sm">
          <div className="mb-3 flex items-center justify-between">
            <div>
              <div className="text-xs uppercase tracking-wide text-slate-500">
                Captures
              </div>
              <div className="text-base font-semibold">Recent frames</div>
            </div>
          </div>
          <div className="grid gap-3 md:grid-cols-2">
            {captures.map((capture) => {
              const first = capture.windows[0];
              return (
                <div
                  key={capture.capture_id}
                  className="rounded-xl border bg-slate-50 p-3 shadow-sm"
                >
                  <div className="flex items-center justify-between text-xs text-slate-500">
                    <span>Frame #{capture.frame_number}</span>
                    <span>{new Date(capture.timestamp_ms).toLocaleTimeString()}</span>
                  </div>
                  {first?.image_base64 ? (
                    // eslint-disable-next-line @next/next/no-img-element
                    <img
                      src={`data:image/png;base64,${first.image_base64}`}
                      alt={first.window_name}
                      className="mt-2 h-36 w-full rounded-lg object-cover"
                    />
                  ) : (
                    <div className="mt-2 flex h-36 items-center justify-center rounded-lg bg-white text-xs text-slate-400">
                      No image
                    </div>
                  )}
                  <div className="mt-2 text-sm font-semibold text-slate-800">
                    {first?.window_name || "Unknown"}
                  </div>
                  <div className="text-xs text-slate-500">{first?.app_name}</div>
                  {first?.browser_url ? (
                    <a
                      className="text-xs text-blue-600 underline"
                      href={first.browser_url}
                      target="_blank"
                      rel="noreferrer"
                    >
                      {first.browser_url}
                    </a>
                  ) : null}
                  {first?.text ? (
                    <div className="mt-2 line-clamp-3 text-xs text-slate-700">
                      {first.text}
                    </div>
                  ) : null}
                </div>
              );
            })}
          </div>
        </section>
      </main>
    </div>
  );
}
