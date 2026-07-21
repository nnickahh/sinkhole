# SinkHole

SinkHole is a local-first desktop ad blocker built with Tauri v2, Rust, React, TypeScript, and Tailwind CSS. It compiles EasyList-style domain rules in memory, exposes a local HTTP/HTTPS `CONNECT` proxy at `127.0.0.1:8118`, and keeps protection preferences, custom list URLs, and allowlisted domains across restarts.

## What it does

- Loads EasyList on startup with a small built-in fallback for offline launches.
- Parses Adblock Plus domain anchors (`||example.com^`), domain exceptions, and hosts-format sources.
- Filters HTTP proxy requests and HTTPS `CONNECT` requests by hostname without decrypting TLS traffic.
- Returns `204 No Content` for blocked requests and increments live session statistics.
- Provides IPC commands for toggling, stats, settings, list updates, and direct URL checks.
- Minimizes to a system tray with protection, dashboard, and quit actions.

The proxy is intentionally local and does not silently change operating-system network settings. Configure `127.0.0.1:8118` as the HTTP and HTTPS proxy in the browser or operating system whose traffic should be filtered. HTTPS filtering is hostname-based at the `CONNECT` boundary; SinkHole does not install a root certificate or inspect encrypted content.

## Development

Prerequisites on Windows are Node.js 18+, pnpm, Rust stable, WebView2, and Visual Studio 2022 Build Tools with the C++ desktop workload.

```powershell
pnpm install
pnpm tauri dev
```

## Verification

```powershell
pnpm check
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml live_easylist_contains_more_than_thirty_thousand_supported_rules -- --ignored
pnpm tauri build
```

The live EasyList test is ignored by default because it requires network access. The release installer and executable bundles are written below `src-tauri/target/release/bundle/`.
