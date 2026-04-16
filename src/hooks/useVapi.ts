import { useEffect, useRef, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
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
  startCall: (publicKey: string, assistantId: string) => Promise<void>;
  stopCall: () => void;
  error: string | null;
}

export function useVapi(): UseVapiReturn {
  const vapiRef = useRef<Vapi | null>(null);
  const [status, setStatus] = useState<VapiStatus>("idle");
  const [transcript, setTranscript] = useState("");
  const [volumeLevel, setVolumeLevel] = useState(0);
  const [analysis, setAnalysis] = useState("");
  const [error, setError] = useState<string | null>(null);

  const startCall = useCallback(async (publicKey: string, assistantId: string) => {
    try {
      setError(null);
      setTranscript("");
      setAnalysis("");
      setStatus("connecting");

      const vapi = new Vapi(publicKey);
      vapiRef.current = vapi;
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const vapiAny = vapi as any;

      vapi.on("call-start", () => setStatus("connected"));
      vapi.on("call-end", () => { setStatus("idle"); setVolumeLevel(0); });
      vapi.on("speech-start", () => setStatus("speaking"));
      vapi.on("speech-end", () => setStatus("listening"));

      let lastVolUpdate = 0;
      vapi.on("volume-level", (vol: number) => {
        const now = Date.now();
        if (now - lastVolUpdate > 120) { setVolumeLevel(vol); lastVolUpdate = now; }
      });

      // Track whether we already screenshotted for the current user turn
      let screenshottedThisTurn = false;

      vapi.on("message", (msg: { type: string; transcript?: string; role?: string; transcriptType?: string }) => {
        // Screenshot fires once when user FIRST starts speaking each turn
        if (msg.type === "transcript" && msg.role === "user" && !screenshottedThisTurn) {
          screenshottedThisTurn = true;
          invoke<AnalysisResult>("capture_and_analyze")
            .then((result) => {
              setAnalysis(result.analysis);
              if (result.analysis) {
                vapiAny.send({
                  type: "add-message",
                  message: { role: "system", content: `[Screen] ${result.analysis}` },
                });
              }
            })
            .catch((e) => console.error("Vision error:", e));
        }

        // Reset flag when assistant finishes responding (ready for next user turn)
        if (msg.type === "transcript" && msg.role === "assistant" && msg.transcriptType === "final") {
          screenshottedThisTurn = false;
        }

        if (msg.type === "transcript" && msg.transcript) {
          const speaker = msg.role === "assistant" ? "Cluddy" : "You";
          setTranscript((prev) => prev ? `${prev}\n${speaker}: ${msg.transcript}` : `${speaker}: ${msg.transcript}`);
        }
      });

      vapi.on("error", (err: unknown) => {
        const raw = JSON.stringify(err, null, 2);
        console.error("Vapi error:", raw, err);
        const msg =
          (err as { message?: string })?.message ||
          (err as { error?: { message?: string } })?.error?.message ||
          raw || "Voice error";
        setError(msg);
        setStatus("error");
      });

      await vapiAny.start(assistantId);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      setStatus("error");
    }
  }, []);

  const stopCall = useCallback(() => {
    vapiRef.current?.stop();
    vapiRef.current = null;
    setStatus("idle");
    setVolumeLevel(0);
  }, []);

  useEffect(() => { return () => { vapiRef.current?.stop(); }; }, []);

  return { status, transcript, volumeLevel, analysis, startCall, stopCall, error };
}
