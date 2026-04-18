import { useEffect, useRef, useState } from "react";
import { VapiStatus } from "../hooks/useVapi";

interface ExpandedPanelProps {
  status: VapiStatus;
  transcript: string;
  volumeLevel: number;
  analysis: string;
  memories: string[];
  error: string | null;
  onStop: () => void;
}

const BAR_COUNT = 24;

function Waveform({ volumeLevel, active }: { volumeLevel: number; active: boolean }) {
  return (
    <div className="flex items-center justify-center gap-[2.5px]" style={{ height: 32 }}>
      {Array.from({ length: BAR_COUNT }).map((_, i) => (
        <div
          key={i}
          className="rounded-full"
          style={{
            width: 2,
            height: 28,
            transformOrigin: "center",
            transform: "scaleY(0.12)",
            opacity: active ? 0.35 + volumeLevel * 0.65 : 0.15,
            background: active
              ? `rgba(139,92,246,${0.6 + volumeLevel * 0.4})`
              : "rgba(139,92,246,0.3)",
            animation: active
              ? `waveform ${0.75 + (i % 6) * 0.12}s ease-in-out infinite`
              : "none",
            animationDelay: `${i * 35}ms`,
            transition: "opacity 0.3s ease, background 0.3s ease",
          }}
        />
      ))}
    </div>
  );
}

function ConnectingSpinner() {
  return (
    <div className="flex items-center justify-center gap-[6px]" style={{ height: 32 }}>
      {[0, 1, 2].map((i) => (
        <div
          key={i}
          className="rounded-full"
          style={{
            width: 6,
            height: 6,
            background: "rgba(251,191,36,0.7)",
            animation: `connectingBounce 1.2s ease-in-out infinite`,
            animationDelay: `${i * 0.15}s`,
          }}
        />
      ))}
    </div>
  );
}

function TranscriptBubble({ line }: { line: string }) {
  const isUser = line.startsWith("You:");
  const isCluddy = line.startsWith("Cluddy:");
  if (!isUser && !isCluddy) return null;

  const speaker = isUser ? "you" : "cluddy";
  const text = line.slice(isUser ? 4 : 7).trim();

  return (
    <div className={`flex flex-col gap-0.5 ${isUser ? "items-end" : "items-start"}`}>
      <span className={`text-[9px] font-medium tracking-wide uppercase ${isUser ? "text-violet-400/60" : "text-cyan-400/60"}`}>
        {speaker}
      </span>
      <div
        className="text-xs leading-relaxed px-2.5 py-1.5 rounded-xl max-w-[90%]"
        style={{
          background: isUser
            ? "rgba(139,92,246,0.12)"
            : "rgba(8,145,178,0.1)",
          border: isUser
            ? "1px solid rgba(139,92,246,0.18)"
            : "1px solid rgba(8,145,178,0.15)",
          color: "rgba(255,255,255,0.75)",
        }}
      >
        {text}
      </div>
    </div>
  );
}

const STATUS_LABELS: Record<VapiStatus, string> = {
  idle:       "session ended",
  connecting: "connecting…",
  connected:  "listening",
  speaking:   "speaking",
  listening:  "thinking…",
  error:      "error",
};

export default function ExpandedPanel({
  status,
  transcript,
  volumeLevel,
  analysis,
  memories,
  error,
  onStop,
}: ExpandedPanelProps) {
  const isActive = status === "connected" || status === "speaking" || status === "listening";
  const isConnecting = status === "connecting";
  const lines = transcript ? transcript.split("\n").filter(Boolean) : [];

  // ── Auto-scroll transcript ──
  const scrollRef = useRef<HTMLDivElement>(null);
  const bottomRef = useRef<HTMLDivElement>(null);
  const [isScrolledUp, setIsScrolledUp] = useState(false);

  // Scroll to bottom whenever new lines arrive, unless user scrolled up
  useEffect(() => {
    if (!isScrolledUp && bottomRef.current) {
      bottomRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [lines.length, isScrolledUp]);

  // Detect when user scrolls away from bottom
  const handleScroll = () => {
    const el = scrollRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
    setIsScrolledUp(!atBottom);
  };

  const scrollToBottom = () => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    setIsScrolledUp(false);
  };

  // ── Esc key to stop ──
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") { onStop(); }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onStop]);

  return (
    <div className="w-full h-full flex flex-col animate-slide-up" style={{ padding: "10px 12px 10px" }}>

      {/* Header */}
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <div
            className="w-5 h-5 rounded-full flex-shrink-0"
            style={{
              background: "radial-gradient(circle at 38% 32%, #6d66f5, #3730a3)",
              boxShadow: "0 0 8px rgba(99,102,241,0.5), inset 0 1px 0 rgba(255,255,255,0.15)",
            }}
          />
          <span className="text-white/70 text-[11px] font-semibold tracking-widest uppercase">
            Cluddy
          </span>
        </div>
        <button
          onClick={onStop}
          className="text-white/30 hover:text-white/60 text-[10px] px-1.5 py-0.5 rounded font-mono transition-colors"
          style={{ border: "1px solid rgba(255,255,255,0.08)" }}
          title="Press Esc to close"
        >
          esc
        </button>
      </div>

      {/* Status row */}
      <div className="flex items-center gap-1.5 mb-2">
        <div
          className="w-1 h-1 rounded-full flex-shrink-0"
          style={{
            background: isActive ? "#4ade80" : isConnecting ? "#fbbf24" : status === "error" ? "#f87171" : "rgba(255,255,255,0.2)",
            boxShadow: isActive ? "0 0 4px #4ade80" : isConnecting ? "0 0 4px #fbbf24" : "none",
            animation: isActive || isConnecting ? "pulse 1.8s ease-in-out infinite" : "none",
          }}
        />
        <span className="text-white/35 text-[10px] tracking-wide">{STATUS_LABELS[status]}</span>
      </div>

      {/* Waveform or Connecting spinner */}
      <div className="flex justify-center mb-1.5">
        {isConnecting ? (
          <ConnectingSpinner />
        ) : (
          <Waveform volumeLevel={volumeLevel} active={isActive} />
        )}
      </div>

      {/* Screen context */}
      {analysis && (
        <div
          className="rounded-lg px-2.5 py-2 mb-2"
          style={{
            background: "rgba(99,102,241,0.06)",
            border: "1px solid rgba(99,102,241,0.12)",
          }}
        >
          <div className="flex items-center gap-1 mb-1">
            <span style={{ fontSize: 8 }}>◎</span>
            <span className="text-violet-400/50 text-[9px] font-medium uppercase tracking-widest">screen</span>
          </div>
          <p className="text-white/50 text-[10px] leading-relaxed line-clamp-2">{analysis}</p>
        </div>
      )}

      {/* Memory pills */}
      {memories.length > 0 && (
        <div className="flex flex-wrap gap-1 mb-2">
          {memories.slice(0, 3).map((m, i) => (
            <span
              key={i}
              title={m}
              className="text-[9px] px-2 py-0.5 rounded-full text-violet-300/50"
              style={{ background: "rgba(139,92,246,0.08)", border: "1px solid rgba(139,92,246,0.15)" }}
            >
              {m.slice(0, 28)}…
            </span>
          ))}
        </div>
      )}

      {/* Transcript */}
      {lines.length > 0 ? (
        <div className="relative flex-1 min-h-0">
          <div
            ref={scrollRef}
            onScroll={handleScroll}
            className="h-full overflow-y-auto transcript-scroll flex flex-col gap-2 rounded-lg p-2"
            style={{ background: "rgba(0,0,0,0.2)" }}
          >
            {lines.map((line, i) => (
              <TranscriptBubble key={i} line={line} />
            ))}
            {/* Invisible anchor for auto-scroll */}
            <div ref={bottomRef} />
          </div>
          {/* Scroll-to-bottom button */}
          {isScrolledUp && (
            <button
              onClick={scrollToBottom}
              className="absolute bottom-2 left-1/2 -translate-x-1/2 text-white/50 hover:text-white/80 transition-colors"
              style={{
                width: 24,
                height: 24,
                borderRadius: 12,
                background: "rgba(139,92,246,0.3)",
                border: "1px solid rgba(139,92,246,0.4)",
                backdropFilter: "blur(8px)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                fontSize: 12,
                lineHeight: 1,
              }}
            >
              ↓
            </button>
          )}
        </div>
      ) : !error && isActive ? (
        <p className="text-white/20 text-[10px] text-center mt-auto mb-2">
          speak to begin…
        </p>
      ) : null}

      {/* Error */}
      {error && (
        <div
          className="text-red-300/70 text-[10px] p-2 rounded-lg mt-auto"
          style={{ background: "rgba(239,68,68,0.08)", border: "1px solid rgba(239,68,68,0.15)" }}
        >
          {error}
        </div>
      )}

    </div>
  );
}
