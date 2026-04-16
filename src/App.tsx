import { useState, useEffect, useCallback } from "react";
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

  const { status, transcript, volumeLevel, analysis, startCall, stopCall, error } = useVapi();

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

  // Auto-return to orb when call ends
  useEffect(() => {
    if ((mode === "calling" || mode === "expanded") && status === "idle" && !error) {
      const t = setTimeout(async () => {
        setMode("orb");
        await invoke("set_window_mode", { mode: "orb" }).catch(() => {});
      }, 3000);
      return () => clearTimeout(t);
    }
  }, [mode, status, error]);

  if (mode === "orb") {
    return <div className="w-full h-full" style={{ background: "transparent" }}><Orb /></div>;
  }

  if (mode === "calling") {
    return (
      <div className="w-full h-full" style={{ background: "transparent" }}>
        <Orb callStatus={status} />
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
        memories={[]}
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
