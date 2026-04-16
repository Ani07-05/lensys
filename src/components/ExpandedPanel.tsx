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

const BAR_COUNT = 20;

function WaveformBar({ index, volumeLevel, active }: { index: number; volumeLevel: number; active: boolean }) {
  const baseHeight = 4;
  const maxHeight = 36;
  const phase = (index / BAR_COUNT) * Math.PI * 2;
  const wave = active ? Math.abs(Math.sin(phase + Date.now() / 300)) : 0;
  const height = baseHeight + (maxHeight - baseHeight) * wave * volumeLevel;

  return (
    <div
      className="rounded-full transition-all duration-75"
      style={{
        width: 3,
        height: Math.max(baseHeight, height),
        background: active
          ? `rgba(139,92,246,${0.5 + volumeLevel * 0.5})`
          : "rgba(139,92,246,0.25)",
        animationDelay: `${index * 60}ms`,
      }}
    />
  );
}

function Waveform({ volumeLevel, active }: { volumeLevel: number; active: boolean }) {
  return (
    <div className="flex items-center gap-0.5 h-10">
      {Array.from({ length: BAR_COUNT }).map((_, i) => (
        <WaveformBar key={i} index={i} volumeLevel={volumeLevel} active={active} />
      ))}
    </div>
  );
}

const STATUS_LABELS: Record<VapiStatus, string> = {
  idle: "Session ended",
  connecting: "Connecting...",
  connected: "Cluddy is listening",
  speaking: "Cluddy is speaking",
  listening: "Listening...",
  error: "Error",
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

  return (
    <div className="w-full h-full p-3 flex flex-col gap-2 animate-slide-up">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div
            className="w-6 h-6 rounded-full flex-shrink-0"
            style={{
              background:
                "radial-gradient(circle at 35% 35%, rgba(196,181,253,0.95), rgba(139,92,246,0.9) 50%, rgba(91,33,182,0.95))",
              boxShadow: "0 0 8px rgba(139,92,246,0.6)",
            }}
          />
          <span className="text-violet-200 text-xs font-semibold tracking-wide">
            CLUDDY
          </span>
        </div>
        <button
          onClick={onStop}
          className="text-white/40 hover:text-white/80 text-xs px-2 py-0.5 rounded border border-white/10 hover:border-white/30 transition-colors"
        >
          ESC
        </button>
      </div>

      {/* Status */}
      <div className="flex items-center gap-1.5">
        <div
          className={`w-1.5 h-1.5 rounded-full flex-shrink-0 ${
            isActive
              ? "bg-green-400 animate-pulse"
              : status === "error"
              ? "bg-red-400"
              : "bg-white/20"
          }`}
        />
        <span className="text-white/50 text-xs">{STATUS_LABELS[status]}</span>
      </div>

      {/* Waveform */}
      <div className="flex justify-center py-1">
        <Waveform volumeLevel={volumeLevel} active={isActive} />
      </div>

      {/* Screen context */}
      {analysis && (
        <div className="rounded-lg p-2 text-xs" style={{ background: "rgba(139,92,246,0.08)", border: "1px solid rgba(139,92,246,0.15)" }}>
          <div className="text-violet-400/70 text-[10px] font-medium uppercase tracking-wider mb-1">
            Screen context
          </div>
          <p className="text-white/60 leading-relaxed line-clamp-3">{analysis}</p>
        </div>
      )}

      {/* Memory pills */}
      {memories.length > 0 && (
        <div className="flex flex-wrap gap-1">
          {memories.slice(0, 3).map((m, i) => (
            <span
              key={i}
              className="text-[10px] px-2 py-0.5 rounded-full text-violet-300/60 border border-violet-500/20"
              style={{ background: "rgba(139,92,246,0.06)" }}
            >
              {m.slice(0, 30)}…
            </span>
          ))}
        </div>
      )}

      {/* Transcript */}
      {transcript && (
        <div className="flex-1 min-h-0 overflow-y-auto transcript-scroll rounded-lg p-2" style={{ background: "rgba(0,0,0,0.3)" }}>
          <div className="text-white/60 text-xs leading-relaxed whitespace-pre-line">
            {transcript}
          </div>
        </div>
      )}

      {/* Error */}
      {error && (
        <div className="text-red-400/80 text-xs p-2 rounded bg-red-500/10 border border-red-500/20">
          {error}
        </div>
      )}

      {/* Hint */}
      {!transcript && !error && status === "connected" && (
        <p className="text-white/25 text-xs text-center">
          Ask me anything about your screen…
        </p>
      )}
    </div>
  );
}
