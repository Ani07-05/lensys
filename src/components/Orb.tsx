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
  bubbleText?: string;
  compact?: boolean;
  showBubble?: boolean;
}

type OrbMode = "idle" | "phrases" | "speech" | "alert";
type BuddyId = "mouse" | "mint" | "ember";
type BuddyFrames = Record<OrbMode, string[]>;

const BUDDY_STORAGE_KEY = "lensys:selected-buddy";

function getMode(callStatus: VapiStatus | undefined): OrbMode {
  if (callStatus === "error") return "alert";
  if (!callStatus || callStatus === "idle" || callStatus === "connecting") return "idle";
  if (callStatus === "speaking") return "speech";
  return "phrases";
}

const BUDDIES: Record<BuddyId, { name: string; frames: BuddyFrames }> = {
  mouse: {
    name: "mouse",
    frames: {
      idle: ["/buddies/mouse-idle.png", "/buddies/mouse-listen.png"],
      phrases: ["/buddies/mouse-listen.png", "/buddies/mouse-idle.png"],
      speech: ["/buddies/mouse-talk.png", "/buddies/mouse-idle.png"],
      alert: ["/buddies/mouse-alert.png", "/buddies/mouse-idle.png"],
    },
  },
  mint: {
    name: "mint",
    frames: {
      idle: ["/buddies/mint-idle.png", "/buddies/mint-listen.png"],
      phrases: ["/buddies/mint-listen.png", "/buddies/mint-idle.png"],
      speech: ["/buddies/mint-talk.png", "/buddies/mint-idle.png"],
      alert: ["/buddies/mint-alert.png", "/buddies/mint-idle.png"],
    },
  },
  ember: {
    name: "ember",
    frames: {
      idle: ["/buddies/ember-idle.png", "/buddies/ember-listen.png"],
      phrases: ["/buddies/ember-listen.png", "/buddies/ember-idle.png"],
      speech: ["/buddies/ember-talk.png", "/buddies/ember-idle.png"],
      alert: ["/buddies/ember-alert.png", "/buddies/ember-idle.png"],
    },
  },
};

const BUDDY_IDS = Object.keys(BUDDIES) as BuddyId[];

function getSavedBuddy(): BuddyId {
  if (typeof window === "undefined") return "mouse";
  const saved = window.localStorage.getItem(BUDDY_STORAGE_KEY);
  return BUDDY_IDS.includes(saved as BuddyId) ? (saved as BuddyId) : "mouse";
}

export default function Orb({
  callStatus = "idle",
  currentSpeech = "",
  bubbleText = "",
  compact = false,
  showBubble = true,
}: OrbProps) {
  const mode = getMode(callStatus);
  const [phraseIdx, setPhraseIdx] = useState(0);
  const [displayText, setDisplayText] = useState("");
  const [frameIdx, setFrameIdx] = useState(0);
  const [selectedBuddy, setSelectedBuddy] = useState<BuddyId>(getSavedBuddy);

  const cycleBuddy = () => {
    setSelectedBuddy((current) => {
      const next = BUDDY_IDS[(BUDDY_IDS.indexOf(current) + 1) % BUDDY_IDS.length];
      window.localStorage.setItem(BUDDY_STORAGE_KEY, next);
      return next;
    });
  };

  useEffect(() => {
    if (mode !== "phrases") return;
    setPhraseIdx(Math.floor(Math.random() * PHRASES.length));
    const t = setInterval(() => setPhraseIdx((i) => (i + 1) % PHRASES.length), 2000);
    return () => clearInterval(t);
  }, [mode]);

  useEffect(() => {
    const text = currentSpeech || bubbleText;
    if (text) {
      setDisplayText(text);
    } else if (mode !== "speech") {
      setDisplayText("");
    }
  }, [mode, currentSpeech, bubbleText]);

  useEffect(() => {
    setFrameIdx(0);
    const interval = mode === "speech" ? 260 : mode === "phrases" ? 520 : 760;
    const t = setInterval(() => {
      setFrameIdx((i) => (i + 1) % BUDDIES[selectedBuddy].frames[mode].length);
    }, interval);
    return () => clearInterval(t);
  }, [mode, selectedBuddy]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const isTyping = target?.tagName === "INPUT" || target?.tagName === "TEXTAREA" || target?.isContentEditable;
      if (isTyping || event.key.toLowerCase() !== "b") return;
      cycleBuddy();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const isSpeech = mode === "speech";
  const isPhrases = mode === "phrases";
  const bubble = isSpeech && displayText
    ? displayText
    : isPhrases
      ? PHRASES[phraseIdx]
      : mode === "alert"
        ? "need a sec..."
        : "";
  const buddy = BUDDIES[selectedBuddy];
  const frame = buddy.frames[mode][frameIdx] ?? buddy.frames.idle[0];
  const clippedBubble = bubble.length > (compact ? 180 : 140)
    ? `${bubble.slice(0, compact ? 176 : 136)}...`
    : bubble;

  return (
    <div className={`w-full h-full flex items-center justify-center ${compact ? "" : "buddy-stage"}`}>
      <button
        type="button"
        className={`buddy-wrap ${compact ? "buddy-wrap-compact" : ""}`}
        data-state={mode}
        onClick={cycleBuddy}
        title={`Buddy: ${buddy.name}. Press B or click to switch.`}
      >
        <div className="buddy-sparkle" />
        <img
          src={frame}
          alt={`Lensys ${buddy.name} buddy`}
          draggable={false}
          className={`buddy-sprite ${isSpeech ? "buddy-sprite-talk" : ""}`}
        />
        {showBubble && clippedBubble && (
          <div className={`buddy-bubble ${compact ? "buddy-bubble-compact" : ""}`}>
            {clippedBubble}
          </div>
        )}
      </button>
    </div>
  );
}
