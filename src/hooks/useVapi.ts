import { useEffect, useRef, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import Vapi from "@vapi-ai/web";

export type VapiStatus =
  | "idle"
  | "connecting"
  | "connected"
  | "speaking"
  | "listening"
  | "error";

interface AnalysisResult { analysis: string; memories: string[]; }

interface UseVapiReturn {
  status: VapiStatus;
  transcript: string;
  volumeLevel: number;
  analysis: string;
  memories: string[];
  currentSpeech: string;
  currentUserSpeech: string;
  startCall: (publicKey: string, assistantId: string) => Promise<void>;
  stopCall: () => void;
  error: string | null;
}

const CAPTURE_INTERVAL_MS = 2500;

export function useVapi(): UseVapiReturn {
  const vapiRef = useRef<Vapi | null>(null);
  const unlistenRef = useRef<(() => void) | null>(null);
  const captureIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const latestCaptureRef = useRef<AnalysisResult | null>(null);
  const [status, setStatus] = useState<VapiStatus>("idle");
  const [transcript, setTranscript] = useState("");
  const [volumeLevel, setVolumeLevel] = useState(0);
  const [analysis, setAnalysis] = useState("");
  const [currentSpeech, setCurrentSpeech] = useState("");
  const [currentUserSpeech, setCurrentUserSpeech] = useState("");
  const [memories, setMemories] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);

  const stopCapturing = useCallback(() => {
    if (captureIntervalRef.current) {
      clearInterval(captureIntervalRef.current);
      captureIntervalRef.current = null;
    }
    latestCaptureRef.current = null;
  }, []);

  const startCapturing = useCallback(() => {
    stopCapturing();
    const tick = () => {
      invoke<AnalysisResult>("capture_and_analyze")
        .then((r) => {
          latestCaptureRef.current = r;
          if (r.memories?.length) setMemories(r.memories);
        })
        .catch(() => {});
    };
    tick(); // immediate first capture
    captureIntervalRef.current = setInterval(tick, CAPTURE_INTERVAL_MS);
  }, [stopCapturing]);

  const startCall = useCallback(async (publicKey: string, assistantId: string) => {
    try {
      setError(null);
      setTranscript("");
      setAnalysis("");
      setStatus("connecting");

      unlistenRef.current?.();
      unlistenRef.current = null;

      const vapi = new Vapi(publicKey);
      vapiRef.current = vapi;
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const vapiAny = vapi as any;

      const unlistenAnalysis = await listen<string>("cluddy:analysis", (event) => {
        setAnalysis(event.payload);
      });
      unlistenRef.current = unlistenAnalysis;

      vapi.on("call-start", () => {
        setStatus("connected");
        startCapturing();
        // Inject whatever we already have from the pre-start capture
        const cached = latestCaptureRef.current;
        if (cached?.analysis) {
          vapiAny.send({
            type: "add-message",
            message: { role: "system", content: `[Screen] ${cached.analysis}` },
          });
        }
      });

      vapi.on("call-end", () => {
        setStatus("idle");
        setVolumeLevel(0);
        stopCapturing();
        unlistenRef.current?.();
        unlistenRef.current = null;
      });

      vapi.on("speech-start", () => setStatus("speaking"));
      vapi.on("speech-end", () => setStatus("listening"));

      let lastVolUpdate = 0;
      vapi.on("volume-level", (vol: number) => {
        const now = Date.now();
        if (now - lastVolUpdate > 120) { setVolumeLevel(vol); lastVolUpdate = now; }
      });

      // Track whether we already injected a FRESH capture for the current user turn.
      // Reset when the assistant finishes speaking (= new turn starts).
      let injectedThisTurn = false;
      let pendingFreshCapture = false;

      vapi.on("message", (msg: { type: string; transcript?: string; role?: string; transcriptType?: string }) => {
        // ── User starts talking: inject the cached snapshot immediately for low latency,
        //    then kick off a FRESH capture that will be injected when it completes.
        if (msg.type === "transcript" && msg.role === "user" && !injectedThisTurn) {
          injectedThisTurn = true;
          // 1) Inject whatever is in the background cache NOW (≤2.5s old)
          const cached = latestCaptureRef.current;
          if (cached?.analysis) {
            const memCtx = cached.memories.length
              ? `\n[Past context]\n${cached.memories.join("\n")}`
              : "";
            vapiAny.send({
              type: "add-message",
              message: { role: "system", content: `[Screen] ${cached.analysis}${memCtx}` },
            });
          }
          // 2) Fire a FRESH capture in the background — will arrive before Vapi's LLM call
          //    because the user is still speaking for another second or two.
          if (!pendingFreshCapture) {
            pendingFreshCapture = true;
            invoke<{ analysis: string; memories: string[] }>("capture_and_analyze")
              .then((r) => {
                latestCaptureRef.current = r;
                if (r.memories?.length) setMemories(r.memories);
                if (r.analysis) {
                  const memCtx2 = r.memories.length
                    ? `\n[Past context]\n${r.memories.join("\n")}`
                    : "";
                  vapiAny.send({
                    type: "add-message",
                    message: { role: "system", content: `[Screen update] ${r.analysis}${memCtx2}` },
                  });
                }
              })
              .catch(() => {})
              .finally(() => { pendingFreshCapture = false; });
          }
        }

        if (msg.type === "transcript" && msg.role === "assistant") {
          if (msg.transcriptType === "final") {
            injectedThisTurn = false; // Reset for next user turn
            setCurrentSpeech("");
          } else {
            setCurrentSpeech(msg.transcript ?? "");
          }
        }

        if (msg.type === "transcript" && msg.role === "user") {
          if (msg.transcriptType === "final") {
            setCurrentUserSpeech("");
          } else {
            setCurrentUserSpeech(msg.transcript ?? "");
          }
        }

        if (msg.type === "transcript" && msg.transcript) {
          const speaker = msg.role === "assistant" ? "Cluddy" : "You";
          setTranscript((prev) => prev ? `${prev}\n${speaker}: ${msg.transcript}` : `${speaker}: ${msg.transcript}`);
        }
      });

      vapi.on("error", (err: unknown) => {
        console.error("Vapi error:", err);
        // Extract a human-readable message from Vapi's error shapes
        let msg = "Voice error";
        const errObj = err as Record<string, unknown> | null;
        if (errObj) {
          const inner =
            (errObj.message as string) ||
            ((errObj.error as Record<string, unknown>)?.message as string) ||
            "";
          if (/microphone|permission|NotAllowed/i.test(inner)) {
            msg = "Microphone access denied — check browser permissions";
          } else if (/network|fetch|ERR_INTERNET/i.test(inner)) {
            msg = "Network error — check your connection";
          } else if (/timeout|timed out/i.test(inner)) {
            msg = "Connection timed out — try again";
          } else if (/quota|rate.?limit|429/i.test(inner)) {
            msg = "Rate limited — wait a moment and try again";
          } else if (inner) {
            msg = inner.length > 120 ? inner.slice(0, 117) + "…" : inner;
          }
        }
        setError(msg);
        setStatus("error");
      });

      // Warm the cache before start() so call-start injection has data ready
      await invoke<AnalysisResult>("capture_and_analyze")
        .then((r) => {
          latestCaptureRef.current = r;
          if (r.memories?.length) setMemories(r.memories);
        })
        .catch(() => {});

      await vapiAny.start(assistantId);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      setStatus("error");
    }
  }, [startCapturing, stopCapturing]);

  const stopCall = useCallback(() => {
    vapiRef.current?.stop();
    vapiRef.current = null;
    unlistenRef.current?.();
    unlistenRef.current = null;
    stopCapturing();
    setStatus("idle");
    setVolumeLevel(0);
  }, [stopCapturing]);

  useEffect(() => {
    return () => {
      vapiRef.current?.stop();
      unlistenRef.current?.();
      stopCapturing();
    };
  }, [stopCapturing]);

  return { status, transcript, volumeLevel, analysis, memories, currentSpeech, currentUserSpeech, startCall, stopCall, error };
}
