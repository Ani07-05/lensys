import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import Orb from "./components/Orb";
import ExpandedPanel from "./components/ExpandedPanel";
import { useVapi } from "./hooks/useVapi";

type AppMode = "orb" | "calling" | "expanded";

interface EnvKeys { vapi_public_key: string; vapi_assistant_id: string; }

export default function App() {
  const [mode, setMode] = useState<AppMode>("orb");
  const [envKeys, setEnvKeys] = useState<EnvKeys>({ vapi_public_key: "", vapi_assistant_id: "" });

  const { status, transcript, volumeLevel, analysis, memories, currentSpeech, currentUserSpeech, startCall, stopCall, error } = useVapi();

  useEffect(() => {
    invoke<EnvKeys>("get_env_keys").then(setEnvKeys).catch(console.error);
  }, []);

  // Ctrl+Shift+A — start call instantly, no screenshot delay
  const handleHotkey = useCallback(async () => {
    if (mode === "calling" || mode === "expanded") {
      stopCall();
      setMode("orb");
      await invoke("set_window_mode", { mode: "orb" }).catch(() => {});
      return;
    }
    if (mode !== "orb") return;

    setMode("calling");
    startCall(envKeys.vapi_public_key, envKeys.vapi_assistant_id)
      .catch((e) => console.error("Vapi start:", e));
  }, [mode, envKeys, startCall, stopCall]);

  // Ctrl+Shift+S — toggle full panel while in call
  const handleTogglePanel = useCallback(async () => {
    if (mode === "calling") {
      await invoke("set_window_mode", { mode: "expanded" }).catch(() => {});
      setMode("expanded");
    } else if (mode === "expanded") {
      await invoke("set_window_mode", { mode: "orb" }).catch(() => {});
      setMode("calling");
    }
  }, [mode]);

  useEffect(() => {
    const u1 = listen("cluddy:hotkey", () => handleHotkey());
    const u2 = listen("cluddy:panel", () => handleTogglePanel());
    return () => { u1.then((f) => f()); u2.then((f) => f()); };
  }, [handleHotkey, handleTogglePanel]);

  // Keep a ref to current status so the delayed timeout can double-check
  const statusRef = useRef(status);
  useEffect(() => { statusRef.current = status; }, [status]);

  // Auto-return to orb when call ends or errors
  useEffect(() => {
    const shouldReturn = (mode === "calling" || mode === "expanded") &&
      (status === "idle" || status === "error");
    if (!shouldReturn) return;
    // Use a longer delay so brief Vapi reconnects don't kill the session
    const delay = status === "error" ? 4000 : 5000;
    const t = setTimeout(async () => {
      // Double-check: only tear down if status is STILL idle/error
      if (statusRef.current !== "idle" && statusRef.current !== "error") return;
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
        {error && (
          <div className="font-mono text-red-400/80 text-center px-2 mt-1"
            style={{ fontSize: 8, maxWidth: 260, wordBreak: "break-word" }}>
            {error}
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="w-full h-full glass rounded-2xl overflow-hidden" style={{ background: "rgba(8,6,18,0.9)" }}>
      <ExpandedPanel
        status={status}
        transcript={transcript}
        volumeLevel={volumeLevel}
        analysis={analysis}
        memories={memories}
        error={error}
        onStop={async () => {
          stopCall();
          setMode("orb");
          await invoke("set_window_mode", { mode: "orb" }).catch(() => {});
        }}
      />
    </div>
  );
}
