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

export interface Symbol {
  kind: string;
  name: string;
  line: number;
}

export interface CodeContext {
  file_path: string | null;
  file_name: string | null;
  language: string | null;
  content: string | null;
  window_title: string;
  active_app: string;
  is_ide: boolean;
  symbols: Symbol[];
}

interface UseVapiReturn {
  status: VapiStatus;
  transcript: string;
  volumeLevel: number;
  codeContext: CodeContext | null;
  memories: string[];
  currentSpeech: string;
  currentUserSpeech: string;
  startCall: (publicKey: string, assistantId: string) => Promise<void>;
  stopCall: () => void;
  sendTextMessage: (text: string) => void;
  askClaude: (question: string) => Promise<string>;
  runAction: (instruction: string) => Promise<CodeActionProposal>;
  applyAction: (proposal: CodeActionProposal) => Promise<ApplyCodeActionResult>;
  attachClipboardContext: (captureSelection?: boolean) => Promise<CodeContext>;
  searchWeb: (query: string) => Promise<SearchResult[]>;
  error: string | null;
}

export interface SearchResult {
  title: string;
  url: string;
  content: string;
}

export interface CodeActionProposal {
  summary: string;
  confidence: number;
  target_file: string | null;
  old_text: string;
  replacement: string;
  needs_confirmation: boolean;
  risk_notes: string[];
}

export interface ApplyCodeActionResult {
  target_file: string;
  changed: boolean;
  message: string;
}

const CAPTURE_INTERVAL_MS = 3000;

function formatCodeContext(ctx: CodeContext): string {
  const parts: string[] = [];
  if (ctx.file_name) {
    parts.push(`[Code: ${ctx.file_name}${ctx.language ? ` (${ctx.language})` : ""}]`);
  }
  if (ctx.symbols.length > 0) {
    const syms = ctx.symbols
      .slice(0, 8)
      .map((s) => `${s.kind} ${s.name}`)
      .join(", ");
    parts.push(`Symbols: ${syms}`);
  }
  if (ctx.content) {
    const snippet = ctx.content.split("\n").slice(0, 30).join("\n");
    parts.push(`\`\`\`\n${snippet}\n\`\`\``);
  }
  if (!ctx.is_ide && ctx.window_title) {
    parts.push(`[Window: ${ctx.window_title}]`);
  }
  return parts.join("\n");
}

export function useVapi(): UseVapiReturn {
  const vapiRef = useRef<Vapi | null>(null);
  const unlistenRef = useRef<(() => void) | null>(null);
  const captureIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const latestCtxRef = useRef<CodeContext | null>(null);
  const [status, setStatus] = useState<VapiStatus>("idle");
  const [transcript, setTranscript] = useState("");
  const [volumeLevel, setVolumeLevel] = useState(0);
  const [codeContext, setCodeContext] = useState<CodeContext | null>(null);
  const [memories] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [currentSpeech, setCurrentSpeech] = useState("");
  const [currentUserSpeech, setCurrentUserSpeech] = useState("");

  const appendTranscript = useCallback((speaker: "You" | "Cluddy", text: string) => {
    setTranscript((prev) => prev ? `${prev}\n${speaker}: ${text}` : `${speaker}: ${text}`);
  }, []);

  const sendVapiMessage = useCallback((role: "user" | "system", content: string) => {
    if (!vapiRef.current) return false;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (vapiRef.current as any).send({
      type: "add-message",
      message: { role, content },
    });
    return true;
  }, []);

  const stopCapturing = useCallback(() => {
    if (captureIntervalRef.current) {
      clearInterval(captureIntervalRef.current);
      captureIntervalRef.current = null;
    }
    latestCtxRef.current = null;
  }, []);

  const startCapturing = useCallback(() => {
    stopCapturing();
    const tick = () => {
      invoke<CodeContext>("get_code_context")
        .then((ctx) => {
          latestCtxRef.current = ctx;
          setCodeContext(ctx);
        })
        .catch(() => {});
    };
    tick();
    captureIntervalRef.current = setInterval(tick, CAPTURE_INTERVAL_MS);
  }, [stopCapturing]);

  const sendTextMessage = useCallback((text: string) => {
    const trimmed = text.trim();
    if (!trimmed) return;
    appendTranscript("You", trimmed);
    sendVapiMessage("user", trimmed);
  }, [appendTranscript, sendVapiMessage]);

  const attachClipboardContext = useCallback(async (captureSelection = false): Promise<CodeContext> => {
    const command = captureSelection ? "capture_selection_context" : "get_clipboard_context";
    const ctx = await invoke<CodeContext>(command);
    latestCtxRef.current = ctx;
    setCodeContext(ctx);

    const formatted = formatCodeContext(ctx);
    if (formatted) {
      sendVapiMessage("system", `[Clipboard selection]\n${formatted}`);
    }

    return ctx;
  }, [sendVapiMessage]);

  const askClaude = useCallback(async (question: string): Promise<string> => {
    const trimmed = question.trim();
    if (!trimmed) return "";

    setError(null);
    appendTranscript("You", trimmed);

    if (!latestCtxRef.current?.content) {
      await invoke<CodeContext>("get_code_context")
        .then((ctx) => {
          latestCtxRef.current = ctx;
          setCodeContext(ctx);
        })
        .catch(() => {});
    }

    const reply = await invoke<string>("ask_claude", { question: trimmed });
    appendTranscript("Cluddy", reply);
    return reply;
  }, [appendTranscript]);

  const runAction = useCallback(async (instruction: string): Promise<CodeActionProposal> => {
    const trimmed = instruction.trim();
    setError(null);

    if (!latestCtxRef.current?.content) {
      await invoke<CodeContext>("get_code_context")
        .then((ctx) => {
          latestCtxRef.current = ctx;
          setCodeContext(ctx);
        })
        .catch(() => {});
    }

    appendTranscript("You", trimmed || "Infer the best edit for the selection.");
    const proposal = await invoke<CodeActionProposal>("propose_code_action", {
      instruction: trimmed,
    });
    appendTranscript("Cluddy", proposal.summary);
    return proposal;
  }, [appendTranscript]);

  const applyAction = useCallback(async (
    proposal: CodeActionProposal,
  ): Promise<ApplyCodeActionResult> => {
    if (!proposal.target_file) {
      throw new Error("This action has no target file");
    }

    const result = await invoke<ApplyCodeActionResult>("apply_code_action", {
      request: {
        target_file: proposal.target_file,
        old_text: proposal.old_text,
        replacement: proposal.replacement,
      },
    });
    appendTranscript("Cluddy", result.message);
    return result;
  }, [appendTranscript]);

  const searchWeb = useCallback(async (query: string): Promise<SearchResult[]> => {
    try {
      const results = await invoke<SearchResult[]>("web_search", { query });
      if (vapiRef.current && results.length > 0) {
        const formatted = results
          .map((r, i) => `${i + 1}. ${r.title}\n   ${r.url}\n   ${r.content}`)
          .join("\n\n");
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        (vapiRef.current as any).send({
          type: "add-message",
          message: { role: "system", content: `[Web Search: ${query}]\n${formatted}` },
        });
      }
      return results;
    } catch {
      return [];
    }
  }, []);

  const startCall = useCallback(async (publicKey: string, assistantId: string) => {
    try {
      setError(null);
      setTranscript("");
      setCodeContext(null);
      setStatus("connecting");

      unlistenRef.current?.();
      unlistenRef.current = null;

      const vapi = new Vapi(publicKey);
      vapiRef.current = vapi;
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const vapiAny = vapi as any;

      const unlistenCtx = await listen<CodeContext>("cluddy:code_context", (event) => {
        setCodeContext(event.payload);
      });
      unlistenRef.current = unlistenCtx;

      vapi.on("call-start", () => {
        setStatus("connected");
        startCapturing();
        const cached = latestCtxRef.current;
        if (cached) {
          const formatted = formatCodeContext(cached);
          if (formatted) {
            vapiAny.send({
              type: "add-message",
              message: { role: "system", content: formatted },
            });
          }
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

      let injectedThisTurn = false;
      let pendingFresh = false;

      vapi.on("message", (msg: { type: string; transcript?: string; role?: string; transcriptType?: string }) => {
        if (msg.type === "transcript" && msg.role === "user" && !injectedThisTurn) {
          injectedThisTurn = true;
          const cached = latestCtxRef.current;
          if (cached) {
            const formatted = formatCodeContext(cached);
            if (formatted) {
              vapiAny.send({
                type: "add-message",
                message: { role: "system", content: formatted },
              });
            }
          }
          // Fire a fresh capture while user is still speaking
          if (!pendingFresh) {
            pendingFresh = true;
            invoke<CodeContext>("get_code_context")
              .then((ctx) => {
                latestCtxRef.current = ctx;
                setCodeContext(ctx);
                const updated = formatCodeContext(ctx);
                if (updated) {
                  vapiAny.send({
                    type: "add-message",
                    message: { role: "system", content: `[Updated] ${updated}` },
                  });
                }
              })
              .catch(() => {})
              .finally(() => { pendingFresh = false; });
          }
        }

        if (msg.type === "transcript" && msg.role === "assistant") {
          if (msg.transcriptType === "final") {
            injectedThisTurn = false;
            setCurrentSpeech("");
            // Async wiki update from this turn
            if (transcript) {
              invoke("wiki_update_from_turn", { turn: transcript }).catch(() => {});
            }
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

        if (msg.type === "transcript" && msg.transcriptType === "final" && msg.transcript) {
          const speaker = msg.role === "assistant" ? "Cluddy" : "You";
          setTranscript((prev) => prev ? `${prev}\n${speaker}: ${msg.transcript}` : `${speaker}: ${msg.transcript}`);
        }
      });

      vapi.on("error", (err: unknown) => {
        console.error("Vapi error:", err);
        let msg = "Voice error";
        const errObj = err as Record<string, unknown> | null;
        if (errObj) {
          const inner =
            (errObj.message as string) ||
            ((errObj.error as Record<string, unknown>)?.message as string) || "";
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

      // Warm cache before start
      await invoke<CodeContext>("get_code_context")
        .then((ctx) => { latestCtxRef.current = ctx; setCodeContext(ctx); })
        .catch(() => {});

      await vapiAny.start(assistantId);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      setStatus("error");
    }
  }, [startCapturing, stopCapturing, transcript]);

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

  return {
    status,
    transcript,
    volumeLevel,
    codeContext,
    memories,
    currentSpeech,
    currentUserSpeech,
    startCall,
    stopCall,
    sendTextMessage,
    askClaude,
    runAction,
    applyAction,
    attachClipboardContext,
    searchWeb,
    error,
  };
}
