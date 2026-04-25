import { useEffect, useRef, useState } from "react";
import { VapiStatus, CodeContext, SearchResult } from "../hooks/useVapi";

interface ExpandedPanelProps {
  status: VapiStatus;
  transcript: string;
  volumeLevel: number;
  codeContext: CodeContext | null;
  memories: string[];
  currentSpeech: string;
  currentUserSpeech: string;
  error: string | null;
  onStop: () => void;
  onSendText: (text: string) => void;
  onAskClaude: (question: string) => Promise<string>;
  onAttachClipboard: () => Promise<CodeContext>;
  onSearch: (query: string) => Promise<SearchResult[]>;
}

const LANG_COLORS: Record<string, string> = {
  Rust: "#f74c00", TypeScript: "#3178c6", JavaScript: "#f0db4f",
  Python: "#3776ab", Go: "#00add8", "C++": "#00599c",
};

// ── Thinking dots animation ────────────────────────────────────────────────
function ThinkingDots() {
  return (
    <div className="flex items-center gap-[5px]">
      {[0, 1, 2, 3].map((i) => (
        <div
          key={i}
          style={{
            width: 5,
            height: 5,
            borderRadius: "50%",
            background: `rgba(139,92,246,${0.4 + i * 0.15})`,
            animation: "thinkingPulse 1.4s ease-in-out infinite",
            animationDelay: `${i * 0.18}s`,
          }}
        />
      ))}
    </div>
  );
}

// ── Live speech display ────────────────────────────────────────────────────
function LiveSpeech({
  status,
  currentSpeech,
  currentUserSpeech,
  volumeLevel,
}: {
  status: VapiStatus;
  currentSpeech: string;
  currentUserSpeech: string;
  volumeLevel: number;
}) {
  const isThinking = status === "listening";
  const isCluddySpeaking = status === "speaking" && currentSpeech;
  const isUserSpeaking = (status === "connected" || status === "speaking") && currentUserSpeech;

  if (isThinking) {
    return (
      <div className="flex flex-col items-center justify-center gap-3 py-4">
        <ThinkingDots />
        <span className="text-white/25 text-[9px] tracking-widest uppercase">thinking</span>
      </div>
    );
  }

  if (isCluddySpeaking) {
    return (
      <div
        className="rounded-xl px-3 py-2.5 mb-1"
        style={{
          background: "rgba(8,145,178,0.08)",
          border: "1px solid rgba(8,145,178,0.18)",
          transition: "opacity 0.2s ease",
        }}
      >
        <span className="text-[8px] font-semibold tracking-widest uppercase text-cyan-300/50 block mb-1">cluddy</span>
        <p
          className="text-[12px] leading-relaxed font-light"
          style={{
            color: `rgba(255,255,255,${0.7 + volumeLevel * 0.3})`,
            textShadow: volumeLevel > 0.3 ? "0 0 12px rgba(8,145,178,0.4)" : "none",
          }}
        >
          {currentSpeech}
        </p>
      </div>
    );
  }

  if (isUserSpeaking) {
    return (
      <div
        className="rounded-xl px-3 py-2.5 mb-1"
        style={{
          background: "rgba(139,92,246,0.08)",
          border: "1px solid rgba(139,92,246,0.18)",
        }}
      >
        <span className="text-[8px] font-semibold tracking-widest uppercase text-violet-300/50 block mb-1">you</span>
        <p className="text-[12px] leading-relaxed font-light text-white/80">{currentUserSpeech}</p>
      </div>
    );
  }

  return null;
}

// ── Past transcript (collapsed history) ───────────────────────────────────
function TranscriptHistory({ transcript }: { transcript: string }) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const bottomRef = useRef<HTMLDivElement>(null);
  const [scrolledUp, setScrolledUp] = useState(false);
  const messages = transcript.split("\n").reduce<Array<{ speaker: "You" | "Cluddy"; text: string }>>((items, line) => {
    if (line.startsWith("You:")) {
      items.push({ speaker: "You", text: line.slice(4).trimStart() });
    } else if (line.startsWith("Cluddy:")) {
      items.push({ speaker: "Cluddy", text: line.slice(7).trimStart() });
    } else if (items.length > 0) {
      items[items.length - 1].text += `\n${line}`;
    }

    return items;
  }, []);

  useEffect(() => {
    if (!scrolledUp) bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages.length, scrolledUp]);

  return (
    <div className="relative flex-1 min-h-0">
      <div
        ref={scrollRef}
        onScroll={(e) => {
          const el = e.currentTarget;
          setScrolledUp(el.scrollHeight - el.scrollTop - el.clientHeight >= 36);
        }}
        className="h-full overflow-y-auto transcript-scroll flex flex-col gap-1.5 px-1 py-1"
      >
        {messages.length === 0 && (
          <div
            className="h-full min-h-[120px] rounded-xl flex items-center justify-center text-[10px] text-white/25 font-mono"
            style={{ border: "1px dashed rgba(255,255,255,0.08)", background: "rgba(255,255,255,0.025)" }}
          >
            transcript will appear here
          </div>
        )}
        {messages.map(({ speaker, text }, i) => {
          const isUser = speaker === "You";
          return (
            <div key={i} className={`flex gap-1.5 min-w-0 ${isUser ? "flex-row-reverse" : "flex-row"}`}>
              <span
                className="text-[8px] font-semibold tracking-widest uppercase mt-0.5 flex-shrink-0"
                style={{ color: isUser ? "rgba(167,139,250,0.5)" : "rgba(103,232,249,0.5)" }}
              >
                {isUser ? "you" : "ai"}
              </span>
              <p
                className="text-[10px] leading-relaxed rounded-lg px-2 py-1 max-w-[85%] min-w-0"
                style={{
                  color: "rgba(255,255,255,0.82)",
                  background: isUser ? "rgba(139,92,246,0.1)" : "rgba(8,145,178,0.08)",
                  overflowWrap: "anywhere",
                  whiteSpace: "pre-wrap",
                }}
              >
                {text}
              </p>
            </div>
          );
        })}
        <div ref={bottomRef} />
      </div>
      {scrolledUp && (
        <button
          onClick={() => { bottomRef.current?.scrollIntoView({ behavior: "smooth" }); setScrolledUp(false); }}
          className="absolute bottom-1 left-1/2 -translate-x-1/2 text-white/40 hover:text-white/70 transition-colors text-xs"
          style={{ width: 20, height: 20, borderRadius: 10, background: "rgba(139,92,246,0.25)", display: "flex", alignItems: "center", justifyContent: "center" }}
        >↓</button>
      )}
    </div>
  );
}

// ── Text / search input bar ────────────────────────────────────────────────
function TextInput({
  status,
  onSend,
  onAsk,
  onAttachClipboard,
  onSearch,
}: {
  status: VapiStatus;
  onSend: (t: string) => void;
  onAsk: (q: string) => Promise<string>;
  onAttachClipboard: () => Promise<CodeContext>;
  onSearch: (q: string) => Promise<SearchResult[]>;
}) {
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [results, setResults] = useState<SearchResult[]>([]);
  const [hint, setHint] = useState<string | null>(null);
  const isLive = status === "connected" || status === "speaking" || status === "listening";

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const submit = async () => {
    const trimmed = text.trim();
    if (!trimmed || busy) return;
    setBusy(true);
    setHint(null);
    try {
      if (isLive) {
        onSend(trimmed);
      } else {
        await onAsk(trimmed);
      }
      setText("");
      setResults([]);
    } catch (err) {
      setHint(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const attachClipboard = async () => {
    if (busy) return;
    setBusy(true);
    setHint(null);
    try {
      const ctx = await onAttachClipboard();
      const lines = ctx.content?.split("\n").length ?? 0;
      setHint(`clipboard attached${lines ? ` (${lines} lines)` : ""}`);
      if (!text.trim()) setText("Explain this selected code.");
    } catch (err) {
      setHint(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const search = async () => {
    if (!text.trim() || busy) return;
    setBusy(true);
    setHint(null);
    try {
      const r = await onSearch(text.trim());
      setResults(r);
      setText("");
      if (r.length === 0) setHint("no search results");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="pt-2 flex-shrink-0">
      {results.length > 0 && (
        <div className="mb-1.5 flex flex-col gap-1">
          {results.map((r, i) => (
            <div key={i} className="rounded-lg px-2 py-1.5 text-[9px]" style={{ background: "rgba(16,185,129,0.06)", border: "1px solid rgba(16,185,129,0.12)" }}>
              <span className="text-emerald-300/70 font-medium block truncate">{r.title}</span>
              <p className="text-white/35 mt-0.5 max-h-[2.6em] overflow-hidden leading-snug">{r.content}</p>
            </div>
          ))}
        </div>
      )}
      {hint && (
        <div className="mb-1 text-[9px] text-white/35 font-mono truncate">{hint}</div>
      )}
      <div className="flex gap-1 items-end">
        <textarea
          ref={inputRef}
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
              e.preventDefault();
              void submit();
            }
          }}
          placeholder={isLive ? "send text to the live call..." : "ask about current code or clipboard..."}
          rows={2}
          className="flex-1 text-[11px] px-2.5 py-1.5 rounded-lg outline-none resize-none transcript-scroll"
          style={{
            background: "rgba(255,255,255,0.05)",
            border: "1px solid rgba(255,255,255,0.1)",
            color: "rgba(255,255,255,0.88)",
            maxHeight: 76,
          }}
        />
        <button onClick={() => void submit()} disabled={busy || !text.trim()} title={isLive ? "Send to call (Ctrl+Enter)" : "Ask about context (Ctrl+Enter)"} className="text-[11px] px-2 py-1.5 rounded-lg text-violet-300/75 hover:text-violet-200 transition-colors flex-shrink-0 disabled:opacity-30" style={{ background: "rgba(139,92,246,0.12)" }}>{isLive ? "send" : "ask"}</button>
        <button onClick={() => void attachClipboard()} disabled={busy} title="Attach clipboard selection" className="text-[10px] px-1.5 py-1.5 rounded-lg text-amber-300/65 hover:text-amber-200 transition-colors flex-shrink-0 disabled:opacity-30" style={{ background: "rgba(245,158,11,0.09)" }}>clip</button>
        <button onClick={search} disabled={busy} title="Web search" className="text-[11px] px-1.5 py-1.5 rounded-lg text-emerald-400/60 hover:text-emerald-300 transition-colors flex-shrink-0 disabled:opacity-30" style={{ background: "rgba(16,185,129,0.08)" }}>⌕</button>
      </div>
    </div>
  );
}

// ── Main panel ─────────────────────────────────────────────────────────────
export default function ExpandedPanel({
  status, transcript, volumeLevel, codeContext,
  currentSpeech, currentUserSpeech, error,
  onStop, onSendText, onAskClaude, onAttachClipboard, onSearch,
}: ExpandedPanelProps) {
  const isConnecting = status === "connecting";
  const lang = codeContext?.language ?? "";
  const langColor = LANG_COLORS[lang] ?? "rgba(139,92,246,0.8)";

  useEffect(() => {
    const handler = (e: KeyboardEvent) => { if (e.key === "Escape") onStop(); };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onStop]);

  return (
    <div className="w-full h-full flex flex-col animate-slide-up" style={{ padding: "12px 14px 10px" }}>

      {/* Header row */}
      <div className="flex items-center justify-between mb-3 flex-shrink-0">
        <div className="flex items-center gap-2">
          {/* Status dot */}
          <div style={{
            width: 6, height: 6, borderRadius: "50%", flexShrink: 0,
            background: status === "error" ? "#f87171"
              : isConnecting ? "#fbbf24"
              : status === "idle" ? "rgba(255,255,255,0.15)"
              : "#4ade80",
            boxShadow: status === "speaking" || status === "connected" ? "0 0 6px #4ade80"
              : isConnecting ? "0 0 6px #fbbf24" : "none",
            animation: (status === "speaking" || status === "connected" || isConnecting)
              ? "pulse 1.8s ease-in-out infinite" : "none",
          }} />
          <span className="text-white/60 text-[10px] tracking-widest uppercase font-semibold">
            {isConnecting ? "connecting…"
              : status === "speaking" ? "speaking"
              : status === "listening" ? "thinking…"
              : status === "connected" ? "listening"
              : status === "error" ? "error"
              : "ended"}
          </span>
        </div>

        <div className="flex items-center gap-2">
          {/* File pill */}
          {codeContext?.file_name && (
            <span
              className="text-[8px] px-1.5 py-0.5 rounded font-mono"
              style={{ background: `${langColor}18`, color: langColor, border: `1px solid ${langColor}30` }}
            >
              {codeContext.file_name}
            </span>
          )}
          <button
            onClick={onStop}
            className="text-white/25 hover:text-white/55 text-[10px] px-1.5 py-0.5 rounded font-mono transition-colors"
            style={{ border: "1px solid rgba(255,255,255,0.08)" }}
          >esc</button>
        </div>
      </div>

      {/* Connecting spinner */}
      {isConnecting && (
        <div className="flex items-center justify-center gap-[6px] py-6">
          {[0, 1, 2].map((i) => (
            <div key={i} style={{
              width: 6, height: 6, borderRadius: "50%",
              background: "rgba(251,191,36,0.7)",
              animation: "connectingBounce 1.2s ease-in-out infinite",
              animationDelay: `${i * 0.15}s`,
            }} />
          ))}
        </div>
      )}

      {/* Live speech — the star of the show */}
      {!isConnecting && (
        <div className="flex-shrink-0">
          <LiveSpeech
            status={status}
            currentSpeech={currentSpeech}
            currentUserSpeech={currentUserSpeech}
            volumeLevel={volumeLevel}
          />
        </div>
      )}

      {/* Past transcript history */}
      {!isConnecting && (
        <TranscriptHistory transcript={transcript} />
      )}

      {/* Error */}
      {error && (
        <div className="text-red-300/70 text-[10px] p-2 rounded-lg mt-1 flex-shrink-0"
          style={{ background: "rgba(239,68,68,0.07)", border: "1px solid rgba(239,68,68,0.12)" }}>
          {error}
        </div>
      )}

      {/* Text input — always at bottom */}
      <TextInput
        status={status}
        onSend={onSendText}
        onAsk={onAskClaude}
        onAttachClipboard={onAttachClipboard}
        onSearch={onSearch}
      />

    </div>
  );
}
