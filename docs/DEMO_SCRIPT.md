# SinkHole demo — 2:35 target

The Build Week announcement explicitly permits AI-assisted narration. A stock or properly licensed ElevenLabs voice is acceptable. Do not clone another person’s voice without permission.

Record the screen first, then generate and add the narration. Keep the final public YouTube upload below three minutes; aim for 2:30–2:40 so encoding or title cards cannot push it over.

## Shot list and narration

### 0:00–0:15 — The problem

**Show:** SinkHole title card, then the full dashboard.

**Narration:**

> This is SinkHole, a local Windows privacy proxy for ads, trackers, and telemetry. Most blockers show a green status without proving that browser traffic is actually reaching them. SinkHole makes that connection visible.

### 0:15–0:42 — Product and categories

**Show:** Cosmic dashboard, total counter, and the Ads, Trackers, and Telemetry breakdown.

**Narration:**

> The Rust proxy checks destination hostnames locally against three recommended sources: EasyList for advertising, EasyPrivacy for tracking, and a HaGeZi list for known Windows and Office telemetry endpoints. Each block keeps its category, so the dashboard explains what was stopped instead of showing one vague number.

### 0:42–1:12 — Prove the browser is connected

**Show:** Select Connect Windows, then open the connection test. Return to the dashboard and show Requests Seen increasing.

**Narration:**

> Connecting saves the current Windows proxy settings, routes browser traffic through SinkHole, and restores the previous settings when disconnected. This local test page only appears through the proxy. After a request is observed, the status changes from waiting to traffic captured, so enabled and actually working are separate states.

### 1:12–1:37 — Blocking and controls

**Show:** Use the three scanner presets, briefly open Settings, and show the recommended lists and allowed domains.

**Narration:**

> The built-in scanner previews whether a destination is classified as an ad, tracker, or telemetry host. Lists can be refreshed or extended, and allowed domains bypass filtering. HTTPS remains encrypted because SinkHole filters the hostname at the CONNECT boundary and does not install a root certificate.

### 1:37–1:57 — Tray quick control

**Show:** Hide the dashboard, left-click the SinkHole tray icon, toggle the compact switch, then open the dashboard from its gear button.

**Narration:**

> For everyday use, the tray opens this compact WARP-style control. Its single switch controls both browser routing and filtering, while the session summary shows whether traffic has actually been seen.

### 1:57–2:24 — Codex and GPT-5.6

**Show:** A quick code montage: `engine.rs`, `system_proxy.rs`, `QuickPanel.tsx`, tests, and the README.

**Narration:**

> We built SinkHole with Codex using GPT-5.6. Codex inspected the original Tauri architecture, implemented categorized rule matching and TCP proxy tests, designed reversible Windows proxy handling, caught a missing Tauri permission on the tray window, created the cosmic interface, and prepared documentation and CI. GPT-5.6 helped reason across Rust, React, Windows networking, and product truthfulness as one system.

### 2:24–2:35 — Honest close

**Show:** Final dashboard and new gravity-well icon.

**Narration:**

> SinkHole is deliberately honest about its limits: it blocks known hosts, not cosmetic page elements or local data collection. Next, we would add a browser-extension companion. For now, privacy traffic goes into the event horizon.

## Editing checklist

- Use 1080p or 1440p screen capture with the cursor visible.
- Keep narration audible throughout; background music should be quiet and optional.
- Remove loading pauses and failed takes.
- Show the app actually changing state—do not use only static screenshots.
- Export under 2:50 and upload as **Public** on YouTube, not Unlisted.
- Include “Built with Codex and GPT-5.6” in the YouTube description.
