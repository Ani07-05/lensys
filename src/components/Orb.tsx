import { useEffect, useState } from "react";
import { VapiStatus } from "../hooks/useVapi";

const PHRASES = [
  "lollygagging…",
  "dillydallying…",
  "noodling on it…",
  "twiddling bits…",
  "pondering…",
  "cogitating…",
  "ruminating…",
  "staring into void…",
  "asking the oracle…",
  "vibing…",
  "mulling it over…",
  "consulting the gods…",
  "buffering…",
];

interface OrbProps {
  callStatus?: VapiStatus;
  currentSpeech?: string;
}

type OrbMode = "sphere" | "phrases" | "speech";

function getMode(callStatus: VapiStatus | undefined): OrbMode {
  if (!callStatus || callStatus === "idle" || callStatus === "connecting" || callStatus === "error") return "sphere";
  if (callStatus === "speaking") return "speech";
  return "phrases";
}

const SPHERE_COLORS: Record<string, { from: string; to: string; glow: string; dur: string }> = {
  idle:       { from: "#6d66f5", to: "#3730a3", glow: "rgba(99,102,241,0.4)",   dur: "3s"    },
  connecting: { from: "#fbbf24", to: "#b45309", glow: "rgba(251,191,36,0.5)",   dur: "0.55s" },
  connected:  { from: "#34d399", to: "#059669", glow: "rgba(52,211,153,0.45)",  dur: "2s"    },
  speaking:   { from: "#38bdf8", to: "#0284c7", glow: "rgba(56,189,248,0.45)",  dur: "0.7s"  },
  listening:  { from: "#a78bfa", to: "#6d28d9", glow: "rgba(167,139,250,0.4)",  dur: "1.5s"  },
  error:      { from: "#f87171", to: "#b91c1c", glow: "rgba(248,113,113,0.45)", dur: "1s"    },
};

export default function Orb({ callStatus = "idle", currentSpeech = "" }: OrbProps) {
  const mode = getMode(callStatus);
  const [phraseIdx, setPhraseIdx] = useState(0);
  const [displayText, setDisplayText] = useState("");

  // Cycle phrases when in phrases mode
  useEffect(() => {
    if (mode !== "phrases") return;
    setPhraseIdx(Math.floor(Math.random() * PHRASES.length));
    const t = setInterval(() => setPhraseIdx((i) => (i + 1) % PHRASES.length), 2000);
    return () => clearInterval(t);
  }, [mode]);

  // Smooth text update for speech — avoid flicker on every partial word
  useEffect(() => {
    if (mode === "speech" && currentSpeech) {
      setDisplayText(currentSpeech);
    } else if (mode !== "speech") {
      setDisplayText("");
    }
  }, [mode, currentSpeech]);

  const sphereColor = SPHERE_COLORS[callStatus] ?? SPHERE_COLORS.idle;
  const isSpeech = mode === "speech";
  const isPhrases = mode === "phrases";
  const isSphere = mode === "sphere";

  // Pill width: sphere=36, phrases~160, speech~220
  const pillW = isSphere ? 36 : isSpeech ? 220 : 160;
  const pillH = 36;
  const borderRadius = isSphere ? 18 : 12;

  // Glow color for active states
  const activeGlow = callStatus === "speaking"
    ? "rgba(56,189,248,0.45)"
    : "rgba(74,222,128,0.35)";

  return (
    <div className="w-full h-full flex items-center justify-center">
      <div
        style={{
          width: pillW,
          height: pillH,
          borderRadius,
          transition: "width 0.35s cubic-bezier(0.34,1.56,0.64,1), border-radius 0.35s ease",
          position: "relative",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          overflow: "hidden",
          background: isSphere
            ? `radial-gradient(circle at 38% 32%, ${sphereColor.from}, ${sphereColor.to})`
            : isSpeech
            ? "rgba(8,145,178,0.15)"
            : "rgba(74,222,128,0.1)",
          boxShadow: isSphere
            ? `0 0 14px ${sphereColor.glow}, inset 0 1px 0 rgba(255,255,255,0.15)`
            : `0 0 12px ${activeGlow}, inset 0 0 0 1px rgba(255,255,255,0.07)`,
        }}
      >
        {/* Animated glow ring for sphere */}
        {isSphere && (
          <div
            className="absolute rounded-full"
            style={{
              width: 36, height: 36,
              background: sphereColor.glow,
              filter: "blur(12px)",
              animation: `orbBreathe ${sphereColor.dur} ease-in-out infinite`,
              willChange: "transform, opacity",
              zIndex: 0,
            }}
          />
        )}

        {/* Specular highlight for sphere */}
        {isSphere && (
          <div
            className="absolute rounded-full"
            style={{ width: 9, height: 5, top: 6, left: 8, background: "rgba(255,255,255,0.22)", filter: "blur(2px)", zIndex: 1 }}
          />
        )}

        {/* Phrases text */}
        {isPhrases && (
          <span
            key={phraseIdx}
            className="font-mono text-white/60 whitespace-nowrap select-none"
            style={{ fontSize: 11, animation: "fadeSlideIn 0.3s ease", zIndex: 1 }}
          >
            {PHRASES[phraseIdx]}
          </span>
        )}

        {/* Speech text — scrolls from right, shows last ~28 chars */}
        {isSpeech && (
          <span
            className="text-white/80 whitespace-nowrap select-none px-3"
            style={{
              fontSize: 11,
              fontFamily: "system-ui, sans-serif",
              animation: "fadeSlideIn 0.15s ease",
              zIndex: 1,
              maxWidth: 210,
              overflow: "hidden",
              textOverflow: "ellipsis",
            }}
          >
            {displayText.length > 30 ? "…" + displayText.slice(-28) : displayText}
          </span>
        )}

        {/* Speaking left-side accent dot */}
        {isSpeech && (
          <div
            className="absolute left-2.5 rounded-full flex-shrink-0"
            style={{
              width: 5, height: 5,
              background: "#38bdf8",
              boxShadow: "0 0 6px rgba(56,189,248,0.8)",
              animation: "orbBreathe 0.6s ease-in-out infinite",
            }}
          />
        )}
      </div>
    </div>
  );
}
