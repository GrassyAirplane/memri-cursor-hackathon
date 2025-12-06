"use client";

import React, {
  KeyboardEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  Check,
  ChevronDown,
  Send,
  Sparkles,
  Paperclip,
  Wand2,
} from "lucide-react";

export type Model = {
  id: string;
  name: string;
  description: string;
  icon?: React.ReactNode;
};

type InputBarProps = {
  model: Model;
  models?: Model[];
  onModelChange: (m: Model) => void;
  onSend: (text: string, model: Model) => void;
  disabled?: boolean;
  inline?: boolean;
  className?: string;
};

export function InputBar({
  model,
  models,
  onModelChange,
  onSend,
  disabled = false,
  inline = false,
  className = "",
}: InputBarProps) {
  const [text, setText] = useState("");
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  const modelOptions = useMemo<Model[]>(
    () =>
      models ??
      [
        {
          id: "claude-3-5-sonnet",
          name: "Claude 3.5 Sonnet",
          description: "Most capable model",
          icon: <Sparkles className="h-4 w-4 text-[var(--color-secondary)]" />,
        },
        {
          id: "gpt-4-turbo",
          name: "GPT-4 Turbo",
          description: "Fast and capable",
          icon: <Wand2 className="h-4 w-4 text-[var(--color-primary)]" />,
        },
      ],
    [models],
  );

  // Close dropdown on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (!containerRef.current) return;
      if (!containerRef.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  // Ctrl/Cmd + M toggles model selector
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "m") {
        e.preventDefault();
        setOpen((v) => !v);
      }
    };
    window.addEventListener("keydown", handler as any);
    return () => window.removeEventListener("keydown", handler as any);
  }, []);

  // Auto-resize the textarea (Eden.so spec: min 52px, max 200px)
  useEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    const max = 200;
    const next = Math.min(el.scrollHeight, max);
    el.style.height = `${next}px`;
    el.style.overflowY = el.scrollHeight > max ? "auto" : "hidden";
  }, [text]);

  const handleSend = useCallback(() => {
    if (!text.trim() || disabled) return;
    onSend(text.trim(), model);
    setText("");
    requestAnimationFrame(() => textareaRef.current?.focus());
  }, [text, model, onSend, disabled]);

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const onSelectModel = (m: Model) => {
    onModelChange(m);
    setOpen(false);
    textareaRef.current?.focus();
  };

  const hasText = text.trim().length > 0;
  const sendDisabled = disabled || !hasText;

  return (
    <div 
      ref={containerRef} 
      className={`flex flex-col gap-3 border-t border-[var(--color-border)] bg-[var(--color-bg)] px-[var(--space-lg)] py-[var(--space-md)] ${className}`}
      style={{
        boxShadow: inline ? 'none' : '0 -1px 3px rgba(0,0,0,0.04)',
      }}
    >
      {/* Top row: attachment button and model selector */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-[var(--space-sm)]">
          <IconButton
            Icon={Paperclip}
            onClick={() => {}}
            label="Attach file"
          />
        </div>
        
        <div className="relative">
          <button
            onClick={() => setOpen((v) => !v)}
            className="flex items-center gap-2 rounded-[var(--radius-sm)] border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-1.5 text-xs font-medium text-[var(--color-text-secondary)] transition-fast hover:border-[var(--color-primary)] hover:text-[var(--color-text)]"
          >
            <span>{model.name}</span>
            <ChevronDown className="h-3.5 w-3.5" />
          </button>
          {open && (
            <div 
              className="absolute bottom-full right-0 mb-2 w-64 overflow-hidden rounded-[var(--radius-sm)] border border-[var(--color-border)] bg-[var(--color-card-bg)]"
              style={{
                boxShadow: '0 2px 8px rgba(0,0,0,0.08)',
              }}
            >
              <div className="max-h-80 overflow-y-auto">
                {modelOptions.map((m) => (
                  <button
                    key={m.id}
                    onClick={() => onSelectModel(m)}
                    className="flex w-full items-start gap-3 px-3 py-2.5 text-left transition-fast hover:bg-[var(--color-hover)]"
                  >
                    <div className="mt-0.5">
                      {m.icon ?? <Sparkles className="h-4 w-4" />}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium text-[var(--color-text)]">
                          {m.name}
                        </span>
                        {m.id === model.id && (
                          <Check className="h-4 w-4 text-[var(--color-primary)]" />
                        )}
                      </div>
                      <div className="text-xs text-[var(--color-text-secondary)]">
                        {m.description}
                      </div>
                    </div>
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Eden.so Input area - with cyan focus ring */}
      <div className="relative flex items-end gap-2">
        <textarea
          ref={textareaRef}
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder="Type a message..."
          rows={1}
          disabled={disabled}
          className="min-h-[52px] flex-1 resize-none rounded-[var(--radius-md)] border border-[var(--color-border)] bg-[var(--color-bg)] px-4 py-[14px] pr-12 text-sm font-normal leading-5 text-[var(--color-text)] placeholder:text-[var(--color-text-tertiary)] transition-fast hover:border-[var(--color-primary)] focus:border-[var(--color-primary)] focus:outline-none disabled:opacity-50"
          style={{ 
            maxHeight: 200,
            caretColor: 'var(--color-primary)',
          }}
        />
        <button
          onClick={handleSend}
          disabled={sendDisabled}
          className="button-press absolute bottom-[10px] right-[10px] flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-[var(--radius-sm)] transition-fast disabled:cursor-not-allowed"
          style={{
            backgroundColor: sendDisabled ? 'transparent' : 'var(--color-primary)',
            color: sendDisabled ? 'var(--color-text-quaternary)' : '#FFFFFF',
          }}
          onMouseEnter={(e) => {
            if (!sendDisabled) {
              e.currentTarget.style.backgroundColor = '#E6FFFE';
              e.currentTarget.style.color = 'var(--color-primary)';
            }
          }}
          onMouseLeave={(e) => {
            if (!sendDisabled) {
              e.currentTarget.style.backgroundColor = 'var(--color-primary)';
              e.currentTarget.style.color = '#FFFFFF';
            }
          }}
        >
          <Send className="h-4 w-4" />
        </button>
      </div>

      {/* Helper text */}
      <div className="flex items-center justify-between text-[11px] font-medium text-[var(--color-text-tertiary)]" style={{ letterSpacing: '0.05em' }}>
        <span>Press Enter to send, Shift+Enter for new line</span>
        <span>⌘M to change model</span>
      </div>
    </div>
  );
}

type IconButtonProps = {
  Icon: React.ComponentType<React.SVGProps<SVGSVGElement>>;
  onClick?: () => void;
  label: string;
};

function IconButton({ Icon, onClick, label }: IconButtonProps) {
  return (
    <button
      onClick={onClick}
      title={label}
      aria-label={label}
      className="flex h-7 w-7 items-center justify-center rounded-[var(--radius-sm)] text-[var(--color-text-secondary)] transition-fast hover:text-[var(--color-primary)]"
    >
      <Icon className="h-[14px] w-[14px]" />
    </button>
  );
}
