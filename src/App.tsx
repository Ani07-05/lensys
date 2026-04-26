import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import Orb from "./components/Orb";
import ExpandedPanel from "./components/ExpandedPanel.tsx";
import { useVapi } from "./hooks/useVapi";

type AppMode = "orb" | "calling" | "expanded";

interface EnvKeys {
  vapi_public_key: string;
  vapi_assistant_id: string;
  has_claude: boolean;
  has_search: boolean;
  has_groq: boolean;
}

export default function App() {
  const [mode, setMode] = useState<AppMode>("orb");
  const [envKeys, setEnvKeys] = useState<EnvKeys>({
    vapi_public_key: "",
    vapi_assistant_id: "",
    has_claude: false,
    has_search: false,
    has_groq: false,
  });

  const {
    status, transcript, volumeLevel, codeContext, memories,
    currentSpeech, currentUserSpeech, startCall, stopCall,
    sendTextMessage, askClaude, runAction, applyAction,
    attachClipboardContext, searchWeb, error,
  } = useVapi();

  useEffect(() => {
    invoke<EnvKeys>("get_env_keys").then(setEnvKeys).catch(console.error);
  }, []);

  const handleHotkey = useCallback(async () => {
    if (mode === "calling" || mode === "expanded") {
      stopCall();
      setMode("orb");
      await invoke("set_window_mode", { mode: "orb" }).catch(() => {});
      return;
    }
    if (mode !== "orb") return;
    await invoke("set_window_mode", { mode: "calling" }).catch(() => {});
    setMode("calling");
    startCall(envKeys.vapi_public_key, envKeys.vapi_assistant_id)
      .catch((e) => console.error("Vapi start:", e));
  }, [mode, envKeys, startCall, stopCall]);

  const handleTogglePanel = useCallback(async () => {
    if (mode === "calling") {
      await invoke("set_window_mode", { mode: "expanded" }).catch(() => {});
      setMode("expanded");
    } else if (mode === "expanded") {
      const nextMode = status === "idle" || status === "error" ? "orb" : "calling";
      await invoke("set_window_mode", { mode: nextMode }).catch(() => {});
      setMode(nextMode);
    }
  }, [mode, status]);

  // Ctrl+Shift+T — open expanded panel directly to text mode
  const handleTextMode = useCallback(async () => {
    await attachClipboardContext(true).catch(() => {});
    if (mode !== "expanded") {
      await invoke("set_window_mode", { mode: "expanded" }).catch(() => {});
      setMode("expanded");
    }
  }, [mode, attachClipboardContext]);

  useEffect(() => {
    const u1 = listen("cluddy:hotkey", () => handleHotkey());
    const u2 = listen("cluddy:panel", () => handleTogglePanel());
    const u3 = listen("cluddy:text_mode", () => handleTextMode());
    const u4 = listen("cluddy:buddy", () => window.dispatchEvent(new Event("lensys:cycle-buddy")));
    return () => {
      u1.then((f) => f());
      u2.then((f) => f());
      u3.then((f) => f());
      u4.then((f) => f());
    };
  }, [handleHotkey, handleTogglePanel, handleTextMode]);

  const statusRef = useRef(status);
  useEffect(() => { statusRef.current = status; }, [status]);

  const hadActiveCallRef = useRef(false);
  useEffect(() => {
    if (status === "connecting" || status === "connected" || status === "speaking" || status === "listening") {
      hadActiveCallRef.current = true;
    }
  }, [status]);

  // Auto-return to orb when an actual call ends. Idle text mode should stay open.
  useEffect(() => {
    const shouldReturn = hadActiveCallRef.current &&
      (mode === "calling" || mode === "expanded") &&
      (status === "idle" || status === "error");
    if (!shouldReturn) return;
    const delay = status === "error" ? 4000 : 5000;
    const t = setTimeout(async () => {
      if (statusRef.current !== "idle" && statusRef.current !== "error") return;
      hadActiveCallRef.current = false;
      setMode("orb");
      await invoke("set_window_mode", { mode: "orb" }).catch(() => {});
    }, delay);
    return () => clearTimeout(t);
  }, [mode, status]);

  if (mode === "orb") {
    return <div className="w-full h-full" style={{ background: "transparent" }}><Orb /></div>;
  }

  if (mode === "calling") {
    return (
      <div className="w-full h-full flex flex-col items-center justify-center" style={{ background: "transparent" }}>
        <Orb callStatus={status} currentSpeech={currentSpeech} />
        {currentUserSpeech && (
          <div className="font-mono text-violet-300/70 text-center px-2 mt-1 truncate"
            style={{ fontSize: 9, maxWidth: 260 }}>
            you: {currentUserSpeech}
          </div>
        )}
        {codeContext?.file_name && (
          <div className="font-mono text-cyan-400/40 text-center px-2 mt-0.5 truncate"
            style={{ fontSize: 8, maxWidth: 260 }}>
            ⌨ {codeContext.file_name}{codeContext.language ? ` · ${codeContext.language}` : ""}
          </div>
        )}
        {error && (
          <div className="font-mono text-red-400/80 text-center px-2 mt-1"
            style={{ fontSize: 8, maxWidth: 260, wordBreak: "break-word" }}>
            {error}
          </div>
        )}
      </div>
    );
  }

  const handleStop = async () => {
    stopCall();
    setMode("orb");
    await invoke("set_window_mode", { mode: "orb" }).catch(() => {});
  };

  return (
    <div className="w-full h-full glass rounded-2xl overflow-hidden" style={{ background: "rgba(8,6,18,0.9)" }}>
      <ExpandedPanel
        status={status}
        transcript={transcript}
        volumeLevel={volumeLevel}
        codeContext={codeContext}
        memories={memories}
        currentSpeech={currentSpeech}
        currentUserSpeech={currentUserSpeech}
        error={error}
        onStop={handleStop}
        onSendText={sendTextMessage}
        onAskClaude={askClaude}
        onRunAction={runAction}
        onApplyAction={applyAction}
        onAttachClipboard={attachClipboardContext}
        onSearch={searchWeb}
      />
    </div>
  );
}
