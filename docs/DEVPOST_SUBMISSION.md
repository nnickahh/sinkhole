# SinkHole — Devpost submission copy

## Core fields

- **Project name:** SinkHole
- **Tagline:** A local privacy event horizon for ads, trackers, and Windows telemetry.
- **Track:** Apps for Your Life
- **Repository:** https://github.com/Skithrills/SinkHole
- **Built with:** Codex, GPT-5.6, Tauri, Rust, React, TypeScript, Tailwind CSS, EasyList, EasyPrivacy, HaGeZi, Windows WinINet

## Description

SinkHole is a cosmic-themed Windows privacy proxy that makes an invisible part of ad blocking visible: whether browser traffic is actually connected to the blocker.

The app runs a local Rust HTTP/HTTPS CONNECT proxy at `127.0.0.1:8118`. With one switch, it connects the current Windows user’s browser traffic, checks destination hostnames against locally compiled privacy lists, and sinks known advertising, tracking, and Windows telemetry endpoints before a connection is made. It never installs a root certificate or decrypts HTTPS content.

SinkHole separates three ideas that privacy tools often blur together:

1. The filter engine is enabled.
2. Windows browser traffic is routed through the proxy.
3. The proxy has actually observed traffic.

The full cosmic dashboard shows those states independently, reports ads, trackers, and telemetry in separate counters, manages privacy lists and allowed domains, and includes a local `sinkhole.test` proof page. A compact WARP-style tray popover exposes the same truthful on/off state without opening the dashboard.

The default sources are EasyList for advertising, EasyPrivacy for tracking, and HaGeZi’s native Windows/Office list for known telemetry endpoints. Custom EasyList-style and hosts-format sources can be added in settings.

SinkHole preserves the previous Windows proxy configuration before connecting and restores it on disconnect or normal tray quit. All filtering and statistics stay on the device.

## How Codex and GPT-5.6 were used

Codex and GPT-5.6 were used as an implementation partner across the full project lifecycle: inspecting the existing Tauri/Rust/React architecture, extending the rule engine, designing safe Windows proxy restoration, adding TCP-level tests, creating the tray quick-control workflow, rebranding the interface and icon, checking Tauri capability boundaries, documenting limitations, and preparing CI and submission material.

The final implementation deliberately reports unsupported behavior honestly. SinkHole performs hostname-level filtering and cannot remove cosmetic gaps from a page, inspect encrypted URL paths, or prevent software from collecting data locally.

## Challenges

- Safely changing a system proxy without losing a user’s previous configuration.
- Making “filter engine on” visually distinct from “browser connected.”
- Categorizing overlapping domain rules while preserving allowlist priority.
- Keeping HTTPS private while still blocking known destinations at the CONNECT hostname boundary.
- Designing a tray-sized interface that remains understandable at a glance.

## Accomplishments

- Categorized ad, tracker, telemetry, and custom-rule counters.
- More than 100,000 supported domain rules across the recommended live lists.
- One-click Windows connection with persisted backup and restoration.
- A local browser connection proof page.
- Full dashboard plus compact tray popover.
- Deterministic proxy and rule-engine tests and Windows CI configuration.

## What we learned

A privacy dashboard should not claim protection merely because a background process is running. The useful state is a chain: proxy listening, browser routed, request observed, destination classified, and block recorded. Modeling each step separately made both the implementation and the user interface more trustworthy.

## What’s next

- A companion browser extension for cosmetic filtering and first-party page controls.
- Per-app routing instead of relying only on the Windows proxy setting.
- Signed installers and crash-safe proxy recovery through a small watchdog.
- Historical charts with no cloud analytics.

## Final submission checklist

- [ ] Rename the shared Devpost project from **Untitled** to **SinkHole**.
- [ ] Add the tagline, description, and built-with values above.
- [ ] Select **Apps for Your Life**.
- [ ] Add `https://github.com/Skithrills/SinkHole` after the changes are pushed.
- [ ] Upload a public YouTube demo shorter than three minutes.
- [ ] Show the tray switch, `sinkhole.test`, counters, settings, and disconnect restoration in the demo.
- [ ] Upload a project thumbnail using the new gravity-well icon.
- [ ] Add the Codex session ID produced by `/feedback`.
- [ ] Submit the project to OpenAI Build Week; a pre-draft is not a submission.

The timed narration and screen-recording shot list are in [`DEMO_SCRIPT.md`](DEMO_SCRIPT.md).
