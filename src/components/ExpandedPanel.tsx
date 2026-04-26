import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  ApplyCodeActionResult,
  CodeActionProposal,
  CodeContext,
  SearchResult,
  VapiStatus,
} from "../hooks/useVapi";
import Buddy from "./Orb";

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
  onRunAction: (instruction: string) => Promise<CodeActionProposal>;
  onApplyAction: (proposal: CodeActionProposal) => Promise<ApplyCodeActionResult>;
  onAttachClipboard: () => Promise<CodeContext>;
  onSearch: (query: string) => Promise<SearchResult[]>;
}

type ComposerMode = "act" | "ask" | "search";

const MODE_COPY: Record<ComposerMode, { label: string; placeholder: string; cta: string }> = {
  act: {
    label: "Act",
    placeholder: "What should I change? Leave blank to infer.",
    cta: "Write",
  },
  ask: {
    label: "Ask",
    placeholder: "Ask briefly...",
    cta: "Ask",
  },
  search: {
    label: "Search",
    placeholder: "Search docs/errors...",
    cta: "Search",
  },
};

function targetLabel(ctx: CodeContext | null) {
  if (!ctx) return "No selection";
  return ctx.file_name || ctx.window_title || "Selection";
}

function selectedLines(ctx: CodeContext | null) {
  return ctx?.content ? ctx.content.split("\n").length : 0;
}

function lastSentence(text: string): string {
  if (!text.trim()) return "";
  const parts = text.split(/(?<=[.!?…])\s+/);
  return parts[parts.length - 1].trim();
}

function latestAssistantOutput(transcript: string, currentSpeech: string) {
  if (currentSpeech.trim()) return lastSentence(currentSpeech);

  const messages = transcript.split("\n").reduce<string[]>((items, line) => {
    if (line.startsWith("Cluddy:")) {
      items.push(line.slice(7).trimStart());
    } else if (line.startsWith("You:")) {
      return items;
    } else if (items.length > 0) {
      items[items.length - 1] += `\n${line}`;
    }
    return items;
  }, []);

  return messages.length ? messages[messages.length - 1].trim() : "";
}

function OutputOnly({
  output,
  busy,
  error,
  volumeLevel,
  status,
  currentSpeech,
}: {
  output: string;
  busy: boolean;
  error: string | null;
  volumeLevel: number;
  status: VapiStatus;
  currentSpeech: string;
}) {
  const label = error ? "error" : busy ? "working" : "assistant";
  const content = error || output || (busy ? "Working on it..." : "Select code, then write or ask.");
  const buddyStatus: VapiStatus = error ? "error" : busy ? "listening" : status;

  return (
    <section className="rounded-2xl px-3 py-2.5 panel-card min-h-[92px] max-h-[280px] overflow-y-auto transcript-scroll">
      <div className="text-[8px] uppercase tracking-[0.22em] text-white/28">{label}</div>
      <div className="mt-1 grid grid-cols-[64px_minmax(0,1fr)] gap-2 items-start">
        <Buddy
          callStatus={buddyStatus}
          currentSpeech={currentSpeech}
          bubbleText={content}
          compact
          showBubble={false}
        />
        <div
          className={`text-[11px] leading-relaxed whitespace-pre-wrap ${error ? "text-red-200/75" : "text-white/78"}`}
          style={{
            overflowWrap: "anywhere",
            opacity: error ? 1 : 0.72 + Math.min(volumeLevel, 0.24),
          }}
        >
          {content}
        </div>
      </div>
    </section>
  );
}

export default function ExpandedPanel({
  status,
  transcript,
  volumeLevel,
  codeContext,
  memories,
  currentSpeech,
  currentUserSpeech: _currentUserSpeech,
  error,
  onStop,
  onSendText,
  onAskClaude,
  onRunAction,
  onApplyAction,
  onAttachClipboard,
  onSearch,
}: ExpandedPanelProps) {
  const rootRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const [mode, setMode] = useState<ComposerMode>("act");
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [localOutput, setLocalOutput] = useState("");
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);

  const output = localOutput || latestAssistantOutput(transcript, currentSpeech);
  const lines = selectedLines(codeContext);
  const isLive = status === "connected" || status === "speaking" || status === "listening";

  const contextSummary = useMemo(() => {
    const target = targetLabel(codeContext);
    return lines ? `${target} - ${lines} lines` : target;
  }, [codeContext, lines]);

  useEffect(() => {
    inputRef.current?.focus();
  }, [mode]);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (event.key === "Escape") onStop();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onStop]);

  // Continuously sync window height to content height via ResizeObserver.
  // This handles all cases (streaming text, layout shifts, mode changes) without
  // a fragile dependency array and without racing against async IPC calls.
  useEffect(() => {
    const el = rootRef.current;
    if (!el) return;
    let raf: number;
    const flush = () => {
      const h = Math.ceil(el.scrollHeight) + 4;
      invoke("resize_panel", { height: h }).catch(() => {});
    };
    const ro = new ResizeObserver(() => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(flush);
    });
    ro.observe(el);
    flush(); // Sync on mount before first observation fires
    return () => {
      ro.disconnect();
      cancelAnimationFrame(raf);
    };
  }, []);

  const recapture = async () => {
    setBusy(true);
    setLocalOutput("");
    try {
      const ctx = await onAttachClipboard();
      const count = selectedLines(ctx);
      setLocalOutput(count ? `Captured ${count} selected lines from ${targetLabel(ctx)}.` : "Captured selection.");
    } catch (err) {
      setLocalOutput(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const writeChange = async () => {
    const proposal = await onRunAction(text);
    const changed = proposal.old_text !== proposal.replacement;
    if (!changed) {
      setLocalOutput(proposal.summary || "No edit was needed.");
      return;
    }

    const result = await onApplyAction(proposal);
    setLocalOutput(result.message || (result.changed ? "Change applied." : "No file change made."));
  };

  const submit = async () => {
    if (busy || (mode !== "act" && !text.trim())) return;

    setBusy(true);
    setLocalOutput("");
    setSearchResults([]);
    try {
      if (isLive && mode === "act" && text.trim()) {
        onSendText(text.trim());
        setLocalOutput("Sent to live call.");
      } else if (mode === "act") {
        await writeChange();
      } else if (mode === "ask") {
        const answer = await onAskClaude(text);
        setLocalOutput(answer);
      } else {
        const results = await onSearch(text);
        setSearchResults(results.slice(0, 2));
        setLocalOutput(results.length ? results[0].content : "No search results.");
      }
      setText("");
    } catch (err) {
      setLocalOutput(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div ref={rootRef} className="w-full flex flex-col gap-2 animate-slide-up" style={{ padding: 12 }} data-tauri-drag-region>
      <header className="flex items-center justify-between gap-2" data-tauri-drag-region>
        <div className="min-w-0" data-tauri-drag-region>
          <div className="text-[10px] uppercase tracking-[0.2em] text-white/55" data-tauri-drag-region>
            Cluddy
          </div>
          <div className="text-[9px] text-white/32 truncate" title={contextSummary}>
            {contextSummary}
            {memories.length > 0 ? ` - ${memories.length} memories` : ""}
          </div>
        </div>
        <div className="flex items-center gap-1.5">
          <button
            onClick={() => void recapture()}
            disabled={busy}
            title="Re-read clipboard (or press Cmd+Shift+T to auto-copy your selection)"
            className="text-[9px] px-2 py-1 rounded-lg text-amber-100/70 bg-amber-300/[0.09] disabled:opacity-40"
          >
            {codeContext?.content ? "recapture" : "capture"}
          </button>
          <button
            onClick={onStop}
            className="text-[9px] px-2 py-1 rounded-lg text-white/35 bg-white/[0.045]"
          >
            esc
          </button>
        </div>
      </header>

      {/* Context preview — shows what Cmd+Shift+T captured */}
      {codeContext?.content && (
        <section className="rounded-2xl panel-card overflow-hidden">
          <div className="flex items-center gap-2 px-3 pt-2 pb-1">
            {codeContext.language && (
              <span className="text-[8px] uppercase tracking-wider text-cyan-300/50 shrink-0">
                {codeContext.language}
              </span>
            )}
            <span className="text-[9px] text-white/50 truncate flex-1">
              {codeContext.file_name ?? "clipboard"}
            </span>
            <span className="text-[8px] text-white/20 shrink-0">
              {codeContext.content.split("\n").length} lines
            </span>
          </div>
          <pre
            className="px-3 pb-2 text-white/40 overflow-hidden"
            style={{ fontSize: 9, lineHeight: "14px", maxHeight: 70, fontFamily: "monospace" }}
          >
            {codeContext.content.split("\n").slice(0, 5).join("\n")}
          </pre>
        </section>
      )}

      <OutputOnly
        output={output}
        busy={busy}
        error={error}
        volumeLevel={volumeLevel}
        status={status}
        currentSpeech={currentSpeech}
      />

      {searchResults.length > 0 && (
        <div className="flex flex-col gap-1">
          {searchResults.map((result) => (
            <div key={result.url} className="rounded-xl px-3 py-2 bg-emerald-400/[0.055] border border-emerald-300/[0.1]">
              <div className="text-[10px] text-emerald-200/75 truncate">{result.title}</div>
              <div className="text-[9px] text-white/35 truncate">{result.content}</div>
            </div>
          ))}
        </div>
      )}

      <section className="rounded-2xl p-2 panel-card">
        <div className="mb-2 flex items-center justify-between">
          <div className="flex gap-1">
            {(["act", "ask", "search"] as ComposerMode[]).map((item) => (
              <button
                key={item}
                onClick={() => setMode(item)}
                className={`text-[10px] px-2 py-1 rounded-lg ${mode === item ? "text-white bg-white/[0.1]" : "text-white/38"}`}
              >
                {MODE_COPY[item].label}
              </button>
            ))}
          </div>
          <div className="text-[9px] text-white/25">{status}</div>
        </div>

        <div className="flex gap-2 items-end">
          <textarea
            ref={inputRef}
            value={text}
            onChange={(event) => setText(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
                event.preventDefault();
                void submit();
              }
            }}
            placeholder={MODE_COPY[mode].placeholder}
            rows={2}
            className="flex-1 text-[11px] px-3 py-2 rounded-xl outline-none resize-none transcript-scroll"
            style={{
              background: "rgba(255,255,255,0.055)",
              border: "1px solid rgba(255,255,255,0.08)",
              color: "rgba(255,255,255,0.88)",
              maxHeight: 76,
            }}
          />
          <button
            onClick={() => void submit()}
            disabled={busy || (mode !== "act" && !text.trim())}
            className="text-[10px] px-3 py-2 rounded-xl text-white disabled:opacity-35"
            style={{ background: "linear-gradient(135deg, rgba(14,165,233,0.32), rgba(139,92,246,0.32))" }}
          >
            {busy ? "..." : MODE_COPY[mode].cta}
          </button>
        </div>
      </section>
    </div>
  );
}
