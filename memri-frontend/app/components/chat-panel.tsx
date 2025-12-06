"use client";

import { useRef, useEffect } from "react";
import { InputBar, Model } from "../input-bar";

export type ChatMessage = {
  id: number;
  role: string;
  content: string;
  created_at_ms: number;
};

type ChatPanelProps = {
  messages: ChatMessage[];
  streaming: boolean;
  model: Model;
  onModelChange: (model: Model) => void;
  onSend: (text: string, model: Model) => Promise<void>;
  disabled: boolean;
  width: number;
};

export function ChatPanel({
  messages,
  streaming,
  model,
  onModelChange,
  onSend,
  disabled,
  width,
}: ChatPanelProps) {
  const chatEndRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  return (
    <div
      className="flex flex-col border-l border-[var(--color-border)] bg-[var(--color-bg)]"
      style={{ width: `${width}px` }}
    >
      {/* Header */}
      <div className="flex h-12 flex-shrink-0 items-center justify-between border-b border-[var(--color-border)] px-4">
        <div className="flex items-center gap-2">
          <span className="text-sm font-semibold text-[var(--color-text)]">
            Assistant
          </span>
          {streaming && <TypingIndicator />}
        </div>
        <span className="text-[11px] text-[var(--color-text-tertiary)]">
          {messages.length} messages
        </span>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto px-4 py-4 smooth-scroll">
        <div className="flex flex-col gap-3">
          {messages.length === 0 && (
            <div className="flex flex-col items-center justify-center py-12 text-center">
              <div className="mb-3 rounded-full bg-[var(--color-bg-elevated)] p-4">
                <span className="text-2xl">✨</span>
              </div>
              <p className="text-sm font-medium text-[var(--color-text)]">
                Start a conversation
              </p>
              <p className="mt-1 text-xs text-[var(--color-text-tertiary)]">
                Ask about your screen captures
              </p>
            </div>
          )}

          {messages.map((m, idx) => (
            <ChatBubble
              key={`${m.id}-${m.created_at_ms}`}
              message={m}
              isCompact={idx > 0 && messages[idx - 1]?.role === m.role}
            />
          ))}
          <div ref={chatEndRef} />
        </div>
      </div>

      {/* Input */}
      <InputBar
        inline
        model={model}
        onModelChange={onModelChange}
        onSend={onSend}
        disabled={disabled}
      />
    </div>
  );
}

function TypingIndicator() {
  return (
    <div className="flex items-center gap-0.5">
      <div
        className="h-1 w-1 rounded-full bg-[var(--color-primary)] opacity-60"
        style={{ animation: "typing-bounce 1.4s ease-in-out infinite" }}
      />
      <div
        className="h-1 w-1 rounded-full bg-[var(--color-primary)] opacity-60"
        style={{ animation: "typing-bounce 1.4s ease-in-out 0.2s infinite" }}
      />
      <div
        className="h-1 w-1 rounded-full bg-[var(--color-primary)] opacity-60"
        style={{ animation: "typing-bounce 1.4s ease-in-out 0.4s infinite" }}
      />
    </div>
  );
}

type ChatBubbleProps = {
  message: ChatMessage;
  isCompact: boolean;
};

function ChatBubble({ message, isCompact }: ChatBubbleProps) {
  const isUser = message.role !== "assistant";

  return (
    <div
      className={`flex ${isUser ? "justify-end" : "justify-start"}`}
      style={{ marginTop: isCompact ? "4px" : undefined }}
    >
      <div
        className={`flex max-w-[85%] flex-col ${
          isUser ? "items-end" : "items-start"
        }`}
      >
        {/* Bubble */}
        <div
          className={`rounded-2xl px-3.5 py-2.5 text-[13px] leading-relaxed ${
            isUser
              ? "text-[var(--color-text)]"
              : "border border-[var(--color-border)] text-[var(--color-text)]"
          }`}
          style={{
            background: isUser
              ? "linear-gradient(135deg, #E6FFFE 0%, #F0EFFF 100%)"
              : "var(--color-bg)",
            borderRadius: isUser ? "18px 18px 4px 18px" : "18px 18px 18px 4px",
          }}
        >
          <p className="whitespace-pre-wrap">{message.content}</p>
        </div>

        {/* Timestamp */}
        {!isCompact && (
          <span className="mt-1 text-[10px] text-[var(--color-text-tertiary)]">
            {new Date(message.created_at_ms).toLocaleTimeString([], {
              hour: "2-digit",
              minute: "2-digit",
            })}
          </span>
        )}
      </div>
    </div>
  );
}

