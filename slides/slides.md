---
theme: default
title: Lensys
titleTemplate: '%s'
highlighter: shiki
transition: fade
mdc: true
layout: cover
background: '#08080f'
fonts:
  sans: 'Syne'
  mono: 'JetBrains Mono'
---

<style>
.slidev-layout {
  background: #08080f !important;
  color: #e2e8f0;
  font-family: 'Syne', sans-serif;
  overflow: hidden;
}
.slidev-layout.cover,
.slidev-layout.default {
  padding: 2rem 2.5rem;
}
.orb {
  width: 72px;
  height: 72px;
  border-radius: 50%;
  background: radial-gradient(circle at 38% 32%, #818cf8, #4f46e5 55%, #1e1b4b 90%);
  box-shadow: 0 0 28px #4f46e5aa, 0 0 56px #4f46e530;
  animation: breathe 3s ease-in-out infinite;
  display: inline-block;
  flex-shrink: 0;
}
@keyframes breathe {
  0%, 100% { transform: scale(1); box-shadow: 0 0 28px #4f46e5aa; }
  50%       { transform: scale(1.08); box-shadow: 0 0 44px #818cf8bb, 0 0 72px #4f46e540; }
}
.glass {
  background: rgba(255,255,255,0.04);
  border: 1px solid rgba(255,255,255,0.08);
  border-radius: 10px;
  padding: 0.9rem 1.1rem;
}
.tag {
  display: inline-block;
  background: rgba(99,102,241,0.12);
  border: 1px solid rgba(99,102,241,0.25);
  color: #a5b4fc;
  border-radius: 4px;
  padding: 2px 9px;
  font-size: 0.65rem;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  margin: 2px;
}
.rule {
  height: 1px;
  background: linear-gradient(90deg, #4f46e5 0%, rgba(79,70,229,0) 100%);
  margin-bottom: 0.6rem;
  width: 40px;
}
.slide-num {
  position: absolute;
  top: 1.1rem;
  right: 1.5rem;
  font-size: 0.6rem;
  color: rgba(255,255,255,0.18);
  font-family: 'JetBrains Mono', monospace;
  letter-spacing: 0.1em;
}
.label {
  font-size: 0.6rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  font-weight: 600;
  font-family: 'JetBrains Mono', monospace;
  margin-bottom: 0.35rem;
}
.dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-right: 0.4rem;
  vertical-align: middle;
  flex-shrink: 0;
}
#slidev-goto-dialog {
  display: none !important;
}
</style>

<!-- SLIDE 1 — COVER -->
<div style="display:flex;flex-direction:column;align-items:center;justify-content:center;height:100%;gap:1.25rem;text-align:center;">
  <div class="orb" style="width:86px;height:86px;"></div>
  <div>
    <h1 style="font-size:3.2rem;font-weight:800;letter-spacing:-0.02em;color:#fff;margin:0 0 0.2rem;line-height:1;">Lensys</h1>
    <p style="font-size:0.78rem;color:#6366f1;letter-spacing:0.16em;text-transform:uppercase;font-weight:600;margin:0;">Floating AI Dev Buddy</p>
  </div>
  <p style="color:#475569;font-size:0.8rem;max-width:360px;line-height:1.6;margin:0;">Voice-driven, screen-aware desktop assistant. Lives in your tray. Understands your code.</p>
  <div>
    <span class="tag">Tauri 2</span>
    <span class="tag">React</span>
    <span class="tag">Rust</span>
    <span class="tag">Vapi AI</span>
    <span class="tag">Groq</span>
    <span class="tag">Qdrant</span>
  </div>
</div>

---
layout: default
background: '#08080f'
---

<!-- SLIDE 2 — THE PROBLEM -->
<div class="slide-num">02 / 09</div>
<div class="rule"></div>
<h2 style="font-size:1.4rem;font-weight:700;color:#fff;margin:0 0 1rem;letter-spacing:-0.01em;">The Problem</h2>

<div style="display:grid;grid-template-columns:1fr 1fr;gap:0.65rem;">
  <div class="glass">
    <div class="label" style="color:#6366f1;">01</div>
    <div style="font-size:0.82rem;font-weight:600;color:#e2e8f0;margin-bottom:0.3rem;">Context switching kills flow</div>
    <p style="font-size:0.72rem;color:#64748b;line-height:1.5;margin:0;">Every alt-tab to ask an AI assistant breaks your train of thought mid-session.</p>
  </div>
  <div class="glass">
    <div class="label" style="color:#6366f1;">02</div>
    <div style="font-size:0.82rem;font-weight:600;color:#e2e8f0;margin-bottom:0.3rem;">AI doesn't see your screen</div>
    <p style="font-size:0.72rem;color:#64748b;line-height:1.5;margin:0;">Copying error messages into a chat window is tedious. The AI has no idea what you're looking at.</p>
  </div>
  <div class="glass">
    <div class="label" style="color:#6366f1;">03</div>
    <div style="font-size:0.82rem;font-weight:600;color:#e2e8f0;margin-bottom:0.3rem;">Typing is slow mid-flow</div>
    <p style="font-size:0.72rem;color:#64748b;line-height:1.5;margin:0;">Switching to keyboard to type a question interrupts deep focus. You want to speak and get an answer.</p>
  </div>
  <div class="glass">
    <div class="label" style="color:#6366f1;">04</div>
    <div style="font-size:0.82rem;font-weight:600;color:#e2e8f0;margin-bottom:0.3rem;">No memory across sessions</div>
    <p style="font-size:0.72rem;color:#64748b;line-height:1.5;margin:0;">AI assistants forget everything. You re-explain the same context again and again.</p>
  </div>
</div>

---
layout: default
background: '#08080f'
---

<!-- SLIDE 3 — MEET LENSYS -->
<div class="slide-num">03 / 09</div>
<div class="rule"></div>
<h2 style="font-size:1.4rem;font-weight:700;color:#fff;margin:0 0 0.9rem;letter-spacing:-0.01em;">Meet Lensys</h2>

<div style="display:flex;gap:2.5rem;align-items:flex-start;">
  <div style="flex:1;">
    <p style="color:#94a3b8;font-size:0.78rem;line-height:1.65;margin:0 0 0.9rem;">A tiny floating orb on your screen. Press a hotkey, speak — it sees your screen, understands your code, and answers instantly.</p>
    <div style="display:flex;flex-direction:column;gap:0.55rem;">
      <div style="display:flex;gap:0.75rem;align-items:flex-start;">
        <span style="color:#6366f1;font-size:0.6rem;font-family:'JetBrains Mono',monospace;padding-top:3px;min-width:18px;font-weight:600;">01</span>
        <div>
          <div style="font-size:0.8rem;font-weight:600;color:#e2e8f0;line-height:1.3;">Always visible, never intrusive</div>
          <div style="font-size:0.7rem;color:#64748b;line-height:1.4;">80x80px orb follows your cursor, stays out of the way</div>
        </div>
      </div>
      <div style="display:flex;gap:0.75rem;align-items:flex-start;">
        <span style="color:#6366f1;font-size:0.6rem;font-family:'JetBrains Mono',monospace;padding-top:3px;min-width:18px;font-weight:600;">02</span>
        <div>
          <div style="font-size:0.8rem;font-weight:600;color:#e2e8f0;line-height:1.3;">Voice-first interaction</div>
          <div style="font-size:0.7rem;color:#64748b;line-height:1.4;">No typing. Press <code style="font-size:0.65rem;background:rgba(255,255,255,0.06);padding:1px 5px;border-radius:3px;">Ctrl+Shift+A</code> and talk.</div>
        </div>
      </div>
      <div style="display:flex;gap:0.75rem;align-items:flex-start;">
        <span style="color:#6366f1;font-size:0.6rem;font-family:'JetBrains Mono',monospace;padding-top:3px;min-width:18px;font-weight:600;">03</span>
        <div>
          <div style="font-size:0.8rem;font-weight:600;color:#e2e8f0;line-height:1.3;">Screen-aware AI</div>
          <div style="font-size:0.7rem;color:#64748b;line-height:1.4;">Captures and analyzes your screen every time you speak</div>
        </div>
      </div>
      <div style="display:flex;gap:0.75rem;align-items:flex-start;">
        <span style="color:#6366f1;font-size:0.6rem;font-family:'JetBrains Mono',monospace;padding-top:3px;min-width:18px;font-weight:600;">04</span>
        <div>
          <div style="font-size:0.8rem;font-weight:600;color:#e2e8f0;line-height:1.3;">Semantic memory</div>
          <div style="font-size:0.7rem;color:#64748b;line-height:1.4;">Remembers past screens and sessions via vector search</div>
        </div>
      </div>
    </div>
  </div>
  <div style="display:flex;flex-direction:column;align-items:center;gap:0.75rem;padding-top:0.25rem;">
    <div class="orb"></div>
    <div class="glass" style="text-align:center;min-width:130px;padding:0.55rem 1rem;">
      <div class="label" style="color:#6366f1;margin-bottom:0.2rem;">Status</div>
      <div style="font-size:0.78rem;color:#4ade80;font-weight:600;">&#9679; Listening</div>
    </div>
  </div>
</div>

---
layout: default
background: '#08080f'
---

<!-- SLIDE 4 — HOW IT WORKS -->
<div class="slide-num">04 / 09</div>
<div class="rule"></div>
<h2 style="font-size:1.4rem;font-weight:700;color:#fff;margin:0 0 1rem;letter-spacing:-0.01em;">How It Works</h2>

<div style="display:flex;align-items:stretch;gap:0;margin-bottom:1.1rem;background:rgba(255,255,255,0.025);border:1px solid rgba(255,255,255,0.06);border-radius:10px;overflow:hidden;">
  <div style="flex:1;padding:0.75rem 0.9rem;border-right:1px solid rgba(255,255,255,0.06);">
    <div class="label" style="color:#475569;margin-bottom:0.25rem;">trigger</div>
    <div style="font-size:0.8rem;font-weight:600;color:#e2e8f0;">You speak</div>
    <div style="font-size:0.62rem;color:#334155;margin-top:0.15rem;font-family:'JetBrains Mono',monospace;">Ctrl+Shift+A</div>
  </div>
  <div style="display:flex;align-items:center;padding:0 0.4rem;color:#4f46e5;font-size:0.9rem;">&#8594;</div>
  <div style="flex:1;padding:0.75rem 0.9rem;border-right:1px solid rgba(255,255,255,0.06);">
    <div class="label" style="color:#475569;margin-bottom:0.25rem;">capture</div>
    <div style="font-size:0.8rem;font-weight:600;color:#e2e8f0;">Screenshot</div>
    <div style="font-size:0.62rem;color:#334155;margin-top:0.15rem;font-family:'JetBrains Mono',monospace;">Rust + xcap</div>
  </div>
  <div style="display:flex;align-items:center;padding:0 0.4rem;color:#4f46e5;font-size:0.9rem;">&#8594;</div>
  <div style="flex:1;padding:0.75rem 0.9rem;border-right:1px solid rgba(255,255,255,0.06);">
    <div class="label" style="color:#475569;margin-bottom:0.25rem;">analyze</div>
    <div style="font-size:0.8rem;font-weight:600;color:#e2e8f0;">Vision AI</div>
    <div style="font-size:0.62rem;color:#334155;margin-top:0.15rem;font-family:'JetBrains Mono',monospace;">Groq Llama 4</div>
  </div>
  <div style="display:flex;align-items:center;padding:0 0.4rem;color:#4f46e5;font-size:0.9rem;">&#8594;</div>
  <div style="flex:1;padding:0.75rem 0.9rem;border-right:1px solid rgba(255,255,255,0.06);">
    <div class="label" style="color:#475569;margin-bottom:0.25rem;">respond</div>
    <div style="font-size:0.8rem;font-weight:600;color:#e2e8f0;">Voice AI</div>
    <div style="font-size:0.62rem;color:#334155;margin-top:0.15rem;font-family:'JetBrains Mono',monospace;">Vapi Assistant</div>
  </div>
  <div style="display:flex;align-items:center;padding:0 0.4rem;color:#4f46e5;font-size:0.9rem;">&#8594;</div>
  <div style="flex:1;padding:0.75rem 0.9rem;">
    <div class="label" style="color:#475569;margin-bottom:0.25rem;">output</div>
    <div style="font-size:0.8rem;font-weight:600;color:#e2e8f0;">Answer</div>
    <div style="font-size:0.62rem;color:#334155;margin-top:0.15rem;font-family:'JetBrains Mono',monospace;">+ memory stored</div>
  </div>
</div>

<div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:0.65rem;">
  <div class="glass">
    <div class="label" style="color:#6366f1;">01 — Capture</div>
    <p style="font-size:0.72rem;color:#64748b;line-height:1.5;margin:0;">Rust captures a PNG screenshot on each user turn. Resized to 1920px max, encoded as base64.</p>
  </div>
  <div class="glass">
    <div class="label" style="color:#6366f1;">02 — Analyze</div>
    <p style="font-size:0.72rem;color:#64748b;line-height:1.5;margin:0;">Groq Llama 4 Scout (17B) produces a one-sentence description of the code or error on screen.</p>
  </div>
  <div class="glass">
    <div class="label" style="color:#6366f1;">03 — Respond</div>
    <p style="font-size:0.72rem;color:#64748b;line-height:1.5;margin:0;">Context injected as <code style="font-size:0.65rem;background:rgba(255,255,255,0.06);padding:1px 4px;border-radius:3px;">[Screen] ...</code> into the Vapi conversation before the assistant speaks.</p>
  </div>
</div>

---
layout: default
background: '#08080f'
---

<!-- SLIDE 5 — TECH STACK -->
<div class="slide-num">05 / 09</div>
<div class="rule"></div>
<h2 style="font-size:1.4rem;font-weight:700;color:#fff;margin:0 0 1rem;letter-spacing:-0.01em;">Tech Stack</h2>

<div style="display:grid;grid-template-columns:1fr 1fr;gap:0.65rem;">
  <div class="glass">
    <div class="label" style="color:#f97316;margin-bottom:0.45rem;">Rust — Tauri 2</div>
    <ul style="list-style:none;margin:0;padding:0;font-size:0.72rem;color:#94a3b8;display:flex;flex-direction:column;gap:0.28rem;">
      <li><span style="color:#6366f1;">xcap</span> — cross-platform screen capture</li>
      <li><span style="color:#6366f1;">reqwest</span> — async HTTP for Groq API</li>
      <li><span style="color:#6366f1;">qdrant-client</span> — vector DB integration</li>
      <li><span style="color:#6366f1;">global-hotkey</span> — system-wide shortcuts</li>
      <li>Custom 256-dim n-gram embedder</li>
    </ul>
  </div>
  <div class="glass">
    <div class="label" style="color:#60a5fa;margin-bottom:0.45rem;">React — Frontend</div>
    <ul style="list-style:none;margin:0;padding:0;font-size:0.72rem;color:#94a3b8;display:flex;flex-direction:column;gap:0.28rem;">
      <li><span style="color:#6366f1;">Vite</span> + TypeScript (strict)</li>
      <li><span style="color:#6366f1;">Tailwind CSS</span> — glass morphism theme</li>
      <li><span style="color:#6366f1;">useVapi hook</span> — full lifecycle management</li>
      <li>Volume-driven waveform animation</li>
      <li>Real-time transcript + memory pills</li>
    </ul>
  </div>
  <div class="glass">
    <div class="label" style="color:#4ade80;margin-bottom:0.45rem;">Vapi — Voice AI</div>
    <ul style="list-style:none;margin:0;padding:0;font-size:0.72rem;color:#94a3b8;display:flex;flex-direction:column;gap:0.28rem;">
      <li>Real-time voice conversation layer</li>
      <li>Event-driven lifecycle callbacks</li>
      <li>Volume levels drive waveform UI</li>
      <li>Screenshot injected as system message</li>
    </ul>
  </div>
  <div class="glass">
    <div class="label" style="color:#c084fc;margin-bottom:0.45rem;">Qdrant — Memory</div>
    <ul style="list-style:none;margin:0;padding:0;font-size:0.72rem;color:#94a3b8;display:flex;flex-direction:column;gap:0.28rem;">
      <li>Vector DB for semantic screen memory</li>
      <li>Top-5 similar past screens recalled</li>
      <li>Similarity threshold: 0.3</li>
      <li>Timestamped metadata per entry</li>
    </ul>
  </div>
</div>

---
layout: default
background: '#08080f'
---

<!-- SLIDE 6 — TWO MODES -->
<div class="slide-num">06 / 09</div>
<div class="rule"></div>
<h2 style="font-size:1.4rem;font-weight:700;color:#fff;margin:0 0 1rem;letter-spacing:-0.01em;">Two Modes</h2>

<div style="display:grid;grid-template-columns:1fr 1fr;gap:1.1rem;">
  <div>
    <div class="label" style="color:#6366f1;margin-bottom:0.5rem;">Orb Mode — default</div>
    <div class="glass" style="display:flex;flex-direction:column;align-items:center;gap:0.65rem;padding:1.25rem 1rem;">
      <div class="orb"></div>
      <p style="font-size:0.68rem;color:#475569;text-align:center;margin:0;line-height:1.5;">80x80px — follows cursor — always-on-top — no taskbar</p>
    </div>
    <div style="margin-top:0.65rem;display:flex;flex-direction:column;gap:0.28rem;font-size:0.7rem;color:#64748b;">
      <div style="display:flex;align-items:center;"><span class="dot" style="background:#6366f1;"></span>Indigo — idle</div>
      <div style="display:flex;align-items:center;"><span class="dot" style="background:#f97316;"></span>Orange — connecting</div>
      <div style="display:flex;align-items:center;"><span class="dot" style="background:#4ade80;"></span>Green — listening</div>
      <div style="display:flex;align-items:center;"><span class="dot" style="background:#22d3ee;"></span>Cyan — speaking</div>
      <div style="display:flex;align-items:center;"><span class="dot" style="background:#f87171;"></span>Red — error</div>
    </div>
  </div>
  <div>
    <div class="label" style="color:#6366f1;margin-bottom:0.5rem;">Expanded Panel — Ctrl+Shift+S</div>
    <div class="glass" style="display:flex;flex-direction:column;gap:0.55rem;padding:0.9rem;">
      <div style="display:flex;justify-content:center;align-items:flex-end;gap:3px;height:26px;">
        <div style="width:4px;height:14px;background:#6366f1;border-radius:2px;opacity:0.5;"></div>
        <div style="width:4px;height:22px;background:#6366f1;border-radius:2px;opacity:0.8;"></div>
        <div style="width:4px;height:10px;background:#6366f1;border-radius:2px;opacity:0.4;"></div>
        <div style="width:4px;height:24px;background:#818cf8;border-radius:2px;"></div>
        <div style="width:4px;height:16px;background:#6366f1;border-radius:2px;opacity:0.6;"></div>
        <div style="width:4px;height:20px;background:#818cf8;border-radius:2px;opacity:0.9;"></div>
        <div style="width:4px;height:12px;background:#6366f1;border-radius:2px;opacity:0.5;"></div>
        <div style="width:4px;height:18px;background:#6366f1;border-radius:2px;opacity:0.7;"></div>
      </div>
      <div style="background:rgba(255,255,255,0.04);border-radius:6px;padding:0.45rem 0.6rem;font-size:0.67rem;color:#94a3b8;font-family:'JetBrains Mono',monospace;line-height:1.65;">
        <span style="color:#818cf8;">You:</span> what's wrong with this code?<br/>
        <span style="color:#67e8f9;">AI:</span> Null check on line 42 is missing...
      </div>
      <div style="background:rgba(255,255,255,0.04);border-radius:6px;padding:0.35rem 0.6rem;font-size:0.63rem;color:#475569;font-family:'JetBrains Mono',monospace;">
        Screen: <span style="color:#94a3b8;">TS error — cannot read property of undefined</span>
      </div>
      <div>
        <span class="tag">auth.ts — 2m ago</span>
        <span class="tag">similar error</span>
      </div>
    </div>
    <p style="font-size:0.68rem;color:#475569;margin-top:0.55rem;line-height:1.5;">380x500px — centered — waveform, transcript, screen context, memories</p>
  </div>
</div>

---
layout: default
background: '#08080f'
---

<!-- SLIDE 7 — SHORTCUTS -->
<div class="slide-num">07 / 09</div>
<div class="rule"></div>
<h2 style="font-size:1.4rem;font-weight:700;color:#fff;margin:0 0 1.75rem;letter-spacing:-0.01em;">Keyboard Shortcuts</h2>

<div style="display:flex;flex-direction:column;gap:0.9rem;max-width:500px;margin:0 auto;">
  <div class="glass" style="display:flex;align-items:center;gap:1.25rem;">
    <div style="display:flex;gap:0.35rem;flex-shrink:0;">
      <kbd style="background:rgba(255,255,255,0.06);border:1px solid rgba(255,255,255,0.1);border-bottom:2px solid rgba(255,255,255,0.18);border-radius:5px;padding:0.28rem 0.55rem;color:#cbd5e1;font-family:'JetBrains Mono',monospace;font-size:0.72rem;">Ctrl</kbd>
      <kbd style="background:rgba(255,255,255,0.06);border:1px solid rgba(255,255,255,0.1);border-bottom:2px solid rgba(255,255,255,0.18);border-radius:5px;padding:0.28rem 0.55rem;color:#cbd5e1;font-family:'JetBrains Mono',monospace;font-size:0.72rem;">Shift</kbd>
      <kbd style="background:rgba(99,102,241,0.14);border:1px solid rgba(99,102,241,0.28);border-bottom:2px solid rgba(99,102,241,0.38);border-radius:5px;padding:0.28rem 0.55rem;color:#a5b4fc;font-family:'JetBrains Mono',monospace;font-size:0.72rem;font-weight:700;">A</kbd>
    </div>
    <div>
      <div style="font-size:0.85rem;font-weight:600;color:#e2e8f0;margin-bottom:0.18rem;">Start / Stop Voice Call</div>
      <div style="font-size:0.7rem;color:#64748b;">Instantly begin talking to the AI assistant</div>
    </div>
  </div>
  <div class="glass" style="display:flex;align-items:center;gap:1.25rem;">
    <div style="display:flex;gap:0.35rem;flex-shrink:0;">
      <kbd style="background:rgba(255,255,255,0.06);border:1px solid rgba(255,255,255,0.1);border-bottom:2px solid rgba(255,255,255,0.18);border-radius:5px;padding:0.28rem 0.55rem;color:#cbd5e1;font-family:'JetBrains Mono',monospace;font-size:0.72rem;">Ctrl</kbd>
      <kbd style="background:rgba(255,255,255,0.06);border:1px solid rgba(255,255,255,0.1);border-bottom:2px solid rgba(255,255,255,0.18);border-radius:5px;padding:0.28rem 0.55rem;color:#cbd5e1;font-family:'JetBrains Mono',monospace;font-size:0.72rem;">Shift</kbd>
      <kbd style="background:rgba(99,102,241,0.14);border:1px solid rgba(99,102,241,0.28);border-bottom:2px solid rgba(99,102,241,0.38);border-radius:5px;padding:0.28rem 0.55rem;color:#a5b4fc;font-family:'JetBrains Mono',monospace;font-size:0.72rem;font-weight:700;">S</kbd>
    </div>
    <div>
      <div style="font-size:0.85rem;font-weight:600;color:#e2e8f0;margin-bottom:0.18rem;">Toggle Expanded Panel</div>
      <div style="font-size:0.7rem;color:#64748b;">Show or hide the full panel during a call</div>
    </div>
  </div>
  <p style="font-size:0.68rem;color:#334155;text-align:center;margin-top:0.25rem;">Registered at system level — active even when Lensys is not focused.</p>
</div>

---
layout: default
background: '#08080f'
---

<!-- SLIDE 8 — DEMO -->
<div class="slide-num">08 / 09</div>
<div class="rule"></div>
<h2 style="font-size:1.4rem;font-weight:700;color:#fff;margin:0 0 1rem;letter-spacing:-0.01em;">Demo</h2>

<video controls style="width:100%;border-radius:10px;border:1px solid rgba(255,255,255,0.08);background:#0d0d1a;max-height:390px;display:block;">
  <source src="./demo.mp4" type="video/mp4" />
</video>

---
layout: default
background: '#08080f'
---

<!-- SLIDE 9 — WHAT'S NEXT -->
<div class="slide-num">09 / 09</div>
<div class="rule"></div>
<h2 style="font-size:1.4rem;font-weight:700;color:#fff;margin:0 0 1rem;letter-spacing:-0.01em;">What's Next</h2>

<div style="display:grid;grid-template-columns:1fr 1fr;gap:0.65rem;">
  <div class="glass">
    <div class="label" style="color:#6366f1;margin-bottom:0.35rem;">IDE Integration</div>
    <p style="font-size:0.72rem;color:#64748b;line-height:1.5;margin:0;">Deep VS Code / Cursor plugin — cursor position, open file, and selected text passed as context automatically.</p>
  </div>
  <div class="glass">
    <div class="label" style="color:#6366f1;margin-bottom:0.35rem;">Custom Assistant Personas</div>
    <p style="font-size:0.72rem;color:#64748b;line-height:1.5;margin:0;">Configure different AI personalities per project — strict code reviewer or brainstorming buddy.</p>
  </div>
  <div class="glass">
    <div class="label" style="color:#6366f1;margin-bottom:0.35rem;">Multi-monitor Support</div>
    <p style="font-size:0.72rem;color:#64748b;line-height:1.5;margin:0;">Capture specific windows or monitors. Let the user choose exactly what to share with the AI.</p>
  </div>
  <div class="glass">
    <div class="label" style="color:#6366f1;margin-bottom:0.35rem;">Local LLM Option</div>
    <p style="font-size:0.72rem;color:#64748b;line-height:1.5;margin:0;">Ollama integration for fully offline, privacy-first operation — no cloud API keys needed.</p>
  </div>
</div>

---
layout: cover
background: '#08080f'
---

<!-- SLIDE 10 — END -->
<div style="display:flex;flex-direction:column;align-items:center;justify-content:center;height:100%;gap:1.25rem;text-align:center;">
  <div class="orb" style="width:86px;height:86px;"></div>
  <div>
    <h1 style="font-size:2.8rem;font-weight:800;letter-spacing:-0.02em;color:#fff;margin:0 0 0.2rem;line-height:1;">Try Lensys</h1>
    <p style="font-size:0.78rem;color:#6366f1;letter-spacing:0.14em;text-transform:uppercase;font-weight:600;margin:0;">Press Ctrl+Shift+A and just talk.</p>
  </div>
  <div class="glass" style="text-align:left;font-family:'JetBrains Mono',monospace;font-size:0.72rem;line-height:2.1;margin-top:0.25rem;">
    <span style="color:#334155;"># get started</span><br/>
    <span style="color:#6366f1;">git clone</span> &lt;repo&gt; &amp;&amp; cd lensys<br/>
    cp .env.example .env<br/>
    pnpm install &amp;&amp; pnpm tauri dev
  </div>
</div>
