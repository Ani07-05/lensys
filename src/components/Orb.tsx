import { VapiStatus } from "../hooks/useVapi";

interface OrbProps {
  isThinking?: boolean;
  callStatus?: VapiStatus;
}

const STATE: Record<string, { color: string; glow: string; dur: string; label: string }> = {
  idle:       { color: "#4f46e5", glow: "rgba(99,102,241,0.45)",  dur: "3s",    label: "</>" },
  connecting: { color: "#d97706", glow: "rgba(217,119,6,0.55)",   dur: "0.6s",  label: "···" },
  connected:  { color: "#16a34a", glow: "rgba(22,163,74,0.55)",   dur: "1.4s",  label: "</>" },
  listening:  { color: "#16a34a", glow: "rgba(22,163,74,0.55)",   dur: "1.4s",  label: "◉" },
  speaking:   { color: "#0891b2", glow: "rgba(8,145,178,0.65)",   dur: "0.65s", label: "▶" },
  error:      { color: "#dc2626", glow: "rgba(220,38,38,0.55)",   dur: "1s",    label: "!" },
};

export default function Orb({ isThinking = false, callStatus = "idle" }: OrbProps) {
  const s = STATE[isThinking ? "connecting" : callStatus] ?? STATE.idle;

  return (
    <div className="w-full h-full flex items-center justify-center">
      <div className="relative flex items-center justify-center" style={{ width: 48, height: 48 }}>

        {/* Glow layer — opacity/transform only */}
        <div
          className="absolute rounded-full"
          style={{
            width: 48, height: 48,
            background: s.glow,
            filter: "blur(13px)",
            animation: `orbBreathe ${s.dur} ease-in-out infinite`,
            willChange: "transform, opacity",
          }}
        />

        {/* Core */}
        <div
          className="relative rounded-full flex items-center justify-center"
          style={{
            width: 40, height: 40,
            background: s.color,
            animation: `orbBreathe ${s.dur} ease-in-out infinite`,
            willChange: "transform, opacity",
            transition: "background 0.4s ease",
          }}
        >
          <span className="font-mono font-bold text-white/90 pointer-events-none select-none"
            style={{ fontSize: 9, letterSpacing: "-0.04em" }}>
            {s.label}
          </span>
        </div>

      </div>
    </div>
  );
}
